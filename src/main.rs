use std::{
    env,
    collections::HashSet,
};
use dotenv;
use songbird::SerenityInit;


mod music;
use music::{MusicStateInit, Song};

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
//        buckets::{LimitedFor, RevertBucket},
        help_commands,
        macros::{check, command, group, help, hook},
        Args,
        CommandGroup,
//        CommandOptions,
        CommandResult,
        DispatchError,
        HelpOptions,
        Reason,
        StandardFramework,
    },
    Result as SerenityResult
};

// TODO: this should get used somewhere:
//FFMPEG_OPTS = '-af loudnorm=I=-16:TP=-1.5:LRA=11'

/// Checks that a message successfully sent; if not, then logs why to stdout.
fn check_msg(result: SerenityResult<Message>) {
    if let Err(why) = result {
        println!("Error sending message: {:?}", why);
    }
}


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
#[commands(queue, enqueue, clearqueue, setlist, autoplay)]
struct QueueControlCmd;


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

#[check]
#[name = "voice_ready"]
// TODO: uhhhh yeah so this gets called by help, so i guess i'm really going to have to factor out this
// TODO: can this be moved into mstate::get(), so it can just be automagic?
async fn voice_ready(ctx: &Context, msg: &Message) -> Result<(), Reason> {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;
    let bot_id = ctx.cache.current_user_id().await;

    let caller_channel_id = guild
        .voice_states.get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id);

    let bot_channel_id = guild
        .voice_states.get(&bot_id)
        .and_then(|voice_state| voice_state.channel_id);

    // Get caller's voice channel, bail if they aren't in one
    let connect_to = match caller_channel_id {
        Some(channel) => channel,
        None => {
            return Err(Reason::User(String::from("You must be in a voice channel to use this command")));
        }
    };

    if let Some(bot_channel) = bot_channel_id {
        if bot_channel == connect_to {
            return Ok(())
        }
        else {
            return Err(Reason::User(String::from("Bot is in another voice channel")));
        }
    }


    let mstate = music::get(&ctx).await.unwrap().clone();
    let mut mstate = mstate.lock().await;
    mstate.init(&ctx, guild_id, connect_to).await;

    Ok(())
}

// TODO: These can definitely be cleaner, but might as well macro out now to make
//  life slightly easier if I do end up needing to replace them
macro_rules! get_mstate {
    ($mstate:ident, $ctx:ident) => {
        let $mstate = music::get(&$ctx).await.unwrap();
        let $mstate = $mstate.lock().await;
    };

    ($mut:ident, $mstate:ident, $ctx:ident) => {
        let $mstate = music::get(&$ctx).await.unwrap();
        let $mut $mstate = $mstate.lock().await;
    };
}


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

/****** HERE THERE BE COMMANDS ******/

#[command]
#[only_in(guilds)]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, "Pong! :)").await?;

    Ok(())
}

#[command]
#[only_in(guilds)]
#[checks(voice_ready)]
async fn join() -> CommandResult {
    Ok(())
}

#[command]
#[only_in(guilds)]
#[checks(voice_ready)] // TODO: make a in_voice or is_playing check
async fn _stop2(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    if let Some(manager) = songbird::get(ctx).await {
        if let Some(handler) = manager.get(guild_id) {
            let mut handler = handler.lock().await;

            handler.stop();
        }
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    if let Some(manager) = songbird::get(ctx).await {
        if let Some(handler) = manager.get(guild_id) {
            let mut handler = handler.lock().await;

            handler.stop();

            match handler.leave().await {
                Ok(()) => println!("left channel"),
                Err(e) => println!("failed to disconnect: {}", e),
            };
        }
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
#[checks(voice_ready)]
async fn play(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    // TODO: confirm if this is actually needed
    let url = args.single::<String>()?;

    let url = match Song::new(url, &msg.author) {
        Ok(u) => u,
        Err(_) => {
            check_msg(msg.channel_id.say(&ctx.http, "Must provide a URL to a video or audio").await);
            return Ok(())
        }
    };

    let mstate = music::get(&ctx).await;
    let ret = mstate.unwrap().lock().await.enqueue_and_play(url).await;

    // TODO: maybe factor this out into a generic reply handler?
    match ret {
        Ok(m) => check_msg(msg.channel_id.say(&ctx.http, m).await),
        Err(e) => check_msg(msg.channel_id.say(&ctx.http, format!("Error playing song: {:?}", e)).await),
    }

    Ok(())
}

#[command]
#[aliases(q, showqueue)]
#[only_in(guilds)]
#[checks(voice_ready)] // TODO: implement "in same voice channel" and use here, don't need to join
async fn queue(ctx: &Context, msg: &Message) -> CommandResult {
    let mstate = music::get(&ctx).await.unwrap();
    let mstate = mstate.lock().await;

    check_msg(msg.channel_id.say(&ctx.http, mstate.show_queue().await).await);

    Ok(())
}

#[command]
#[aliases(append)]
#[only_in(guilds)]
#[checks(voice_ready)] // TODO: implement "in same voice channel" and use here, don't need to join
async fn enqueue(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let url = args.single::<String>()?;

    let url = match Song::new(url, &msg.author) {
        Ok(u) => u,
        Err(_) => { // TODO: actually handle errors, probably make a generic surrender replier
            check_msg(msg.channel_id.say(&ctx.http, "Must provide a URL to a video or audio").await);
            return Ok(())
        }
    };

    let mstate = music::get(&ctx).await.unwrap();
    let mut mstate = mstate.lock().await;

    let ret = mstate.enqueue(url).await;

    // TODO: maybe factor this out into a generic reply handler?
    match ret {
        Ok(m) => check_msg(msg.channel_id.say(&ctx.http, m).await),
        Err(e) => check_msg(msg.channel_id.say(&ctx.http, format!("Error playing song: {:?}", e)).await),
    }

    Ok(())
}

#[command]
#[aliases(np)]
#[only_in(guilds)]
#[checks(voice_ready)] // TODO: implement "in same voice channel" and use here, don't need to join
async fn nowplaying(ctx: &Context, msg: &Message) -> CommandResult {
    get_mstate!(mstate, ctx);

    if let Some(song) = mstate.current_song() {
        // TODO: consider making this a helper, so the sticky nowplaying can use this

        let md = song.metadata;
        let thumb = match md.thumbnail.clone() {
            Some(t) => t,
            None => {
                // TODO: attempt to fetch thumb?
                String::from("")
            }
        };

        let nick = match song.requested_by.nick_in(&ctx.http, msg.guild_id.unwrap()).await {
            Some(n) => n,
            None => song.requested_by.name.clone()
        };

        check_msg(msg.channel_id.send_message(&ctx.http, |m| {
            m.embed(|e| { e
                .title(md.title)
                .thumbnail(thumb)
                .url(song.url)
                .description(md.uploader.unwrap_or(String::from("Unknown")))
                .footer(|f| { f
                    .icon_url(song.requested_by.face())
                    .text(format!("Requested by: {}", nick))
                })
            });

            m
        }).await);
    }
    else {
        check_msg(msg.channel_id.say(&ctx.http, "Nothing currently playing!").await);
    }


    Ok(())
}

#[command]
#[only_in(guilds)]
#[checks(voice_ready)] // TODO: implement "in same voice channel" and use here, don't need to join
async fn setlist(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let url = args.single::<String>()?;
    let mstate = music::get(&ctx).await.unwrap();
    let mut mstate = mstate.lock().await;

    /*
    println!("getting playlist");
    let data = youtube_dl::YoutubeDl::new(&url)
        .flat_playlist(true)
        .run();

    let data = match data {
        Ok(YoutubeDlOutput::SingleVideo(_)) => {
            check_msg(msg.channel_id.say(&ctx.http, "Must provide link to a playlist, not a single video").await);
            return Ok(()); // Not ok
        }
        Err(e) => {
            check_msg(msg.channel_id.say(&ctx.http, format!("Error fetching playlist: {:?}", e)).await);
            return Ok(()); // Not ok
        }
        Ok(YoutubeDlOutput::Playlist(p)) => p,
    };

    println!("done fetching playlist");


    for (i,e) in data.entries.unwrap().iter().enumerate() {
        if i == 9 {
            mstate.enqueue_and_play(Song::_from_video(e.clone(), &msg.author)).await.ok();
            break;
        }
        mstate.enqueue(Song::_from_video(e.clone(), &msg.author)).await.ok();
    }

    println!("done being weird");
    */
    mstate.autoplay.register(&msg.author, &url).ok();
    // TODO: send some feedback here

    Ok(())
}

#[command]
#[aliases(skip, n)]
#[only_in(guilds)]
#[checks(voice_ready)] // TODO: implement "in same voice channel" and use here, don't need to join
// TODO: require permissions to do this
async fn next(ctx: &Context, msg: &Message) -> CommandResult {
    get_mstate!(mut, mstate, ctx);

    let ret = mstate.skip().await;

    if let Ok(s) = ret {
        check_msg(msg.channel_id.say(&ctx.http, s).await);
    }
    else if let Err(e) = ret {
        check_msg(msg.channel_id.say(&ctx.http, format!("Error playing next: {:?}", e)).await);
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
#[checks(voice_ready)] // TODO: implement "in same voice channel" and use here, don't need to join
// TODO: require permissions to do this
async fn stop(ctx: &Context, msg: &Message) -> CommandResult {
    get_mstate!(mut, mstate, ctx);

    let ret = mstate.stop().await;

    if let Ok(s) = ret {
        check_msg(msg.channel_id.say(&ctx.http, s).await);
    }
    else if let Err(e) = ret {
        check_msg(msg.channel_id.say(&ctx.http, format!("Error stopping song: {:?}", e)).await);
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
#[checks(voice_ready)] // TODO: implement "in same voice channel" and use here, don't need to join
async fn start(ctx: &Context, msg: &Message) -> CommandResult {
    get_mstate!(mut, mstate, ctx);

    let ret = mstate.start().await;

    if let Ok(s) = ret {
        check_msg(msg.channel_id.say(&ctx.http, s).await);
    }
    else if let Err(e) = ret {
        check_msg(msg.channel_id.say(&ctx.http, format!("Error stopping song: {:?}", e)).await);
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
#[checks(voice_ready)] // TODO: implement "in same voice channel" and use here, don't need to join
// TODO: permissions
async fn clearqueue(ctx: &Context, msg: &Message) -> CommandResult {
    get_mstate!(mut, mstate, ctx);

    let ret = mstate.clear_queue();

    if let Ok(s) = ret {
        check_msg(msg.channel_id.say(&ctx.http, s).await);
    }
    else if let Err(e) = ret {
        check_msg(msg.channel_id.say(&ctx.http, format!("Error stopping song: {:?}", e)).await);
    }

    Ok(())
}


#[command]
#[aliases(ap)]
#[only_in(guilds)]
#[checks(voice_ready)] // TODO: implement "in same voice channel" and use here, don't need to join
async fn autoplay(ctx: &Context, msg: &Message) -> CommandResult {
    get_mstate!(mut, mstate, ctx);

    mstate.autoplay.enabled = !mstate.autoplay.enabled;

    // No need to do anything here if autoplay is disabled, it will probably stop itself
    if mstate.autoplay.enabled == false {
        return Ok(())
    }

    let ret = mstate.start().await;

    if let Ok(_) = ret {
        check_msg(msg.channel_id.say(&ctx.http, "Started autoplay!").await);
    }
    else if let Err(e) = ret {
        check_msg(msg.channel_id.say(&ctx.http, format!("Error starting autoplay: {:?}", e)).await);
    }

    Ok(())
}