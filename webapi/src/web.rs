// TODO: rename this file probably
use std::convert::Infallible;
use warp::Filter;

use std::sync::Arc;
use tokio::sync::Mutex;

use log::*;

use music::MusicState;

use futures_util::{
    StreamExt,
    SinkExt
};

use rust_embed::RustEmbed;


#[derive(RustEmbed)]
#[folder = "../webdash/dist/"]
struct EmbeddedWebdash;


async fn show_state(
    mstate: Arc<Mutex<MusicState>>
) -> Result<impl warp::Reply, Infallible> {
    let ret = {
        let mstate = mstate.lock().await;

        mstate.get_webdata()
    };

    Ok(warp::reply::json(&ret))
}

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
        .and_then(show_state);

    let ws = warp::path("ws")
        .and(warp::ws())
        .and(mstate)
        .then(ws_connect);

    let files = warp::get()
        .and(warp::path::param())
        .map(|filename: String| {
            let file = EmbeddedWebdash::iter().find(|f| *f == filename);
            debug!("GET /{}", filename);

            if let Some(data) = file {
                let mime = mime_guess::from_path(filename.as_str()).first();
                let data = EmbeddedWebdash::get(&data).unwrap().data;

                if let Some(mime) = mime {
                    debug!("mime = {}", mime);
                    warp::http::Response::builder()
                        .header("Content-Type", mime.to_string())
                        .body(Vec::from(data))
                } else {
                    warp::http::Response::builder().status(500).body(Vec::new())
                }
            } else {
                warn!("file not embedded: {}", filename);
                warp::http::Response::builder().status(404).body(Vec::new())
            }
        });

    api.or(ws).or(files)
}