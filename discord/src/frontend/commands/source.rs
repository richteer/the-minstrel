use model::{
    SourceType,
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

use crate::get_mstate;

#[group]
#[prefixes("source", "sources", "src")]
#[commands(add, show, update, remove)]
struct SourceCmd;

#[command]
#[num_args(1)]
async fn add(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let url = args.single::<String>()?;

    get_mstate!(mut, mstate, ctx);

    let muid = mstate.db.get_userid_from_discordid(*msg.author.id.as_u64()).await;
    let muid = match muid {
        Ok(Some(id)) => id,
        Ok(None) => {
            msg.reply(&ctx.http, "You are not registered.").await?;
            return Ok(())
        },
        Err(e) => {
            msg.reply(&ctx.http, format!("Error attempting to get userid: {:?}", e)).await?;
            return Ok(())
        }
    };

    let resp = mstate.db.create_source(muid, &SourceType::YoutubePlaylist(url), true).await;
    if resp.is_err() {
        msg.reply(&ctx.http, "Failed to add source").await?;
        return Ok(())
    }

    let req = mstate.db.get_requester(muid).await.unwrap();
    // TODO: put this on another thread
    let resp = mstate.autoplay.update_userplaylist(&req).await;
    match resp {
        Ok(_) => msg.reply(&ctx.http, "Added source and refreshed upcoming!").await?,
        Err(e) => msg.reply(&ctx.http, format!("Error updating sources: {:?}", e)).await?,
    };

    Ok(())
}

#[command]
async fn show(ctx: &Context, msg: &Message) -> CommandResult {
    get_mstate!(mstate, ctx);

    let muid = mstate.db.get_userid_from_discordid(*msg.author.id.as_u64()).await;
    let muid = match muid {
        Ok(Some(id)) => id,
        Ok(None) => {
            msg.reply(&ctx.http, "You are not registered.").await?;
            return Ok(())
        },
        Err(e) => {
            msg.reply(&ctx.http, format!("Error attempting to get userid: {:?}", e)).await?;
            return Ok(())
        }
    };

    let sources = mstate.db.get_sources_from_userid(muid, false).await;
    let mut sources = match sources {
        Ok(srcs) => srcs,
        Err(e) => {
            msg.reply(&ctx.http, format!("Error fetching sources: {:?}", e)).await?;
            return Ok(())
        }
    };

    if sources.is_empty() {
        msg.reply(&ctx.http, "You have no sources.").await?;
        return Ok(())
    }

    sources.sort_by_key(|e| e.id);

    let mut output = "```\n".to_string();
    for (i, src) in sources.iter().enumerate() {
        output += match &src.path {
            SourceType::YoutubePlaylist(url) => format!("{}: {}\n", i+1, url),
        }.as_str();
    }
    output += "```";

    msg.reply(&ctx.http, output).await?;

    Ok(())
}

#[command]
#[min_args(0)]
#[max_args(1)]
#[aliases("refresh", "up", "ref")]
async fn update(ctx: &Context, msg: &Message, mut _args: Args) -> CommandResult {
    // TODO: update only selected source

    get_mstate!(mut, mstate, ctx);

    let muid = mstate.db.get_userid_from_discordid(*msg.author.id.as_u64()).await;
    let muid = match muid {
        Ok(Some(id)) => id,
        Ok(None) => {
            msg.reply(&ctx.http, "You are not registered.").await?;
            return Ok(())
        },
        Err(e) => {
            msg.reply(&ctx.http, format!("Error attempting to get userid: {:?}", e)).await?;
            return Ok(())
        }
    };

    let req = mstate.db.get_requester(muid).await.unwrap();
    // TODO: put this on another thread
    let resp = mstate.autoplay.update_userplaylist(&req).await;
    match resp {
        Ok(_) => msg.reply(&ctx.http, "Added source and refreshed upcoming!").await?,
        Err(e) => msg.reply(&ctx.http, format!("Error updating sources: {:?}", e)).await?,
    };

    Ok(())
}

#[command]
#[num_args(1)]
#[aliases("delete")]
async fn remove(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let index = args.single::<u64>()?;

    if index == 0 {
        msg.reply(&ctx.http, "Use the number from `!source show`, it is not zero indexed.").await?;
        return Ok(())
    }
    let index = (index - 1) as usize; // Except we are zero indexed, technically

    get_mstate!(mstate, ctx);

    let muid = mstate.db.get_userid_from_discordid(*msg.author.id.as_u64()).await;
    let muid = match muid {
        Ok(Some(id)) => id,
        Ok(None) => {
            msg.reply(&ctx.http, "You are not registered.").await?;
            return Ok(())
        },
        Err(e) => {
            msg.reply(&ctx.http, format!("Error attempting to get userid: {:?}", e)).await?;
            return Ok(())
        }
    };

    let sources = mstate.db.get_sources_from_userid(muid, false).await;
    let mut sources = match sources {
        Ok(srcs) => srcs,
        Err(e) => {
            msg.reply(&ctx.http, format!("Error fetching sources: {:?}", e)).await?;
            return Ok(())
        }
    };

    if sources.is_empty() {
        msg.reply(&ctx.http, "You have no sources.").await?;
        return Ok(())
    }

    if sources.len() < index {
        msg.reply(&ctx.http, format!("Source #{} does not exist", index)).await?;
        return Ok(())
    }

    sources.sort_by_key(|e| e.id);

    let srcid = sources[index].id;

    match mstate.db.delete_source(srcid).await {
        Ok(_) => msg.reply(&ctx.http, "Deleted source successfully!").await?,
        Err(_) => msg.reply(&ctx.http, "There was an error attempting to remove the source.").await?,
    };

    Ok(())
}