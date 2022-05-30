use std::{
    env,
    sync::Arc,
};
use songbird::SerenityInit;

use music::musiccontroller::MusicAdapter;
use crate::player::*;
use crate::helpers::*;
use crate::{
    get_mstate,
    get_dstate,
};
use crate::state::DiscordState;


use log::*;

use serenity::{
    async_trait,
    client::ClientBuilder,
    model::{
        channel::Message,
        gateway::Ready,
        id::GuildId,
        voice::VoiceState,
    },
    prelude::*,
    framework::standard::{
        macros::{hook},

        CommandError,
        DispatchError,
    },
};

// TODO: this should get used somewhere:
//FFMPEG_OPTS = '-af loudnorm=I=-16:TP=-1.5:LRA=11'

struct Handler;


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
    get_dstate!(mut, dstate, ctx);

    if let Some(m) = &dstate.sticky {
        m.channel_id.delete_message(&ctx.http, m).await.unwrap();

        let mdata = mstate.get_webdata().await;
        let ap_enabled = mstate.autoplay.is_enabled().await;

        let qs_embed = get_queuestate_embed(&mdata, ap_enabled);
        let np_embed = get_nowplay_embed(ctx, &mdata).await;

        let new = m.channel_id.send_message(&ctx.http, |m| {
            m.add_embeds(vec![qs_embed, np_embed])
        }).await.unwrap();

        dstate.sticky = Some(new);
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
        autoplay_voice_state_update(ctx, guildid, old, new).await;
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

            get_dstate!(mut, dstate, ctx);
            dstate.leave().await;
        }
    }
}


/* Enter mess to make the singleton magic via serenity here */
pub struct MusicStateKey;

impl TypeMapKey for MusicStateKey {
    type Value = MusicAdapter;
}

pub trait MusicStateInit {
    fn register_musicstate(self, mstate: MusicAdapter) -> Self;
}

impl MusicStateInit for ClientBuilder<'_> {
    fn register_musicstate(self, mstate: MusicAdapter) -> Self {
        self.type_map_insert::<MusicStateKey>(mstate)
    }
}

pub struct DiscordPlayerKey;

impl TypeMapKey for DiscordPlayerKey {
    type Value = Arc<Mutex<DiscordPlayer>>;
}

pub trait DiscordPlayerInit {
    fn register_player(self, dplayer: Arc<Mutex<DiscordPlayer>>) -> Self;
}

impl DiscordPlayerInit for ClientBuilder<'_> {
    fn register_player(self, dplayer: Arc<Mutex<DiscordPlayer>>) -> Self {
        self.type_map_insert::<DiscordPlayerKey>(dplayer)
    }
}

pub struct DiscordStateKey;

impl TypeMapKey for DiscordStateKey {
    type Value = Arc<Mutex<DiscordState>>;
}

pub trait DiscordStateInit {
    fn register_dstate(self, dstate: Arc<Mutex<DiscordState>>) -> Self;
}

impl DiscordStateInit for ClientBuilder<'_> {
    fn register_dstate(self, dstate: Arc<Mutex<DiscordState>>) -> Self {
        self.type_map_insert::<DiscordStateKey>(dstate)
    }
}


pub async fn create_player(mstate: MusicAdapter, dplayer: Arc<Mutex<DiscordPlayer>>) -> serenity::Client {
    let token = env::var("DISCORD_TOKEN").expect("Must provide env var DISCORD_TOKEN");
    let framework = crate::frontend::framework::init_framework();

    let dstate = DiscordState::new(mstate.clone(), dplayer.clone());
    let dstate = Arc::new(Mutex::new(dstate));

    // Create a new instance of the Client, logging in as a bot. This will
    // automatically prepend your bot token with "Bot ", which is a requirement
    // by Discord for bot users.
    let client =
        Client::builder(&token)
            .event_handler(Handler)
            .framework(framework)
            .register_songbird()
            // TODO: really consider unifying these maybe. DiscordState holds references to both
            //  DiscordPlayer and MusicAdapter, maybe only dstate should be used everywhere.
            .register_musicstate(mstate)
            .register_player(dplayer)
            .register_dstate(dstate)
            .await.expect("Err creating client");

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform
    // exponential backoff until it reconnects.
    client
}
