use serenity::{
    model::{
        channel::Message,
        id::UserId,
    },
    prelude::*,
    framework::standard::{
        help_commands,
        macros::{help, hook},
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
    user::*,
    //debug::*,
};

use crate::helpers::*;
use crate::{
    get_mstate,
    get_dstate,
};


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
    get_dstate!(mut, dstate, ctx);

    if let Some(m) = &dstate.sticky {
        m.channel_id.delete_message(&ctx.http, m).await.unwrap();

        let mdata = mstate.get_webdata().await;

        let qs_embed = get_queuestate_embed(&mdata, mstate.autoplay.is_enabled().await);
        let np_embed = get_nowplay_embed(ctx, &mdata).await;

        let new = m.channel_id.send_message(&ctx.http, |m| {
            m.set_embeds(vec![qs_embed, np_embed])
        }).await.unwrap();

        dstate.sticky = Some(new);
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
    .group(&USERCMD_GROUP)
    //.group(&DEBUGCMD_GROUP)
    .help(&HELPME)
}

