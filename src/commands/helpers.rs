use log::*;

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
        error!("Error sending message: {:?}", why);
    }
}


/// Join voice chat of the command caller
/// Call this function *BEFORE* get_mstate!, as this will need to access mstate first
/// TODO: this is still pretty messy, consider cleaning up
pub async fn _join_voice(ctx: &Context, msg: &Message) -> Result<(), String> {
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
            return Err(String::from("You must be in a voice channel to use this command"));
        }
    };

    let mstate = music::get(&ctx).await.unwrap().clone();
    let mut mstate = mstate.lock().await;

    if let Some(bot_channel) = bot_channel_id {
        if bot_channel == connect_to {
            if mstate.is_ready() {
                return Ok(()); // We're done here, otherwise fall through and init
            }
        }
        else {
            return Err(String::from("Bot is in another voice channel"));
        }
    }

    mstate.init(&ctx, guild_id, connect_to).await;

    Ok(())
}

#[macro_export]
macro_rules! join_voice {
    ($ctx:ident, $msg:ident) => {{
        let ret = _join_voice($ctx, $msg).await;
        match ret {
            Ok(_) => (),
            Err(e) => {
                check_msg($msg.channel_id.say(&$ctx.http, format!("{}", e)).await);
                return Ok(());
            },
        };
    }};
}

// Check if the bot is in the same voice channel as the command caller
// Does not join voice, use join_voice! before get_mstate instead
#[check]
#[name = "in_same_voice"]
pub async fn in_same_voice(ctx: &Context, msg: &Message) -> Result<(), Reason> {
    let guild = msg.guild(&ctx.cache).await.unwrap();
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

    // Get caller's voice channel, bail if they aren't in one
    let bot_channel = match bot_channel_id {
        Some(channel) => channel,
        None => {
            return Err(Reason::User(String::from("The bot is not in a voice channel")));
        }
    };

    if bot_channel == connect_to {
        Ok(())
    }
    else {
        Err(Reason::User(String::from("Bot is in another voice channel")))
    }
}
