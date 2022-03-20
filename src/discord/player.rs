use std::{
    env,
    sync::Arc,
    collections::HashSet,
};
use songbird::SerenityInit;

use crate::music;
use crate::music::*;
use crate::get_mstate;

use crate::discord::commands::{
    general::*,
    musicctl::*,
    queuectl::*,
    autoplay::*,
    debug::*,
    helpers::*,
};

use log::*;

use serenity::{
    async_trait,
    client::ClientBuilder,
    model::{
        channel::Message,
        gateway::Ready,
        id::UserId,
        id::GuildId,
        voice::VoiceState,
    },
    prelude::*,
//    client::bridge::gateway::{GatewayIntents, ShardId, ShardManager},
    framework::standard::{
        help_commands,
        macros::{group, help, hook},
        Args,
        CommandGroup,
        CommandError,
//        CommandOptions,
        CommandResult,
        DispatchError,
        HelpOptions,
        StandardFramework,
    },
};

use songbird::{
    Event,
    EventContext,
    EventHandler as VoiceEventHandler,
};

// TODO: this should get used somewhere:
//FFMPEG_OPTS = '-af loudnorm=I=-16:TP=-1.5:LRA=11'



struct Handler;


#[group]
#[commands(ping, join, leave)]
struct General;

#[group]
#[description = "Commands for controlling the music player"]
#[commands(play, nowplaying, next, stop, start, display, history, previous)]
struct MusicControlCmd;

#[group]
#[description = "Commands to manage the music queue"]
#[commands(queue, enqueue, clearqueue, queuestatus)]
struct QueueControlCmd;

#[group]
#[description = "Commands to manage autoplay state"]
#[prefixes("autoplay", "ap")]
#[commands(toggle, setlist, upcoming, enrolluser, removeuser, rebalance, shuffle, dump, advance)]
struct AutoplayCmd;

#[group]
#[description = "Commands for debugging purposes"]
#[prefix("debug")]
#[commands(usertime, dropapuser, modutime, musicstate)]
// TODO: require owner
struct DebugCmd;

#[hook]
async fn dispatch_error(ctx: &Context, msg: &Message, error: DispatchError) {
    match error {
        DispatchError::CheckFailed(s, reason) =>
            msg.channel_id.say(&ctx.http, format!("Command failed: {:?} {:?}", s, reason)).await.unwrap(),
        err => msg.channel_id.say(&ctx.http, format!("Error executing command: {:?}", err)).await.unwrap(),
    };
}

#[hook]
async fn stickymessage_hook(ctx: &Context, _msg: &Message, _cmd_name: &str, _error: Result<(), CommandError>) {
    get_mstate!(mut, mstate, ctx);

    if let Some(m) = &mstate.sticky {
        m.channel_id.delete_message(&ctx.http, m).await.unwrap();

        let new = m.channel_id.send_message(&ctx.http, |m| {
            m.add_embeds(vec![get_queuestate_embed(&mstate), get_nowplay_embed(&mstate)])
        }).await.unwrap();

        mstate.sticky = Some(new);
    }
}

#[async_trait]
impl EventHandler for Handler {
    // TODO: probably not need this
    async fn message(&self, ctx: Context, msg: Message) {
        // Ignore self
        if msg.author.id == ctx.cache.current_user().await.id {
            return
        }
        // TODO: use an actual logging system
        trace!("{}", msg.content);
    }

    // Set a handler to be called on the `ready` event. This is called when a
    // shard is booted, and a READY payload is sent by Discord. This payload
    // contains data like the current user's guild Ids, current user data,
    // private channels, and more.
    //
    // In this case, just print what the current user's username is.
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);

        info!("operating as {:?}", ctx.cache.current_user().await);

    }

    async fn voice_state_update(&self, ctx: Context, guildid: Option<GuildId>, old: Option<VoiceState>, new: VoiceState) {
        // TODO: maybe factor out common useful values like, botid, guild, etc

        // Common cases to ignore this voice state change
        if let Some(o) = &old {
            if o.self_mute ^ new.self_mute {
                debug!("ignoring self-mute voice state change");
                return;
            }
            if o.self_deaf ^ new.self_deaf {
                debug!("ignoring self-deafen voice state change");
                return;
            }
            if o.mute ^ new.mute {
                debug!("ignoring mute voice state change");
                return;
            }
            if o.deaf ^ new.deaf {
                debug!("ignoring deafen voice state change");
                return;
            }
        }

        last_one_in_checker(&ctx, &guildid, &old, &new).await;
        music::autoplay::autoplay_voice_state_update(ctx, guildid, old, new).await;
    }
}

// TODO: perhaps move this elsewhere
async fn last_one_in_checker(ctx: &Context, guildid: &Option<GuildId>, old: &Option<VoiceState>, new: &VoiceState) {
    let bot = ctx.cache.current_user_id().await;
    let guild = ctx.cache.guild(guildid.unwrap()).await.unwrap(); // TODO: don't unwrap here, play nice
    let bot_voice = guild.voice_states.get(&bot);

    if bot_voice.is_none() {
        // Don't bother if bot isn't in voice
        return;
    }
    let bot_voice = bot_voice.unwrap();
    let bot_chan = bot_voice.channel_id.unwrap();

    if let Some(n) = new.channel_id {
        if n == bot_chan {
            debug!("connect detected to bot's channel");
            // TODO: disable disconnect timer if enabled
        }

        return;
    }

    // old == None implies join, already handled the join case we care about
    if old.is_none() {
        return;
    }
    let old = old.as_ref().unwrap();

    // Bail if for some reason there's no channel_id in the old
    if old.channel_id.is_none() {
        return;
    }
    let old_chan = old.channel_id.unwrap();

    // Someone left the bot's channel...
    if old_chan == bot_chan {
        debug!("disconnect detected from bot's channel");

        // Count how many users are still connected, ignoring the bot itself
        let cnt = guild.voice_states.iter()
            .filter(|(_, vs)| vs.channel_id.unwrap() == bot_chan)
            .filter(|(u, _)| **u != bot)
            .count();

        // No users remaining -> start the timer
        if cnt == 0 {
            info!("channel appears empty, disconnecting...");

            get_mstate!(mut, mstate, ctx);
            mstate.leave().await;
        }
    }
}


#[help]
#[command_not_found_text = "Could not find: {}"]
#[max_levenshtein_distance(3)]
async fn helpme(
    context: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    let _ = help_commands::with_embeds(context, msg, args, help_options, groups, owners).await;
    Ok(())
}


/* Possible mess for queue support */


pub struct TrackEndNotifier {
    pub ctx: Context,
}

#[async_trait]
impl VoiceEventHandler for TrackEndNotifier {

    // TODO: somehow make this a signaling thing so we don't have to await here
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        debug!("TrackEndNotifier fired");
        let mstate = mstate_get(&self.ctx).await.unwrap();
        let mut mstate = mstate.lock().await;

        if let Some((_, song)) = &mstate.current_track.take() {
            mstate.history.push_front(song.clone());
            mstate.history.truncate(10); // TODO: config max history buffer length
        }
        else {
            debug!("TrackEnd handler somehow called with mstate.current_track = None");
        }

        match mstate.status {
            MusicStateStatus::Stopping => {
                debug!("stopping music play via event handler");
                return None; // We're done here
            }
            _ => {}
        };

        let ret = mstate.next().await;
        if let Ok(_) = ret {
            debug!("TrackEnd handler mstate.next() = {:?}", ret);
        }
        else if let Err(e) = ret {
            error!("{:?}", e);
        }

        if let Some(sticky) = &mstate.sticky {
            sticky.channel_id.edit_message(&self.ctx.http, sticky, |m| {
                m.set_embeds(vec![get_queuestate_embed(&mstate), get_nowplay_embed(&mstate)])
            }).await.unwrap();
        }

        None
    }
}

pub mod discord_mstate {
    pub use super::TrackEndNotifier as TrackEndNotifier;
}


/* Enter mess to make the singleton magic via serenity here */
pub struct MusicStateKey;

impl TypeMapKey for MusicStateKey {
    type Value = Arc<Mutex<MusicState>>;
}

pub trait MusicStateInit {
    fn register_musicstate(self) -> Self;
}

fn register(client_builder: ClientBuilder) -> ClientBuilder {
    let tmp = Arc::new(Mutex::new(MusicState::new()));
    client_builder
        .type_map_insert::<MusicStateKey>(tmp.clone())
}

impl MusicStateInit for ClientBuilder<'_> {
    fn register_musicstate(self) -> Self {
        register(self)
    }
}




pub async fn create_player() -> serenity::Client {
    let token = env::var("DISCORD_TOKEN").expect("Must provide env var DISCORD_TOKEN");

    let framework = StandardFramework::new()
        .configure(|c| c
            .with_whitespace(true)
            //.on_mention(Some(bot_id)) // TODO: not sure
            .prefix("!")
            .delimiters(vec![", ", ",", " "])
            //.owners(owners) // TODO: set owners so adminy commands work
            )
        .after(stickymessage_hook)
        .on_dispatch_error(dispatch_error)
        .group(&GENERAL_GROUP)
        .group(&MUSICCONTROLCMD_GROUP)
        .group(&QUEUECONTROLCMD_GROUP)
        .group(&AUTOPLAYCMD_GROUP)
        .group(&DEBUGCMD_GROUP)
        .help(&HELPME);


    // Create a new instance of the Client, logging in as a bot. This will
    // automatically prepend your bot token with "Bot ", which is a requirement
    // by Discord for bot users.
    let client =
        Client::builder(&token)
            .event_handler(Handler)
            .framework(framework)
            .register_songbird()
            .register_musicstate()
            .await.expect("Err creating client");

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform
    // exponential backoff until it reconnects.
    return client;
}
