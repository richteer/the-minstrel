use serenity::{
    model::{
        channel::Message,
    },
    prelude::*,
    framework::standard::{
        macros::command,
        CommandResult,
    },
};

use crate::get_mstate;
use crate::join_voice;
use super::helpers::*;
use super::music;

#[command]
#[only_in(guilds)]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, "Pong! :)").await?;

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn join(ctx: &Context, msg: &Message) -> CommandResult {
    join_voice!(ctx, msg);
    Ok(())
}

#[command]
#[only_in(guilds)]
async fn leave(ctx: &Context, _msg: &Message) -> CommandResult {
    get_mstate!(mut, mstate, ctx);

    mstate.leave().await;

    Ok(())
}
