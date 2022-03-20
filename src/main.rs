use std::{
    env,
    path::Path,
};
use dotenv;
use log::*;

mod discord;
mod music;

#[tokio::main]
async fn main() {
    if let Ok(path) = env::var("PATH") {
        if path.split(":").into_iter()
            .find(|p| Path::new(p).join("yt-dlp").exists())
            .is_none() {
            panic!("yt-dlp could not be found in $PATH");
        }
    }
    else {
        panic!("Could not read $PATH variable!");
    }

    dotenv::dotenv().ok();

    env_logger::init();

    let mut client = discord::player::create_player().await;

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform
    // exponential backoff until it reconnects.
    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }
}
