use serenity::{
    model::{
        channel::Message,
    },
    prelude::*,
    framework::standard::{
        Args,
        macros::{
            command,
            group,
        },
        CommandResult,
    },
};
use crate::get_mstate;
use crate::requester::*;
use minstrel_config::CONFIG;


#[group]
#[description = "Commands for debugging purposes"]
#[prefix("debug")]
#[commands(usertime, dropapuser, addapuser, apenableall, modutime, musicstate, dumpconfig)]
// TODO: require owner
struct DebugCmd;


#[command]
#[only_in(guilds)]
async fn usertime(ctx: &Context, msg: &Message) -> CommandResult {
    get_mstate!(mstate, ctx);

    //let ut = mstate.autoplay.debug_get_usertime();

    msg.channel_id.say(&ctx.http, format!("```{}```", ut)).await?;

    Ok(())
}


#[command]
#[only_in(guilds)]
#[num_args(1)]
async fn dropapuser(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    get_mstate!(mut, mstate, ctx);

    let user = args.single::<String>().unwrap();
    let guild = ctx.cache
                .guild(msg.guild_id.unwrap())
                .await.unwrap();
    let member = guild.member_named(&user).unwrap();

    mstate.autoplay.disable_user(&muid_from_userid(&member.user.id)).unwrap();

    let ut = mstate.autoplay.debug_get_usertime();

    msg.channel_id.say(&ctx.http, format!("In theory dropped user:\n```{}```", ut)).await?;

    Ok(())
}


#[command]
#[only_in(guilds)]
#[num_args(1)]
async fn addapuser(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    get_mstate!(mut, mstate, ctx);

    let user = args.single::<String>().unwrap();
    let guild = ctx.cache
                .guild(msg.guild_id.unwrap())
                .await.unwrap();
    let member = guild.member_named(&user).unwrap();

    mstate.autoplay.enable_user(&muid_from_userid(&member.user.id)).unwrap();

    let ut = mstate.autoplay.debug_get_usertime();

    msg.channel_id.say(&ctx.http, format!("In theory added user:\n```{}```", ut)).await?;

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn apenableall(ctx: &Context, msg: &Message) -> CommandResult {
    get_mstate!(mut, mstate, ctx);

    mstate.autoplay.debug_enable_all_users();

    msg.channel_id.say(&ctx.http, format!("In theory enabled all users in autoplay.json")).await?;

    Ok(())
}


#[command]
#[only_in(guilds)]
#[num_args(2)]
async fn modutime(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    get_mstate!(mut, mstate, ctx);

    let user = args.single::<String>()?;
    let delta = args.single::<i64>()?;
    let guild = ctx.cache
                .guild(msg.guild_id.unwrap())
                .await.unwrap();

    let member = guild.member_named(&user).unwrap();

    mstate.autoplay.add_time_to_user(&muid_from_userid(&member.user.id), delta);

    msg.channel_id.say(&ctx.http, "In theory modified usertime").await?;

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn musicstate(ctx: &Context, msg: &Message) -> CommandResult {
    get_mstate!(mstate, ctx);

    msg.channel_id.send_message(&ctx.http, |m| {
        m.embed(|e| {
            e.description(format!("```{:?}```", mstate))
        })
    }).await.unwrap();

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn dumpconfig(ctx: &Context, msg: &Message) -> CommandResult {
    let conf = { CONFIG.read().unwrap().clone() };

    // TODO: move this to debug.rs
    // TODO: make a fancier config display function, perhaps one that is better copy-pasteable

    msg.channel_id.say(&ctx.http, format!("```{:?}```", conf)).await?;

    Ok(())
}
