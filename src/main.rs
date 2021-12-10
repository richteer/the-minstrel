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
    debug::*,
};


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
#[commands(play, nowplaying, next, stop, start)]
struct MusicControlCmd;

#[group]
#[description = "Commands to manage the music queue"]
#[commands(queue, enqueue, clearqueue, setlist, autoplay, upcoming)]
struct QueueControlCmd;

#[group]
#[description = "Commands for debugging purposes"]
#[prefix("debug")]
#[commands(usertime)]
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

#[async_trait]
impl EventHandler for Handler {
    // TODO: probably not need this
    async fn message(&self, _ctx: Context, msg: Message) {
        // TODO: use an actual logging system
        println!("{}", msg.content);
    }

    // Set a handler to be called on the `ready` event. This is called when a
    // shard is booted, and a READY payload is sent by Discord. This payload
    // contains data like the current user's guild Ids, current user data,
    // private channels, and more.
    //
    // In this case, just print what the current user's username is.
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        println!("operating as {:?}", ctx.cache.current_user().await);

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

    let framework = StandardFramework::new()
        .configure(|c| c
            .with_whitespace(true)
            //.on_mention(Some(bot_id)) // TODO: not sure
            .prefix("!")
            .delimiters(vec![", ", ","])
            //.owners(owners) // TODO: set owners so adminy commands work
            )
        .on_dispatch_error(dispatch_error)
        .group(&GENERAL_GROUP)
        .group(&MUSICCONTROLCMD_GROUP)
        .group(&QUEUECONTROLCMD_GROUP)
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
        println!("Client error: {:?}", why);
    }
}
