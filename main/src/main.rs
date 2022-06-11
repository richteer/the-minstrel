use std::{
    env,
    path::Path,
    sync::Arc,
};
use log::*;

use tokio::sync::Mutex;

use minstrel_config::{
    CONFIG,
    read_config,
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

    let db = db::init_db().await;

    let (tx, rx) = tokio::sync::mpsc::channel(3);
    let mut mstate = MusicState::new(tx, db.clone()).await;

    // TODO: I really don't like this flow, it needs to be handled by some higher level controller probably.

    #[cfg(feature = "discord")]
    {
        // TODO: make this under a discord-player feature, depends on splitting DiscordPlayer into a DiscordState probably
        let dplayer = Arc::new(Mutex::new(discord::player::DiscordPlayer::new()));
        let mut dplayertask = music::player::MusicPlayerTask::new(dplayer.clone(), rx);

        debug!("spawning discord player task");
        tokio::spawn(async move {
            dplayertask.run().await;
        });

        let mut client = discord::client::create_player(mstate.get_adapter(), dplayer.clone()).await;


        info!("spawning discord client");
        tokio::spawn(async move {
            if let Err(why) = client.start().await {
                error!("Client error: {:?}", why);
            }
        });
    }

    #[cfg(feature = "web-frontend")]
    {
        let site = webapi::web::get_web_filter(mstate.get_adapter());
        let addr = format!("{}:{}", read_config!(web.bind_address), read_config!(web.port))
            .parse::<std::net::SocketAddr>().unwrap();

        info!("spawning web server");
        tokio::spawn(async move {
            warp::serve(site)
            .run(addr)
            .await;
        });
    }

    // TODO: Have an application controller that properly shuts things down and exits here

    // This currently never ends (probably?), so use this to keep the application waiting until end
    info!("spawning musicmanager task");
    mstate.run().await;
}
