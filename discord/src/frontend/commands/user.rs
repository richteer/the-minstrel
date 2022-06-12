use music::adapters::{
    UserInfo,
    AuthType,
};
use model::{
    UserMgmtError,
};
use serenity::{
    model::{
        channel::Message,
    },
    prelude::*,
    framework::standard::{
        Args,
        macros::{
            group,
            command,
        },
        CommandResult,
    },
};

#[group]
#[commands(register, link)]
struct UserCmd;

use crate::get_mstate;

#[command]
async fn register(ctx: &Context, msg: &Message) -> CommandResult {
    get_mstate!(mstate, ctx);

    let displayname = if let Some(gid) = msg.guild_id {
        let displayname = msg.author.nick_in(&ctx.http, gid).await;
        match displayname {
            Some(d) => d,
            None => msg.author.name.clone(),
        }
    } else {
        msg.author.name.clone()
    };

    let info = UserInfo {
        displayname,
        icon: msg.author.avatar_url(),
    };

    let reply = match mstate.user.user_create(AuthType::Discord(*msg.author.id.as_u64()), info).await {
        Ok(_) => "Registered successfully!".into(),
        Err(UserMgmtError::UserExists) => "You have already registered! If you are trying to link to a web account, see `!help link`".into(),
        Err(e) => format!("An unknown error occurred: {:?}", e),
    };

    msg.reply(&ctx.http, reply).await?;

    Ok(())
}



#[command]
#[min_args(0)] // Create a new link
#[max_args(1)] // Apply a link
async fn link(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {

    // Create a new link
    if args.is_empty() {
        get_mstate!(mstate, ctx);

        let user_id = mstate.db.get_userid_from_discordid(*msg.author.id.as_u64())
            .await.unwrap();
        let user_id = if let Some(uid) = user_id {
            uid
        } else {
            msg.reply(&ctx.http, "You don't appear to be registered in discord, please run `!register` first.").await?;
            return Ok(())
        };

        let link = mstate.user.create_link(user_id).await;

        match link {
            Ok(link) => {
                let linktext = format!("Here is your link number:\n`{}`\nEnter this when registering a new account, or through the link option/menu.", link);
                let resp = msg.author.dm(&ctx.http, |m| m.content(linktext)).await;

                if let Err(e) = resp {
                    msg.reply(&ctx.http, format!("There was an error DM'ing you the link number: {:?}", e)).await?;
                }
            },
            Err(e) => {
                msg.reply(&ctx.http, format!("There was an error generating your link: {:?}", e)).await?;
            }
        }

        return Ok(())
    }

    match args.single::<u64>() {
        Ok(link) => {
            get_mstate!(mstate, ctx);

            let resp = mstate.user.user_link(link, AuthType::Discord(*msg.author.id.as_u64())).await;

            let response = match resp {
                Ok(_) => "User successfully linked!".into(),
                Err(UserMgmtError::InvalidLink) => "Link is invalid or expired.".into(),
                Err(UserMgmtError::UserExists) => "You already have a Discord auth, regenerate a link and use in some other auth.".into(),
                Err(UserMgmtError::UserDoesNotExist) => "You are somehow not registered? Register first then try linking.".into(),
                Err(e) => format!("There was a problem attempting to link users: {:?}", e),
            };

            msg.reply(&ctx.http, response).await?;
        },
        Err(_) => {
            msg.reply(&ctx.http, "Invalid format for link, it should be a number.").await?;
        },
    }

    Ok(())
}