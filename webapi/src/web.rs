use model::MinstrelBroadcast;
use warp::Filter;

use std::sync::Arc;
use tokio::sync::{
    Mutex,
    broadcast::error::RecvError
};

use log::*;

use music::{
    adapters::MusicAdapter,
};

use futures_util::{
    StreamExt,
    SinkExt
};


async fn ws_connect(ws: warp::ws::Ws, mstate: Arc<Mutex<MusicAdapter>>) -> impl warp::reply::Reply {
    ws.on_upgrade(|websocket| async move {
        let mstate = mstate.lock().await.clone();

        let (mut ws_tx, _) = websocket.split();

        let mut bc_rx = mstate.subscribe();

        tokio::task::spawn(async move {
            // TODO: figure out a nicer way to assign these task or thread IDs, would be nice for debug
            debug!("spawning ws thread");

            {
                debug!("sending initial state");
                let msg = mstate.get_webdata().await;
                let msg = MinstrelBroadcast::MusicState(msg);
                let msg = serde_json::to_string(&msg).unwrap();
                let msg = warp::ws::Message::text(msg);
                if let Err(resp) = ws_tx.send(msg).await {
                    error!("websocket appears to have disconnected before it could even receive the initial state {}", resp);
                    return;
                };
            }

            loop {
                match bc_rx.recv().await {
                    Ok(msg) => {
                        trace!("broadcast received, sending to websocket");
                        let msg = serde_json::to_string(&msg).unwrap();
                        if let Err(resp) = ws_tx.send(warp::ws::Message::text(msg)).await {
                            debug!("websocket appears to have disconnected, dropping? {}", resp);
                            break;
                        }
                    },
                    Err(RecvError::Lagged(c)) => error!("Lagging behind: {c:?}"),
                    Err(RecvError::Closed) => {
                        error!("broadcast appears closed, exiting loop");
                        break;
                    }
                }
            }
            debug!("exiting websocket loop!");
        });
    })
}

pub fn get_web_filter(mstate: MusicAdapter) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let mstate_mutex = Arc::new(Mutex::new(mstate.clone()));
    let mstate_filter = warp::any().map(move || { mstate_mutex.clone() });

    let api = crate::api::get_api_filter(mstate);

    let ws = warp::path("ws")
        .and(warp::ws())
        .and(mstate_filter)
        .then(ws_connect);

    let files = crate::embed::get_embedded_file_filter();

    let root_redir = warp::get()
        .and(warp::path::end())
        .map(|| {
            warp::redirect::redirect(warp::hyper::Uri::from_static("/index.html"))
        });

    api.or(ws).or(root_redir).or(files)
}