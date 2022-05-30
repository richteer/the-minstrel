use warp::Filter;

use std::sync::Arc;
use tokio::sync::Mutex;

use log::*;

use music::MusicState;

use futures_util::{
    StreamExt,
    SinkExt
};


async fn ws_connect(ws: warp::ws::Ws, mstate: Arc<Mutex<MusicState>>) -> impl warp::reply::Reply {
    ws.on_upgrade(|websocket| async move {
        let mstate = mstate.lock().await;

        let (mut ws_tx, _) = websocket.split();

        let mut bc_rx = mstate.subscribe();

        tokio::task::spawn(async move {
            // TODO: figure out a nicer way to assign these task or thread IDs, would be nice for debug
            debug!("spawning ws thread");
            while let Ok(msg) = bc_rx.recv().await {
                trace!("broadcast received, sending to websocket");
                let msg = serde_json::to_string(&msg).unwrap();
                if let Err(resp) = ws_tx.send(warp::ws::Message::text(msg)).await {
                    debug!("websocket appears to have disconnected, dropping? {}", resp);
                    break;
                }
            }
            debug!("exiting websocket loop!");
        });
    })
}

pub fn get_web_filter(mstate: Arc<Mutex<MusicState>>) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let mstate = warp::any().map(move || { mstate.clone() });

    let api = warp::get()
        .and(warp::path("api"))
        .and(mstate.clone())
        .and_then(crate::api::show_state);

    let ws = warp::path("ws")
        .and(warp::ws())
        .and(mstate)
        .then(ws_connect);

    let files = crate::embed::get_embedded_file_filter();

    api.or(ws).or(files)
}