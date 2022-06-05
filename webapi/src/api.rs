use std::sync::Arc;
use tokio::sync::Mutex;
use music::{
    musiccontroller::MusicAdapter,
    Song,
    requester::*,
};
use std::convert::Infallible;
use serde::{
    Serialize,
    Deserialize,
};

use warp::{
    Filter,
    Reply,
    Rejection,
    hyper::StatusCode,
};

use log::*;

// TODO: delete all this, just here for reference
//type Song = String;
type AutoplayControlCmd = String;

pub enum MusicControlCmd {
    Play(Song),
    Skip,
    Stop,
    Start,
    Enqueue(Song),
    EnqueueAndPlay(Song),
    ClearQueue,
    Previous,
    SongEnded,
    GetData,
    AutoplayCmd(AutoplayControlCmd),
}

use model::web::ReplyStatus;


#[derive(Serialize, Deserialize, Clone, Debug)]
struct SongBody {
    pub song: String,
}


pub async fn show_state(
    mstate: Arc<Mutex<MusicAdapter>>
) -> Result<impl warp::Reply, Infallible> {
    let ret = {
        let mstate = mstate.lock().await;

        mstate.get_webdata().await
    };

    Ok(warp::reply::json(&ret))
}

// TODO: Unify these, or implement handlers for each unique endpoint
async fn handle_body_api(
    mut mstate: MusicAdapter,
    func: String,
    body: SongBody,
) -> Result<impl warp::Reply, Infallible> {
    debug!("body = '{:?}'", &body);

    let requester = Requester {
        username: "webuser".to_string(),
        displayname: "webuser".to_string(),
        icon: "".to_string(),
        id: "0".to_string(),
    };

    let song = match Song::new(body.song.clone(), &requester) {
        Ok(s) => s,
        Err(e) =>
            return Ok(warp::reply::json(&ReplyStatus::new(400, &format!("error fetching song: {:?}", e))).into_response())
    };

    let ret = match func.as_str() {
        "play" => mstate.play(song).await,
        "enqueue" => mstate.enqueue(song).await,
        "enqueueandplay" => mstate.enqueue_and_play(song).await,
        _ => return Ok(warp::reply::json(&ReplyStatus::new(400, "no such function")).into_response())
    };

    match ret {
        Ok(o) => Ok(warp::reply::json(&ReplyStatus::new(200, &o.to_string())).into_response()),
        Err(e) => {
            debug!("error from musicstatus: {:?}", e);

            let resp = warp::reply::json(&ReplyStatus::new(StatusCode::BAD_REQUEST.as_u16() as u64, &format!("{:?}", e)));
            let mut resp = resp.into_response();
            *resp.status_mut() = StatusCode::BAD_REQUEST;

            Ok(resp)
        }
    }
}

async fn handle_simple_api(
    mut mstate: MusicAdapter,
    func: String,
) -> Result<impl warp::Reply, Rejection> {

    debug!("called simple, func = '{}'", &func);
    let ret = match func.as_str() {
        "skip" => mstate.skip().await,
        "stop" => mstate.stop().await,
        "start" => mstate.start().await,
        "clearqueue" => mstate.clear_queue().await,
        "previous" => mstate.previous().await,
        _ => return Err(warp::reject::reject())
    };

    match ret {
        Ok(o) => Ok(warp::reply::json(&ReplyStatus::new(200, &o.to_string())).into_response()),
        Err(e) => {
            debug!("error from musicstatus: {:?}", e);

            let resp = warp::reply::json(&ReplyStatus::new(StatusCode::BAD_REQUEST.as_u16() as u64, &format!("{:?}", e)));
            let mut resp = resp.into_response();
            *resp.status_mut() = StatusCode::BAD_REQUEST;

            Ok(resp)
        }
    }
}

pub fn get_api_filter(mstate: MusicAdapter) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let mstate = warp::any().map(move || { mstate.clone() });

    let body = warp::body::json()
        .and(warp::body::content_length_limit(256)); // Arbitrary length limit, we should not be expecting big data

    let api_base = warp::post()
        .and(warp::path("api"))
        .and(mstate)
        .and(warp::path::param::<String>()
        .and(warp::path::end()));

    let api_body = api_base.clone()
        .and(body)
        .and_then(handle_body_api);

    let api_no_body = api_base
        .and_then(handle_simple_api);

    api_no_body.or(api_body)
}