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
use super::VOICE_READY_CHECK;
use super::music;
use super::music::{
    Song,
};


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
