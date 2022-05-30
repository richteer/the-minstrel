use serenity::{
    model::{
        channel::Message,
        id::UserId,
    },
    prelude::*,
    framework::standard::{
        help_commands,
        macros::{group, help, hook},
        Args,
        CommandGroup,
        CommandError,
        CommandResult,
        DispatchError,
        HelpOptions,
        StandardFramework,
    },
};

use std::{
    collections::HashSet,
};

use crate::frontend::commands::{
    general::*,
    musicctl::*,
    queuectl::*,
    autoplay::*,
    config::*,
    //debug::*,
};

use crate::helpers::*;
use crate::{
    get_mstate,
    get_dplayer,
};



#[group]
#[commands(ping, join, leave)]
struct General;

#[group]
#[description = "Commands for controlling the music player"]
#[commands(play, nowplaying, next, stop, start, display, history, previous)]
struct MusicControlCmd;

#[group]
#[description = "Commands to manage the music queue"]
#[commands(queue, enqueue, clearqueue, queuestatus)]
struct QueueControlCmd;

#[group]
#[description = "Commands to manage autoplay state"]
#[prefixes("autoplay", "ap")]
#[commands(toggle, setlist, upcoming, enrolluser, removeuser, rebalance, shuffle, dump, advance)]
struct AutoplayCmd;

#[group]
#[description = "Commands for reading or manipulating config"]
#[prefix("config")]
#[commands(set, get)]
// TODO: require owner
struct ConfigCmd;

/*
#[group]
#[description = "Commands for debugging purposes"]
#[prefix("debug")]
#[commands(usertime, dropapuser, addapuser, apenableall, modutime, musicstate, dumpconfig)]
// TODO: require owner
struct DebugCmd;
*/

#[hook]
async fn dispatch_error(ctx: &Context, msg: &Message, error: DispatchError) {
    match error {
        DispatchError::CheckFailed(s, reason) =>
            msg.channel_id.say(&ctx.http, format!("Command failed: {:?} {:?}", s, reason)).await.unwrap(),
        err => msg.channel_id.say(&ctx.http, format!("Error executing command: {:?}", err)).await.unwrap(),
    };
}

#[hook]
async fn stickymessage_hook(ctx: &Context, _msg: &Message, _cmd_name: &str, _error: Result<(), CommandError>) {
    get_mstate!(mut, mstate, ctx);
    get_dplayer!(mut, dplayer, ctx);

    if let Some(m) = &dplayer.sticky {
        m.channel_id.delete_message(&ctx.http, m).await.unwrap();

        let embed = get_nowplay_embed(ctx, &mstate.get_webdata()).await;

        let new = m.channel_id.send_message(&ctx.http, |m| {
            m.add_embeds(vec![get_queuestate_embed(&mut *mstate), embed])
        }).await.unwrap();

        dplayer.sticky = Some(new);
    }
}


#[help]
#[command_not_found_text = "Could not find: {}"]
#[max_levenshtein_distance(3)]
async fn helpme(
    context: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    let _ = help_commands::with_embeds(context, msg, args, help_options, groups, owners).await;
    Ok(())
}

pub fn init_framework() -> StandardFramework {
    StandardFramework::new()
    .configure(|c| c
        .with_whitespace(true)
        //.on_mention(Some(bot_id)) // TODO: not sure
        .prefix("!")
        .delimiters(vec![", ", ",", " "])
        //.owners(owners) // TODO: set owners so adminy commands work
        )
    .after(stickymessage_hook)
    .on_dispatch_error(dispatch_error)
    .group(&GENERAL_GROUP)
    .group(&MUSICCONTROLCMD_GROUP)
    .group(&QUEUECONTROLCMD_GROUP)
    .group(&AUTOPLAYCMD_GROUP)
    .group(&CONFIGCMD_GROUP)
    //.group(&DEBUGCMD_GROUP)
    .help(&HELPME)
}

