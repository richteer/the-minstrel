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