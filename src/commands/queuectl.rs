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

use crate::get_mstate;
use super::check_msg;
use super::music;
use super::music::{
    Song,
};
use super::VOICE_READY_CHECK;


#[command]
#[aliases(q, showqueue)]
#[only_in(guilds)]
#[checks(voice_ready)] // TODO: implement "in same voice channel" and use here, don't need to join
async fn queue(ctx: &Context, msg: &Message) -> CommandResult {
    let mstate = music::get(&ctx).await.unwrap();
    let mstate = mstate.lock().await;

    check_msg(msg.channel_id.say(&ctx.http, mstate.show_queue()).await);

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
#[only_in(guilds)]
#[checks(voice_ready)] // TODO: implement "in same voice channel" and use here, don't need to join
async fn setlist(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let url = args.single::<String>()?;
    let mstate = music::get(&ctx).await.unwrap();
    let mut mstate = mstate.lock().await;

    mstate.autoplay.register(&msg.author, &url).ok();

    check_msg(msg.channel_id.say(&ctx.http, "Setlist Registered!").await);

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

// TODO: implement an autoplay enabled check?
// TODO: perhaps make this a subcommand of !autoplay?
#[command]
#[aliases(up)]
#[only_in(guilds)]
#[checks(voice_ready)] // TODO: implement "in same voice channel" and use here, don't need to join
#[min_args(0)]
#[max_args(1)]
async fn upcoming(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    get_mstate!(mstate, ctx);

    if !mstate.autoplay.enabled {
        check_msg(msg.channel_id.say(&ctx.http, "Autoplay is not enabled").await);
        return Ok(())
    }

    let num = args.single::<u64>().unwrap_or(10);

    check_msg(msg.channel_id.say(&ctx.http, mstate.autoplay.show_upcoming(num)).await);

    Ok(())
}