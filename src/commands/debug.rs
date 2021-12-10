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
use super::music;
use crate::get_mstate;


#[command]
#[only_in(guilds)]
async fn usertime(ctx: &Context, msg: &Message) -> CommandResult {
    get_mstate!(mstate, ctx);

    let ut = mstate.autoplay.debug_get_usertime();

    msg.channel_id.say(&ctx.http, format!("```{}```", ut)).await?;

    Ok(())
}
