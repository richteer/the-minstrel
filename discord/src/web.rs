use serenity::Client;
use std::convert::Infallible;
use warp::Filter;
use crate::client::MusicStateKey;

use std::sync::Arc;
use tokio::sync::Mutex;

use log::*;

use music::MusicState;
use crate::player::DiscordPlayer;

use futures_util::{
    StreamExt,
    SinkExt
};

use minstrel_config::read_config;

use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../webdash/dist/"]
struct EmbeddedWebdash;

pub fn get_mstate_webdata(mstate: &MusicState<DiscordPlayer>) -> webdata::MinstrelWebData {
    let upcoming = mstate.autoplay.prefetch(read_config!(discord.webdash_prefetch))
        // TODO: Better handle when autoplay is not enabled, or no users are enrolled
        .unwrap_or_else(|| Vec::new()).iter()
            .map(|e| e.clone().into())
            .collect();

    webdata::MinstrelWebData {
        current_track: match mstate.current_track.clone() {
            Some(s) => Some(s.into()),
            None => None,
        },
        status: mstate.status.clone().into(),
        queue: mstate.queue.iter().map(|e| e.clone().into()).collect(),
        upcoming: upcoming,
        history: mstate.history.iter().map(|e| e.clone().into()).collect(),
    }
}


async fn show_state(
    mstate: Arc<Mutex<MusicState<DiscordPlayer>>>
) -> Result<impl warp::Reply, Infallible> {
    let ret = {
        let mstate = mstate.lock().await;

        get_mstate_webdata(&mstate);
    };

    Ok(warp::reply::json(&ret))
}

pub async fn get_web_filter(client: &Client) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let mstate = client.data.read().await.get::<MusicStateKey>().cloned().unwrap();
    let mstate = warp::any().map(move || { mstate.clone() });

    let api = warp::get()
        .and(warp::path("api"))
        .and(mstate.clone())
        .and_then(show_state);

    let ws = warp::path("ws")
        .and(warp::ws())
        .and(mstate)
        .then(async move |ws: warp::ws::Ws, mstate: Arc<Mutex<MusicState<DiscordPlayer>>>| {
            // And then our closure will be called when it completes...
            ws.on_upgrade(async move |websocket| {
                let mstate = mstate.lock().await;
                let player = mstate.player.as_ref().unwrap();
                let player = player.lock().await;

                let (mut ws_tx, _) = websocket.split();

                let mut bc_rx = player.bcast.subscribe();

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
        });

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