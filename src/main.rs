use std::{
    env,
    collections::HashSet,
};
use dotenv;
use songbird::SerenityInit;


mod music;
use music::MusicStateInit;

mod commands;
use commands::{
    general::*,
    musicctl::*,
    queuectl::*,
    autoplay::*,
    debug::*,
};

use log::*;

use serenity::{
    async_trait,
    model::{
        channel::Message,
        gateway::Ready,
        id::UserId,
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

// TODO: this should get used somewhere:
//FFMPEG_OPTS = '-af loudnorm=I=-16:TP=-1.5:LRA=11'



struct Handler;


#[group]
#[commands(ping, join, leave)]
struct General;

#[group]
#[description = "Commands for controlling the music player"]
#[commands(play, nowplaying, next, stop, start, display, history)]
struct MusicControlCmd;

#[group]
#[description = "Commands to manage the music queue"]
#[commands(queue, enqueue, clearqueue, queuestatus)]
struct QueueControlCmd;

#[group]
#[description = "Commands to manage autoplay state"]
#[prefixes("autoplay", "ap")]
#[commands(toggle, setlist, upcoming, enrolluser, removeuser, rebalance, shuffle)]
struct AutoplayCmd;

#[group]
#[description = "Commands for debugging purposes"]
#[prefix("debug")]
#[commands(usertime, dropapuser, modutime)]
// TODO: require owner
struct DebugCmd;


#[hook]
async fn dispatch_error(ctx: &Context, msg: &Message, error: DispatchError) {
    match error {
        DispatchError::CheckFailed(s, reason) =>
            msg.channel_id.say(&ctx.http, format!("Command failed: {:?} {:?}", s, reason)).await.unwrap(),

            _ => msg.channel_id.say(&ctx.http, "Unknown error").await.unwrap(),
    };
}

#[hook]
async fn stickymessage_hook(ctx: &Context, _msg: &Message, _cmd_name: &str, _error: Result<(), CommandError>) {
    get_mstate!(mut, mstate, ctx);

    if let Some(m) = &mstate.sticky {
        m.channel_id.delete_message(&ctx.http, m).await.unwrap();

        let new = m.channel_id.send_message(&ctx.http, |m| {
            m.add_embeds(vec![mstate.get_queuestate_embed(), mstate.get_nowplay_embed()])
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


#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let token = env::var("DISCORD_TOKEN").expect("Must provide env var DISCORD_TOKEN");

    env_logger::init();

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
    let mut client =
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
    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }
}
