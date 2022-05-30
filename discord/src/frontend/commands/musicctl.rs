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

use crate::{
    get_mstate,
    get_dstate,
    join_voice,
};
use crate::helpers::*;
use crate::requester::*;
use music::{
    Song,
    MusicOk,
    MusicError,
};


#[command]
#[only_in(guilds)]
async fn play(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    // TODO: confirm if this is actually needed
    let url = args.single::<String>()?;

    let requester = requester_from_user(ctx, &msg.guild_id, &msg.author).await;

    let url = match Song::new(url, &requester) {
        Ok(u) => u,
        Err(_) => {
            check_msg(msg.channel_id.say(&ctx.http, "Must provide a URL to a video or audio").await);
            return Ok(())
        }
    };

    join_voice!(ctx, msg);
    get_mstate!(mut, mstate, ctx);
    let ret = mstate.enqueue_and_play(url).await;

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
async fn nowplaying(ctx: &Context, msg: &Message) -> CommandResult {
    get_mstate!(mstate, ctx);
    let mstate = mstate.get_webdata().await;

    let embed = get_nowplay_embed(ctx, &mstate).await;

    check_msg(msg.channel_id.send_message(&ctx.http, |m| {
        m.set_embed(embed)
    }).await);

    Ok(())
}

#[command]
#[aliases(skip)]
#[only_in(guilds)]
#[checks(in_same_voice)]
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
#[checks(in_same_voice)]
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
async fn start(ctx: &Context, msg: &Message) -> CommandResult {
    join_voice!(ctx, msg);
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
// TODO: consider permissions here, this might be annoying if regular users can toggle it
async fn display(ctx: &Context, msg: &Message) -> CommandResult {
    get_dstate!(mut, dstate, ctx);

    if dstate.sticky.is_some() {
        dstate.sticky = None;

        check_msg(msg.channel_id.say(&ctx.http, "Disabled sticky display.").await);

        return Ok(());
    }

    check_msg(msg.channel_id.say(&ctx.http, "Enabling sticky display message.").await);

    // Just send a blank message to fill in sticky, let the hook actually send the first output
    let sticky = msg.channel_id.say(&ctx.http, ".").await.unwrap();

    dstate.sticky = Some(sticky);

    Ok(())
}

#[command]
#[only_in(guilds)]
#[min_args(0)]
#[max_args(1)]
// TODO: probably add a bunch of other args to manage the output, order and such
async fn history(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let num = args.single::<usize>().unwrap_or(5);

    get_mstate!(mstate, ctx);
    let mstate = mstate.get_webdata().await;

    check_msg(msg.channel_id.send_message(&ctx.http, |m|
        m.set_embed(get_history_embed(&mstate, num))
    ).await);

    Ok(())
}


#[command]
#[only_in(guilds)]
#[checks(in_same_voice)]
async fn previous(ctx: &Context, msg: &Message) -> CommandResult {
    get_mstate!(mut, mstate, ctx);

    check_msg(msg.channel_id.say(&ctx.http, match mstate.previous().await {
        Ok(MusicOk::EnqueuedSong) => "Enqueued last played song.".to_string(),
        Ok(o) => o.to_string(),
        Err(MusicError::EmptyHistory) => "No history to pull a song from".to_string(),
        Err(e) => format!("{:?}", e),
    }).await);

    Ok(())
}
