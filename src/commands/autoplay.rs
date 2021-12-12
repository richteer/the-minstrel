use serenity::{
    model::{
        channel::Message,
    },
    prelude::*,
    framework::standard::{
        Args,
        macros::command,
        CommandResult,
    },
};

use crate::{get_mstate, join_voice};
use super::check_msg;
use super::music;
use super::music::Requester;
use super::music::MusicError;

use super::helpers::*;


#[command]
#[aliases(t)]
#[only_in(guilds)]
async fn toggle(ctx: &Context, msg: &Message) -> CommandResult {
    get_mstate!(mut, mstate, ctx);

    mstate.autoplay.enabled = !mstate.autoplay.enabled;

    // No need to do anything here if autoplay is disabled, it will probably stop itself
    if mstate.autoplay.enabled == false {
        check_msg(msg.channel_id.say(&ctx.http, "Disabling autoplay.").await);
        return Ok(())
    }

    check_msg(msg.channel_id.say(&ctx.http, "Enabling autoplay.").await);

    drop(mstate); // Close mstate here, since we're going to need to relock in join_voice()
    join_voice!(ctx, msg);
    get_mstate!(mut, mstate, ctx);

    let ret = mstate.start().await;

    match ret {
        Err(MusicError::AlreadyPlaying) => (), // Suppress AlreadyPlaying, doesn't matter here
        Err(e) => check_msg(msg.channel_id.say(&ctx.http, format!("Error starting autoplay: {:?}", e)).await),
        _ => (),
    };

    Ok(())
}


#[command]
#[only_in(guilds)]
async fn setlist(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let url = args.single::<String>()?;
    let requester = Requester::from_msg(&ctx, &msg).await;

    get_mstate!(mut, mstate, ctx);
    mstate.autoplay.register(requester, &url).ok();

    check_msg(msg.channel_id.say(&ctx.http, "Setlist Registered!").await);

    Ok(())
}


// TODO: implement an autoplay enabled check?
#[command]
#[aliases(up)]
#[only_in(guilds)]
#[min_args(0)]
#[max_args(1)]
async fn upcoming(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    get_mstate!(mstate, ctx);

    if !mstate.autoplay.enabled {
        check_msg(msg.channel_id.say(&ctx.http, "Autoplay is not enabled").await);
        return Ok(())
    }

    let num = args.single::<u64>().unwrap_or(10);

    check_msg(msg.channel_id.send_message(&ctx.http, |m| {
        m.embed(|e| { e
            .description(mstate.autoplay.show_upcoming(num))
        })
    }).await);

    Ok(())
}


#[command]
#[only_in(guilds)]
#[checks(in_same_voice)]
async fn enrolluser(ctx: &Context, msg: &Message) -> CommandResult {
    get_mstate!(mut, mstate, ctx);

    let ret = match mstate.autoplay.enable_user(&msg.author) {
        Ok(m) => m.to_string(),
        Err(e) => format!("Error enabling user: {:?}", e),
    };

    check_msg(msg.channel_id.say(&ctx.http, ret).await);

    Ok(())
}


#[command]
#[only_in(guilds)]
async fn removeuser(ctx: &Context, msg: &Message) -> CommandResult {
    get_mstate!(mut, mstate, ctx);

    let ret = match mstate.autoplay.disable_user(&msg.author) {
        Ok(m) => m.to_string(),
        Err(e) => format!("Error disabling user: {:?}", e),
    };

    check_msg(msg.channel_id.say(&ctx.http, ret).await);

    Ok(())
}


#[command]
#[only_in(guilds)]
#[checks(in_same_voice)]
// TODO: require permissions for this
async fn rebalance(ctx: &Context, msg: &Message) -> CommandResult {
    get_mstate!(mut, mstate, ctx);

    mstate.autoplay.reset_usertime();

    check_msg(msg.channel_id.say(&ctx.http, "Reset all users' autoplay scores to 0.").await);

    Ok(())
}


#[command]
#[only_in(guilds)]
#[checks(in_same_voice)]
// TODO: require permissions for this
async fn shuffle(ctx: &Context, msg: &Message) -> CommandResult {
    get_mstate!(mut, mstate, ctx);

    mstate.autoplay.shuffle_user(&msg.author).unwrap();

    check_msg(msg.channel_id.say(&ctx.http, "Shuffled your playlist.").await);

    Ok(())
}


#[command]
#[only_in(guilds)]
#[checks(in_same_voice)]
#[num_args(1)]
// TODO: require permissions for this
// TODO: come up with a better name for this command
async fn dump(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let num = match args.single::<u64>() {
        Ok(n) => n,
        Err(e) => {
            check_msg(msg.channel_id.say(&ctx.http,
                format!("Must provide a number of songs to dump from autoplay into queue: {:?}", e)).await);
            return Ok(());
        }
    };

    // TODO: use config autoplay prefetch maximum here
    let max = 20;

    if num > max {
        check_msg(msg.channel_id.say(&ctx.http, format!("Requested dump exceeds maximum allowed, max is {}", max)).await);

        return Ok(());
    }

    get_mstate!(mut, mstate, ctx);

    if !mstate.autoplay.enabled {
        // TODO: this can probably work without autoplay enabled, but users need to be registered, etc etc
        check_msg(msg.channel_id.say(&ctx.http, "Autoplay is not enabled.").await);
        return Ok(())
    }

    for i in 0..num {
        if let Some(song) = mstate.autoplay.next() {
            match mstate.enqueue(song) {
                Ok(_) => (),
                Err(MusicError::QueueFull) => {
                    check_msg(msg.channel_id.say(&ctx.http, format!("Queue capacity reached, only could add {}", i)).await);
                    break;
                },
                Err(e) => panic!("dump: {:?}", e),
            };
        }
        else {
            // TODO: probably better error handle this, this implies there's no autoplay users registered
            break;
        }
    }

    mstate.autoplay.enabled = false;
    check_msg(msg.channel_id.say(&ctx.http, "Autoplay has been disabled.").await);

    Ok(())
}