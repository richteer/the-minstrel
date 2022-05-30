use serenity::{
    model::{
        channel::Message,
    },
    prelude::*,
    framework::standard::{
        macros::{
            group,
            command,
        },
        CommandResult,
    },
};

use crate::get_dstate;
use crate::join_voice;
use crate::helpers::*;

#[group]
#[commands(ping, join, leave)]
struct General;


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
    get_dstate!(mut, dstate, ctx);

    dstate.leave().await;

    Ok(())
}
