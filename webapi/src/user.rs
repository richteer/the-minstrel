use std::sync::Arc;
use bimap::BiHashMap;
use tokio::sync::Mutex;
use music::{
    adapters::{
        MusicAdapter,
        usermgmt,
    }
};
use model::{
    web::{
        LoginRequest,
        RegisterRequest, ReplyStatus, LinkRequest, ReplyData,
    }, MinstrelUserId, UserMgmtError,
};
use std::convert::Infallible;

use warp::{
    hyper::StatusCode,
};

use rand::{
    distributions::Alphanumeric,
    Rng,
};

fn gen_auth_token() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(128)
        .map(char::from)
        .collect()
}

use crate::ReplyStatusFuncs;

#[cfg(not(debug_assertions))]
// Require HTTPS for cookie support in-release mode, permit it in debug.
// TODO: figure out a way to require an https reverse proxy, fail login otherwise
const COOKIEOPTS: &str = "httponly; Secure; SameSite=Strict";
#[cfg(debug_assertions)]
const COOKIEOPTS: &str = "httponly; SameSite=Strict";


pub async fn handle_login(
    mstate: MusicAdapter,
    tokens: Arc<Mutex<BiHashMap<MinstrelUserId, String>>>,
    body: LoginRequest,
) -> Result<impl warp::Reply, Infallible> {
    let auth = mstate.user.user_authenticate(&body.username, body.password).await;

    let (status, error, userinfo, auth_token) = match auth {
        Ok(Some(id)) => {
            let req = mstate.db.get_requester(id).await.unwrap();
            let token = gen_auth_token();

            {
                tokens.lock().await.insert(id, token.clone());
            }

            (StatusCode::OK, "Login Successful".into(), Some(req), Some(token))
        },
        Ok(None) => (StatusCode::UNAUTHORIZED, "Incorrect username or password".into(), None, None),
        Err(model::UserMgmtError::DbError) => (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong with the database".into(), None, None),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Something really went wrong internally: {:?}", e), None, None),
    };

    let userinfo = userinfo.map(ReplyData::UserInfo);
    let reply = ReplyStatus::new(status, error, userinfo);

    if let Some(token) = auth_token {
        Ok(warp::http::Response::builder()
            // TODO: probably set an expiry for these
            .header("Set-Cookie", format!("auth_token={token}; {COOKIEOPTS}"))
            .status(reply.status)
            .body(serde_json::to_string(&reply).unwrap()).unwrap())
    } else {
        Ok(warp::http::Response::builder()
            .status(reply.status)
            .body(serde_json::to_string(&reply).unwrap()).unwrap())
    }
}

pub async fn handle_register(
    mstate: MusicAdapter,
    tokens: Arc<Mutex<BiHashMap<MinstrelUserId, String>>>,
    body: RegisterRequest,
) -> Result<impl warp::Reply, Infallible> {

    // TODO: actually validate username/password requirements against a regex

    let auth = usermgmt::AuthType::UserAuth(body.username, body.password);
    let info = usermgmt::UserInfo { displayname: body.displayname, icon: body.icon };

    let (status, error, userinfo , auth_token) = {
        let resp = mstate.user.user_create(auth, info).await;
        match resp {
            Ok(id) => {
                let req = mstate.db.get_requester(id).await.unwrap();
                let token = gen_auth_token();

                // TODO: ensure tokens are unique, a collision here would be really bad
                {
                    tokens.lock().await.insert(id, token.clone());
                }

                (StatusCode::OK, "User successfully created.".into(), Some(req), Some(token))
            },
            Err(UserMgmtError::UserExists) => (StatusCode::UNAUTHORIZED, "Username has already been taken.".into(), None, None),
            Err(UserMgmtError::DbError) => (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong with the database".into(), None, None),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Something went really wrong internally: {e:?}"), None, None),
        }
    };

    let userinfo = userinfo.map(ReplyData::UserInfo);
    let reply = ReplyStatus::new(status, error, userinfo);

    if let Some(token) = auth_token {
        Ok(warp::http::Response::builder()
            // TODO: probably set an expiry for these
            .header("Set-Cookie", format!("auth_token={token}; {COOKIEOPTS}"))
            .status(reply.status)
            .body(serde_json::to_string(&reply).unwrap()).unwrap())
    } else {
        Ok(warp::http::Response::builder()
            .status(reply.status)
            .body(serde_json::to_string(&reply).unwrap()).unwrap())
    }
}

pub async fn handle_link(
    mstate: MusicAdapter,
    tokens: Arc<Mutex<BiHashMap<MinstrelUserId, String>>>,
    body: LinkRequest,
) -> Result<impl warp::Reply, Infallible> {

    // TODO: validate username/password
    let link = body.link;
    let auth = usermgmt::AuthType::UserAuth(body.username, body.password);

    let resp = mstate.user.user_link(link, auth).await;
    let (status, error, userinfo , auth_token) = {
            match resp {
            Ok(id) => {
                let req = mstate.db.get_requester(id).await.unwrap();
                let token = gen_auth_token();

                {
                    tokens.lock().await.insert(id, token.clone());
                }

                (StatusCode::OK, "User linked successfully.".into(), Some(req), Some(token))
            },
            Err(UserMgmtError::InvalidLink) => (StatusCode::UNAUTHORIZED, "Invalid or expired link, please regenerate and try again.".into(), None, None),
            Err(UserMgmtError::DbError) => (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong with the database".into(), None, None),
            Err(UserMgmtError::UserExists) => (StatusCode::UNAUTHORIZED, "You appear to have an account already, please recreate this link and reuse with a different auth method (e.g. discord).".into(), None, None),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Something really went wrong internally: {:?}", e), None, None),
        }
    };

    let userinfo = userinfo.map(ReplyData::UserInfo);
    let reply = ReplyStatus::new(status, error, userinfo);

    if let Some(token) = auth_token {
        Ok(warp::http::Response::builder()
            // TODO: probably set an expiry for these
            .header("Set-Cookie", format!("auth_token={token}; {COOKIEOPTS}"))
            .status(reply.status)
            .body(serde_json::to_string(&reply).unwrap()).unwrap())
    } else {
        Ok(warp::http::Response::builder()
            .status(reply.status)
            .body(serde_json::to_string(&reply).unwrap()).unwrap())
    }}


pub async fn handle_logout(
    _mstate: MusicAdapter,
    tokens: Arc<Mutex<BiHashMap<MinstrelUserId, String>>>,
    user_auth: Option<String>,
) -> Result<impl warp::Reply, Infallible> {

    if let Some(tok) = user_auth {
        let mut tokens = tokens.lock().await;
        if tokens.remove_by_right(&tok).is_some() {
            Ok(warp::http::Response::builder()
                .header("Set-Cookie", format!(r#"auth_token=""; {COOKIEOPTS}"#))
                .status(StatusCode::OK)
                .body(serde_json::to_string(&ReplyStatus::ok()).unwrap()).unwrap())
        } else {
            Ok(warp::http::Response::builder()
                .header("Set-Cookie", format!(r#"auth_token=""; {COOKIEOPTS}"#))
                .status(StatusCode::UNAUTHORIZED)
                .body(serde_json::to_string(&ReplyStatus::new_nd(StatusCode::UNAUTHORIZED, "User not logged in, or invalid session ID")).unwrap()).unwrap())
        }
    } else {
        Ok(warp::http::Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(serde_json::to_string(&ReplyStatus::new_nd(StatusCode::UNAUTHORIZED, "User not logged in, or invalid session ID")).unwrap()).unwrap())
    }

}

pub async fn handle_create_link(
    user_auth: Option<String>,
    mstate: MusicAdapter,
    tokens: Arc<Mutex<BiHashMap<MinstrelUserId, String>>>,
) -> Result<impl warp::Reply, Infallible> {
    let tok = if let Some(tok) = user_auth {
        tok
    } else {
        return Ok(warp::http::Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(serde_json::to_string(&ReplyStatus::new_nd(StatusCode::UNAUTHORIZED, "User not logged in, or invalid session ID")).unwrap()).unwrap())
    };

    let tokens = tokens.lock().await;
    let user_id = if let Some(user_id) = tokens.get_by_right(&tok) {
        *user_id
    } else {
        return Ok(warp::http::Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(serde_json::to_string(&ReplyStatus::new_nd(StatusCode::UNAUTHORIZED, "User not logged in, or invalid session ID")).unwrap()).unwrap())
    };
    drop(tokens); // No longer need to hold lock here

    let resp = mstate.user.create_link(user_id).await;
    let (status, error, link) = match resp {
        Ok(link) => (StatusCode::OK, "Link successfully created.".into(), Some(link)),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Something went really wrong internally: {e:?}"), None)
    };

    let link = link.map(ReplyData::LinkInfo);
    let reply = ReplyStatus::new(status, error, link);

    Ok(warp::http::Response::builder()
    // TODO: probably set an expiry for these
        .status(reply.status)
        .body(serde_json::to_string(&reply).unwrap()).unwrap())
}

pub async fn handle_userinfo(
    muid: MinstrelUserId,
    mstate: MusicAdapter,
) -> Result<impl warp::Reply, Infallible> {

    let (status, userinfo, error) = {
        let req = mstate.db.get_requester(muid).await;
        match req {
            Ok(req) => (StatusCode::OK, Some(req), "UserInfo Retrieved".into()),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, None, format!("Something went really wrong internally: {e:?}")),
        }
    };

    let userinfo = userinfo.map(ReplyData::UserInfo);
    let reply = ReplyStatus::new(status, error, userinfo);

    let resp = warp::http::Response::builder()
        .status(reply.status);

    // Clear bogus cookie if it wasn't accepted
    let resp = if !status.is_success() {
        resp.header("Set-Cookie", format!(r#"auth_token=""; {COOKIEOPTS}"#))
    } else { resp };

    let resp = resp.body(serde_json::to_string(&reply).unwrap()).unwrap();

    Ok(resp)
}