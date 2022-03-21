use log::*;

use serenity::{
    builder::CreateEmbed,
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

use std::{
    collections::HashMap,
    sync::Arc,
};

use super::music::MusicState;

use crate::discord::MusicStateKey;
use crate::discord::player::DiscordPlayer;

pub async fn mstate_get(ctx: &Context) -> Option<Arc<Mutex<MusicState<DiscordPlayer>>>> {
    let data = ctx.data.read().await;

    let mstate = data.get::<MusicStateKey>().cloned();

    mstate
}

// TODO: These can definitely be cleaner, but might as well macro out now to make
//  life slightly easier if I do end up needing to replace them
#[macro_export]
macro_rules! get_mstate {
    ($mstate:ident, $ctx:ident) => {
        let $mstate = crate::discord::mstate_get(&$ctx).await.unwrap();
        let $mstate = $mstate.lock().await;
    };

    ($mut:ident, $mstate:ident, $ctx:ident) => {
        let $mstate = crate::discord::mstate_get(&$ctx).await.unwrap();
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
/// TODO: this is still pretty messy, consider cleaning up
pub async fn _join_voice(ctx: &Context, msg: &Message) -> Result<bool, String> {
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

    get_mstate!(mut, mstate, ctx);
    if let Some(bot_channel) = bot_channel_id {
        if bot_channel == connect_to {
            if mstate.player.is_some() {
                return Ok(false); // We're done here, otherwise fall through and init
            }
        }
        else {
            return Err(String::from("Bot is in another voice channel"));
        }
    }

    mstate.player = Some(
        Arc::new(
            Mutex::new(
                Box::new(
                    DiscordPlayer::connect(&ctx, guild_id, connect_to).await
    ))));

    Ok(true)
}

#[macro_export]
/// Joins the channel if not already, otherwise dumps a message to the channel for why it didn't
/// Returns true if a channel was joined, false if was already in the channel
macro_rules! join_voice {
    ($ctx:ident, $msg:ident) => {{
        let ret = _join_voice($ctx, $msg).await;
        match ret {
            Ok(b) => b,
            Err(e) => {
                check_msg($msg.channel_id.say(&$ctx.http, format!("{}", e)).await);
                return Ok(());
            },
        }
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

/*** Functions that were previously on mstate, but for discord-specific output ***/
/**   These are subject to moving again, but can live here now for convenience  **/

pub fn show_queuestate(mstate: &MusicState<DiscordPlayer>) -> String {
    let mut q = None;
    let mut ap = None;

    if !mstate.is_queue_empty() {
        q = Some(mstate.show_queue());
    }

    if mstate.autoplay.enabled {
        ap = Some(mstate.autoplay.show_upcoming(10));
    }

    let mut ret = String::new();

    if let Some(his) = show_history(mstate, 5) {
        ret += &format!("{}\n", his);
    }

    if let Some(curr) = &mstate.current_song() {
        ret += &format!("Now Playing:\n:musical_note: {}\n\n", curr);
    }
    else {
        ret += &format!("_Nothing is currently playing._\n\n");
    }

    let tmp = match (q,ap) {
        (None,    None    ) => format!("Queue is empty and Autoplay is disabled"),
        (Some(q), None    ) => format!("{}\nAutoplay is disabled", q),
        (None,    Some(ap)) => format!("{}", ap),
        (Some(q), Some(ap)) => format!("{}\n{}", q, ap),
    };

    ret + &tmp
}


pub fn get_queuestate_embed(mstate: &MusicState<DiscordPlayer>) -> CreateEmbed {
    let mut ret = CreateEmbed { 0: HashMap::new() };

    ret.description(show_queuestate(mstate));

    return ret;
}

pub fn get_nowplay_embed(mstate: &MusicState<DiscordPlayer>) -> CreateEmbed {
    let mut ret = CreateEmbed { 0: HashMap::new() };

    let song = match mstate.current_song() {
        Some(s) => s,
        None => {
            ret.description("Nothing currently playing");
            return ret;
        }
    };

    let md = song.metadata;
    let thumb = match md.thumbnail.clone() {
        Some(t) => t,
        None => String::from(
            format!("https://img.youtube.com/vi/{}/maxresdefault.jpg", &md.id)),
            // This URL might change in the future, but meh, it works.
            // TODO: Config the thumbnail resolution probably
    };

    let mins = song.duration / 60;
    let secs = song.duration % 60;

    ret.thumbnail(thumb)
        .title(format!("{} [{}:{:02}]", md.title, mins, secs))
        .url(song.url)
        .description(format!("Uploaded by: {}",
            md.uploader.unwrap_or(String::from("Unknown")),
            )
        )
        .footer(|f| { f
            .icon_url(song.requested_by.user.face())
            .text(format!("Requested by: {}", song.requested_by.name))
        });

    ret
}

pub fn show_history(mstate: &MusicState<DiscordPlayer>, num: usize) -> Option<String> {
    if mstate.history.len() == 0 {
        return None
    }

    let mut ret = String::from("Last played songs:\n");

    for (i,s) in mstate.history.iter().take(num).enumerate().rev() {
        ret += &format!("{0}: {1}\n", i+1, s);
    }

    Some(ret)
}

pub fn get_history_embed(mstate: &MusicState<DiscordPlayer>, num: usize) -> CreateEmbed {
    let mut ret = CreateEmbed { 0: HashMap::new() };

    ret.description(match show_history(mstate, num) {
        Some(s) => s,
        None => String::from("No songs have been played"),
    });

    ret
}
