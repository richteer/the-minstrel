use music::song::fetch_song_from_yt;
use serenity::{
    model::{
        channel::Message,
    },
    prelude::*,
    framework::standard::{
        Args,
        macros::{
            command,
            group,
        },
        CommandResult,
    },
};

use crate::get_mstate;
use crate::helpers::*;
use crate::userconv::*;

use model::{
    SongRequest
};


#[group]
#[description = "Commands to manage the music queue"]
#[commands(queue, enqueue, clearqueue, queuestatus)]
struct QueueControlCmd;


#[command]
#[aliases(q, showqueue)]
#[only_in(guilds)]
async fn queue(ctx: &Context, msg: &Message) -> CommandResult {
    get_mstate!(mstate, ctx);
    let mstate = mstate.get_webdata().await;

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

    get_mstate!(mut, mstate, ctx);

    let requester = mstate.requester_from_user(ctx, &msg.guild_id, &msg.author).await;

    let song = match fetch_song_from_yt(url) {
        Ok(u) => u,
        Err(_) => { // TODO: actually handle errors, probably make a generic surrender replier
            check_msg(msg.channel_id.say(&ctx.http, "Must provide a URL to a video or audio").await);
            return Ok(())
        }
    };

    let song = SongRequest::new(song, requester);



    let ret = mstate.enqueue(song).await;

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

    let ret = mstate.clear_queue().await;

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
    get_mstate!(mut, mstate, ctx);

    let ap_enabled = mstate.autoplay.is_enabled().await;
    let mstate = mstate.get_webdata().await;

    let embed = get_queuestate_embed(&mstate, ap_enabled);

    check_msg(msg.channel_id.send_message(&ctx.http, |m| {
        m.set_embed(embed)
    }).await);

    Ok(())
}
