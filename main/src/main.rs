use std::{
    env,
    path::Path,
};
use log::*;

use minstrel_config::{
    CONFIG,
    read_config,
};

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

    #[cfg(feature = "discord-webdash")]
    let ddash;

    #[cfg(feature = "discord-player")]
    {
        let mut client = discord::client::create_player().await;

        #[cfg(all(feature = "discord-webdash"))]
        {
            ddash = discord::web::get_web_filter(&client).await;
        }

        info!("spawning discord player");
        tokio::spawn(async move {
            if let Err(why) = client.start().await {
                error!("Client error: {:?}", why);
            }
        });
    }

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

    info!("Exiting...");
}
