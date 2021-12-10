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
use super::helpers::*;
use super::check_msg;
use super::music;
use super::music::{
    Song,
    Requester,
};


#[command]
#[aliases(q, showqueue)]
#[only_in(guilds)]
async fn queue(ctx: &Context, msg: &Message) -> CommandResult {
    let mstate = music::get(&ctx).await.unwrap();
    let mstate = mstate.lock().await;

    check_msg(msg.channel_id.send_message(&ctx.http, |m| {
        m.embed(|e| { e
            .description(mstate.show_queue())
        })
    }).await);

    Ok(())
}

#[command]
#[aliases(append)]
#[only_in(guilds)]
#[checks(in_same_voice)]
async fn enqueue(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let url = args.single::<String>()?;

    let requester = Requester::from_msg(&ctx, &msg).await;

    let url = match Song::new(url, requester) {
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
#[checks(in_same_voice)]
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
#[aliases(qs)]
#[only_in(guilds)]
async fn queuestatus(ctx: &Context, msg: &Message) -> CommandResult {
    get_mstate!(mstate, ctx);

    let mut q = None;
    let mut ap = None;

    if !mstate.is_queue_empty() {
        q = Some(mstate.show_queue());
    }

    if mstate.autoplay.enabled {
        ap = Some(mstate.autoplay.show_upcoming(5));
    }

    let mut ret = String::new();

    if let Some(curr) = &mstate.current_song() {
        ret += &format!("Now Playing:\n{}\n\n", curr);
    }
    else {
        ret += &format!("_Nothing is currently playing._\n\n");
    }

    let tmp = match (q,ap) {
        (None,    None    ) => format!("Queue is empty and Autoplay is disabled"),
        (Some(q), None    ) => format!("{}\nAutoplay is disabled", q),
        (None,    Some(ap)) => format!("{}", ap),
        (Some(q), Some(ap)) => format!("{}\n{}", q, ap),
    };

    ret += &tmp;

    check_msg(msg.channel_id.send_message(&ctx.http, |m| {
        m.embed(|e| { e
            .description(ret)
        })
    }).await);

    Ok(())
}
