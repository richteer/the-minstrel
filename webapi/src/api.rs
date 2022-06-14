use std::sync::Arc;
use bimap::BiHashMap;
use tokio::sync::Mutex;
use music::{
    adapters::MusicAdapter,
    song::fetch_song_from_yt,
};
use model::{
    SongRequest,
    Requester, MinstrelUserId,
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
    Play(SongRequest),
    Skip,
    Stop,
    Start,
    Enqueue(SongRequest),
    EnqueueAndPlay(SongRequest),
    ClearQueue,
    Previous,
    SongEnded,
    GetData,
    AutoplayCmd(AutoplayControlCmd),
}

use model::web::ReplyStatus;

use crate::user::*;


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
    _cookie: Option<String>,
    mut mstate: MusicAdapter,
    _tokens: Arc<Mutex<BiHashMap<MinstrelUserId, String>>>,
    func: String,
    body: SongBody,
) -> Result<impl warp::Reply, Infallible> {
    debug!("body = '{:?}'", &body);

    let requester = Requester {
        displayname: "webuser".to_string(),
        icon: "".to_string(),
        id: 0,
    };

    let song = match fetch_song_from_yt(body.song.clone()) {
        Ok(s) => SongRequest::new(s, requester),
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
    _cookie: Option<String>,
    mut mstate: MusicAdapter,
    _tokens: Arc<Mutex<BiHashMap<MinstrelUserId, String>>>,
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
    let auths = Arc::new(Mutex::new(BiHashMap::<MinstrelUserId, String>::new()));
    let mstate = warp::any().map(move || { mstate.clone() });
    let authtable = warp::any().map(move || { auths.clone() });

    let body = warp::body::json()
        .and(warp::body::content_length_limit(256)); // Arbitrary length limit, we should not be expecting big data

    let api_base = warp::post()
        .and(warp::cookie::optional::<String>("auth_token"))
        .and(warp::path("api"))
        .and(mstate)
        .and(authtable);

    let api_func_base = api_base.clone()
        .and(warp::path::param::<String>()
        .and(warp::path::end()));

    let api_body = api_func_base.clone()
        .and(body.clone())
        .and_then(handle_body_api);

    let api_no_body = api_func_base.clone()
        .and_then(handle_simple_api);

    let login = api_base.clone()
        .and(warp::path("login"))
        .and(warp::path::end())
        .and(warp::body::json())
        .and_then(handle_login);

    let register = api_base.clone()
        .and(warp::path("register"))
        .and(warp::path::end())
        .and(warp::body::json())
        .and_then(handle_register);

    let link = api_base.clone()
        .and(warp::path("link"))
        .and(warp::path::end())
        .and(warp::body::json())
        .and_then(handle_link);

    let logout = api_base.clone()
        .and(warp::path("logout"))
        .and(warp::path::end())
        .and_then(handle_logout);

    // TODO: this should probably just be a GET?
    let userinfo = api_base.clone()
        .and(warp::path("userinfo"))
        .and(warp::path::end())
        .and_then(handle_userinfo);

    login
        .or(logout)
        .or(register)
        .or(link)
        .or(userinfo)
        .or(api_no_body)
        .or(api_body)
}