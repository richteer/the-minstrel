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

pub async fn get_web_filter(client: &Client) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let mstate = client.data.read().await.get::<MusicStateKey>().cloned().unwrap();
    let mstate = warp::any().map(move || { mstate.clone() });

    warp::get()
        .and(mstate)
        .and_then(show_state)
}