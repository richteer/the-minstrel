use serenity::Client;
use std::convert::Infallible;
use warp::Filter;
use crate::discord::client::MusicStateKey;

use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::music::MusicState;
use crate::music::MusicStateStatus;
use crate::music::Song;
use crate::discord::player::DiscordPlayer;

use serde::Serialize;

#[derive(Serialize)]
struct DiscordDisplayState {
    current_track: Option<Song>,
    status: MusicStateStatus,
    queue: VecDeque<Song>,
    upcoming: Vec<Song>,
    history: VecDeque<Song>,
}

async fn show_state(
    mstate: Arc<Mutex<MusicState<DiscordPlayer>>>
) -> Result<impl warp::Reply, Infallible> {
    let ret = {
        let mstate = mstate.lock().await;

        DiscordDisplayState {
            current_track: mstate.current_track.clone(),
            status: mstate.status.clone(),
            queue: mstate.queue.clone(),
            history: mstate.history.clone(),
            upcoming: mstate.autoplay.prefetch(10).unwrap(),
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

