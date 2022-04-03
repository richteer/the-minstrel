use serenity::Client;
use std::convert::Infallible;
use warp::Filter;
use crate::client::MusicStateKey;

use std::sync::Arc;
use tokio::sync::Mutex;

use music::MusicState;
use crate::player::DiscordPlayer;

async fn show_state(
    mstate: Arc<Mutex<MusicState<DiscordPlayer>>>
) -> Result<impl warp::Reply, Infallible> {
    let ret = {
        let mstate = mstate.lock().await;

        webdata::MinstrelWebData {
            current_track: match mstate.current_track.clone() {
                Some(s) => Some(s.into()),
                None => None,
            },
            status: mstate.status.clone().into(),
            queue: mstate.queue.iter().map(|e| e.clone().into()).collect(),
            upcoming: mstate.autoplay.prefetch(10).unwrap().iter().map(|e| e.clone().into()).collect(),
            history: mstate.history.iter().map(|e| e.clone().into()).collect(),
        }
    };

    Ok(warp::reply::json(&ret))
}

pub async fn start_webserver(client: &Client) {
    let mstate = client.data.read().await.get::<MusicStateKey>().cloned().unwrap();

    tokio::spawn(async move {
        let mstate = warp::any().map(move || { mstate.clone() });

        let dash = warp::get()
            .and(mstate)
            .and_then(show_state);

        warp::serve(dash)
            .run(([127,0,0,1], 3030))
            .await;
    });
}
