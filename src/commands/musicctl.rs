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
use super::helpers::*;
use super::check_msg;
use super::music;
use super::music::{
    Song,
};
use super::music::Requester;


#[command]
#[only_in(guilds)]
async fn play(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    // TODO: confirm if this is actually needed
    let url = args.single::<String>()?;

    let requester = Requester::from_msg(&ctx, &msg).await;

    let url = match Song::new(url, requester) {
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

    if let Some(song) = mstate.current_song() {
        // TODO: consider making this a helper, so the sticky nowplaying can use this

        let md = song.metadata;
        let thumb = match md.thumbnail.clone() {
            Some(t) => t,
            None => String::from(
                format!("https://img.youtube.com/vi/{}/maxresdefault.jpg", &md.id)),
                // This URL might change in the future, but meh, it works.
                // TODO: Config the thumbnail resolution probably
        };

        check_msg(msg.channel_id.send_message(&ctx.http, |m| {
            m.embed(|e| { e
                .title(md.title)
                .thumbnail(thumb)
                .url(song.url)
                .description(md.uploader.unwrap_or(String::from("Unknown")))
                .footer(|f| { f
                    .icon_url(song.requested_by.user.face())
                    .text(format!("Requested by: {}", song.requested_by.name))
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
#[aliases(skip, n)]
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
