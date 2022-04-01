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

use config::{Config, Source};
use crate::conf::{CONFIG, Configuration};

#[command]
#[only_in(guilds)]
#[num_args(2)]
async fn set(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let key = args.single::<String>().unwrap();
    let val = args.single::<String>().unwrap();

    // TODO: validate key before attempting any of this

    let new: Configuration = {
        let conf = CONFIG.read().unwrap();

        let temp = Config::try_from(&*conf).unwrap();
        Config::builder()
            .add_source(temp)
            .set_override(&key, val.clone())?
            .build()
            .unwrap()
            .try_deserialize()
            .unwrap()
    };

    {
        let mut conf = CONFIG.write().unwrap();
        *conf = new;
    }

    msg.channel_id.say(&ctx.http, format!("`{} = {}`", key, val)).await?;

    Ok(())
}



#[command]
#[only_in(guilds)]
#[min_args(0)]
#[max_args(1)]
async fn get(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let conf = {
        let conf = CONFIG.read().unwrap();
        Config::try_from(&*conf).unwrap()
    };

    if let Ok(key) = args.single::<String>() {
        let val = conf.get_string(key.as_str()).unwrap();
        msg.channel_id.say(&ctx.http, format!("`{} = {}`", key, val)).await?;
    } else {
        // TODO: make the generic "get all" option display much much nicer
        let ret: String = conf.collect().unwrap().iter()
            .map(|(k,v)| format!("{} = {}\n", k,v))
            .collect();

        msg.channel_id.say(&ctx.http, format!("```{}```", ret)).await?;
    };

    Ok(())
}
