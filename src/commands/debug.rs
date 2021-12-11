use serenity::{
    model::{
        channel::Message,
    },
    prelude::*,
    framework::standard::{
        Args,
        macros::command,
        CommandResult,
    },
};
use super::music;
use crate::get_mstate;


#[command]
#[only_in(guilds)]
async fn usertime(ctx: &Context, msg: &Message) -> CommandResult {
    get_mstate!(mstate, ctx);

    let ut = mstate.autoplay.debug_get_usertime();

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

    mstate.autoplay.disable_user(&member.user.clone()).unwrap();

    let ut = mstate.autoplay.debug_get_usertime();

    msg.channel_id.say(&ctx.http, format!("In theory dropped user:\n```{}```", ut)).await?;

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

    mstate.autoplay.add_time_to_user(&member.user.clone(), delta);

    msg.channel_id.say(&ctx.http, format!("In theory modified usertime")).await?;

    Ok(())
}
