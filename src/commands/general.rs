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

use super::VOICE_READY_CHECK;

#[command]
#[only_in(guilds)]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, "Pong! :)").await?;

    Ok(())
}

#[command]
#[only_in(guilds)]
#[checks(voice_ready)]
async fn join() -> CommandResult {
    Ok(())
}

#[command]
#[only_in(guilds)]
async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    if let Some(manager) = songbird::get(ctx).await {
        if let Some(handler) = manager.get(guild_id) {
            let mut handler = handler.lock().await;

            handler.stop();

            match handler.leave().await {
                Ok(()) => println!("left channel"),
                Err(e) => println!("failed to disconnect: {}", e),
            };
        }
    }

    Ok(())
}