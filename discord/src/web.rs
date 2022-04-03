use serenity::Client;
use std::convert::Infallible;
use warp::Filter;
use crate::client::MusicStateKey;

use std::sync::Arc;
use tokio::sync::Mutex;

use music::MusicState;
use crate::player::DiscordPlayer;

use futures_util::{
    StreamExt,
    SinkExt
};

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
        .and(warp::path("api"))
        .and(mstate.clone())
        .and_then(show_state)
    .or(
    warp::path("ws")
        .and(warp::ws())
        .and(mstate)
        .then(async move |ws: warp::ws::Ws, mstate: Arc<Mutex<MusicState<DiscordPlayer>>>| {
            // And then our closure will be called when it completes...
            ws.on_upgrade(async move |websocket| {
                let mstate = mstate.lock().await;
                let player = mstate.player.as_ref().unwrap();
                let mut player = player.lock().await;
                let mut player = &mut *player;

                let (mut tx, _) = websocket.split();
                tx.send(warp::ws::Message::text("hi :)")).await.unwrap();

                if player.listeners.is_some() {
                    let mut ls = player.listeners.take().unwrap();
                    ls.push(tx);
                    player.listeners = Some(ls);
                } else {
                    player.listeners = Some(vec!(tx));
                }
            })
        })
    )
}