use std::{
    env,
    path::Path,
    sync::Arc,
};
use log::*;

use tokio::sync::Mutex;

use minstrel_config::{
    CONFIG,
};

use music::MusicState;

#[tokio::main]
async fn main() {
    if let Ok(path) = env::var("PATH") {
        if !path.split(':').into_iter()
                .any(|p| Path::new(p).join("yt-dlp").exists()) {
            panic!("yt-dlp could not be found in $PATH");
        }
    }
    else {
        panic!("Could not read $PATH variable!");
    }

    dotenv::dotenv().ok();

    env_logger::init();

    debug!("config = {:?}", *CONFIG);

    let (tx, rx) = tokio::sync::mpsc::channel(3);

    // TODO: I really don't like this flow, it needs to be handled by some higher level controller probably.
    let dplayer = Arc::new(Mutex::new(discord::player::DiscordPlayer::new()));
    let mut dplayertask = music::player::MusicPlayerTask::new(dplayer.clone(), rx);
    let mstate = Arc::new(Mutex::new(MusicState::new(tx)));


    let mut client = discord::client::create_player(mstate.clone(), dplayer.clone()).await;

    debug!("spawning discord player task");
    tokio::spawn(async move {
        dplayertask.run().await;
    });

    info!("spawning discord client");
    tokio::spawn(async move {
        if let Err(why) = client.start().await {
            error!("Client error: {:?}", why);
        }
    });

    /*
    // TODO: figure out a method of composing multiple filters if there ever are multiple filters
    #[cfg(feature = "discord-webdash")]
    let site = ddash;

    #[cfg(feature = "web-server")]
    {
        let addr = format!("{}:{}", read_config!(web.bind_address), read_config!(web.port))
            .parse::<std::net::SocketAddr>().unwrap();

        info!("spawning web server");
        warp::serve(site)
            .run(addr)
            .await;
    }
    */

    // TODO: Have an application controller that properly shuts things down and exists here
    loop {}
}
