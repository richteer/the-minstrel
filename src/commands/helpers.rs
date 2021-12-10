use serenity::{
    model::{
        channel::Message,
    },
    prelude::*,
    framework::standard::{
        macros::check,
        Reason,
    },
    Result as SerenityResult,
};

use super::super::music;


#[check]
#[name = "voice_ready"]
// TODO: uhhhh yeah so this gets called by help, so i guess i'm really going to have to factor out this
// TODO: can this be moved into mstate::get(), so it can just be automagic?
async fn voice_ready(ctx: &Context, msg: &Message) -> Result<(), Reason> {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;
    let bot_id = ctx.cache.current_user_id().await;

    let caller_channel_id = guild
        .voice_states.get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id);

    let bot_channel_id = guild
        .voice_states.get(&bot_id)
        .and_then(|voice_state| voice_state.channel_id);

    // Get caller's voice channel, bail if they aren't in one
    let connect_to = match caller_channel_id {
        Some(channel) => channel,
        None => {
            return Err(Reason::User(String::from("You must be in a voice channel to use this command")));
        }
    };

    if let Some(bot_channel) = bot_channel_id {
        if bot_channel == connect_to {
            return Ok(())
        }
        else {
            return Err(Reason::User(String::from("Bot is in another voice channel")));
        }
    }


    let mstate = music::get(&ctx).await.unwrap().clone();
    let mut mstate = mstate.lock().await;
    mstate.init(&ctx, guild_id, connect_to).await;

    Ok(())
}

// TODO: These can definitely be cleaner, but might as well macro out now to make
//  life slightly easier if I do end up needing to replace them
#[macro_export]
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

/// Checks that a message successfully sent; if not, then logs why to stdout.
pub fn check_msg(result: SerenityResult<Message>) {
    if let Err(why) = result {
        println!("Error sending message: {:?}", why);
    }
}