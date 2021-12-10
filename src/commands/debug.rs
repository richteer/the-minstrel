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

// TODO: figure out how to actually share this, this is a pain.
macro_rules! get_mstate {
    ($mstate:ident, $ctx:ident) => {
        let $mstate = music::get(&$ctx).await.unwrap();
        let $mstate = $mstate.lock().await;
    };

    ($mut:ident, $mstate:ident, $ctx:ident) => {
        let $mstate = music::get(&$ctx).await.unwrap();
        let $mut $mstate = $mstate.lock().await;
    };
}

#[command]
#[only_in(guilds)]
async fn usertime(ctx: &Context, msg: &Message) -> CommandResult {
    get_mstate!(mstate, ctx);

    let ut = mstate.autoplay.debug_get_usertime();

    msg.channel_id.say(&ctx.http, format!("```{}```", ut)).await?;

    Ok(())
}
