use std::sync::Arc;
use bimap::BiHashMap;
use tokio::sync::Mutex;
use music::{
    adapters::MusicAdapter,
    song::fetch_song_from_yt,
    autoplay::AutoplayError,
    MusicError,
};
use model::{
    SongRequest,
    Requester, MinstrelUserId,
    web::*,
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
use crate::ReplyStatusFuncs;


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
    _muid: MinstrelUserId,
    mut mstate: MusicAdapter,
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
            return Ok(warp::reply::json(&ReplyStatus::new_nd(StatusCode::BAD_REQUEST, format!("error fetching song: {e:?}"))).into_response())
    };

    let ret = match func.as_str() {
        "play" => mstate.play(song).await,
        "enqueue" => mstate.enqueue(song).await,
        "enqueueandplay" => mstate.enqueue_and_play(song).await,
        _ => return Ok(warp::reply::json(&ReplyStatus::new_nd(StatusCode::BAD_REQUEST, "no such function")).into_response())
    };

    match ret {
        Ok(o) => Ok(warp::reply::json(&ReplyStatus::new_nd(StatusCode::OK, &o.to_string())).into_response()),
        Err(e) => {
            debug!("error from musicstatus: {:?}", e);

            let resp = warp::reply::json(&ReplyStatus::new_nd(StatusCode::BAD_REQUEST, format!("{e:?}")));
            let mut resp = resp.into_response();
            *resp.status_mut() = StatusCode::BAD_REQUEST;

            Ok(resp)
        }
    }
}

async fn handle_simple_api(
    _muid: MinstrelUserId,
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
        Ok(o) => Ok(warp::reply::json(&ReplyStatus::new_nd(StatusCode::OK, &o.to_string())).into_response()),
        Err(e) => {
            debug!("error from musicstatus: {:?}", e);

            let resp = warp::reply::json(&ReplyStatus::new_nd(StatusCode::BAD_REQUEST, format!("{e:?}")));
            let mut resp = resp.into_response();
            *resp.status_mut() = StatusCode::BAD_REQUEST;

            Ok(resp)
        }
    }
}

async fn handle_ap_bump(
    muid: MinstrelUserId,
    mut mstate: MusicAdapter,
    body: ApBumpRequest,
) -> Result<impl warp::Reply, Rejection> {
    match mstate.autoplay.bump_userplaylist(&muid, body.index).await {
        Ok(_) => Ok(warp::reply::json(&ReplyStatus::ok())),
        Err(e) => Ok(warp::reply::json(&ReplyStatus::new_nd(StatusCode::BAD_REQUEST, format!("Error: {e:?}"))))
    }
}

async fn handle_ap_toggle(
    muid: MinstrelUserId,
    mut mstate: MusicAdapter,
    body: ApToggleRequest,
) -> Result<impl warp::Reply, Rejection> {

    let ret = match body.enabled {
        true => mstate.autoplay.enable().await,
        false => mstate.autoplay.disable().await,
    };
    if let Err(e) = ret {
        return Ok(warp::reply::json(&ReplyStatus::new_nd(StatusCode::INTERNAL_SERVER_ERROR, format!("error toggling autoplay: {e:?}"))))
    }

    // TODO: consider reporting errors to the user here
    if mstate.autoplay.is_enabled().await {
        debug!("entered in the enabled section");
        match mstate.autoplay.enable_user(&muid).await {
            Ok(_) => (),
            Err(AutoplayError::AlreadyEnrolled) => debug!("already enrolled"), // This is fine.
            Err(e) => error!("Error enabling user after ap toggle: {:?}", e),
        }

        match mstate.start().await {
            Ok(_) => (),
            Err(MusicError::AlreadyPlaying) => debug!("already playing"), // This is also fine.
            Err(e) => error!("Error starting playback after ap toggle: {:?}", e)
        }
    }

    Ok(warp::reply::json(&ReplyStatus::ok()))
}


pub fn get_api_filter(mstate: MusicAdapter) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let auths = Arc::new(Mutex::new(BiHashMap::<MinstrelUserId, String>::new()));
    let mstate = warp::any().map(move || { mstate.clone() });
    let authtable = warp::any().map(move || { auths.clone() });

     // TODO: probably make a custom rejection and handle that as a 401 Unauthorized
    let cookie_to_muid = warp::any()
        .and(authtable.clone())
        .and(warp::cookie::cookie::<String>("auth_token"))
        .and_then(|authtable: Arc<Mutex<BiHashMap<i64, String>>>, auth_token: String| async move {
            let table = authtable.lock().await;
            match table.get_by_right(&auth_token) {
                Some(uid) => Ok(*uid),
                None => {
                    debug!("rejecting, token = {auth_token:?}, table = {authtable:?}");
                    Err(warp::reject::reject())
                }
            }
        });

    let body = warp::body::json()
        .and(warp::body::content_length_limit(256)); // Arbitrary length limit, we should not be expecting big data

    let api_base = warp::post()
        .and(warp::path("api"))
        .and(cookie_to_muid.clone())
        .and(mstate.clone());

    let api_func_base = api_base.clone()
        .and(warp::path::param::<String>()
        .and(warp::path::end()));

    let api_body = api_func_base.clone()
        .and(body.clone())
        .and_then(handle_body_api);

    let api_no_body = api_func_base.clone()
        .and_then(handle_simple_api);

    let api_user_base = warp::post()
        .and(warp::path("api")
        .and(mstate.clone())
        .and(authtable.clone()));

    let login = api_user_base.clone()
        .and(warp::path("login"))
        .and(warp::path::end())
        .and(warp::body::json())
        .and_then(handle_login);

    let register = api_user_base.clone()
        .and(warp::path("register"))
        .and(warp::path::end())
        .and(warp::body::json())
        .and_then(handle_register);

    let link = api_user_base.clone()
        .and(warp::path("link"))
        .and(warp::path::end())
        .and(warp::body::json())
        .and_then(handle_link);

    let logout = api_user_base.clone()
        .and(warp::cookie::optional::<String>("auth_token"))
        .and(warp::path("logout"))
        .and(warp::path::end())
        .and_then(handle_logout);

    // TODO: this should probably just be a GET?
    let userinfo = api_base.clone()
        .and(warp::path("userinfo"))
        .and(warp::path::end())
        .and_then(handle_userinfo);

    let autoplay_ap_bump = api_base.clone()
        .and(warp::path("autoplay"))
        .and(warp::path("bump"))
        .and(warp::path::end())
        .and(warp::body::json())
        .and_then(handle_ap_bump);

    let autoplay_toggle = api_base.clone()
        .and(warp::path("autoplay"))
        .and(warp::path("toggle"))
        .and(warp::path::end())
        .and(warp::body::json())
        .and_then(handle_ap_toggle);

    // TODO: seriously clean up this filter building, this is getting out of hand
    login
        .or(logout)
        .or(register)
        .or(link)
        .or(userinfo)
        .or(autoplay_ap_bump)
        .or(autoplay_toggle)
        .or(api_no_body)
        .or(api_body)
}