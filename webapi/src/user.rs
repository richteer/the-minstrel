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
        UserInfo, RegisterRequest, ReplyStatus, LinkInfo,
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


pub async fn handle_login(
    _user_auth: Option<String>,
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

    let reply = UserInfo {
        status: status.as_u16(),
        error,
        userinfo,
    };

    if let Some(token) = auth_token {
        Ok(warp::http::Response::builder()
            // TODO: probably set an expiry for these
            .header("Set-Cookie", format!("auth_token={}; httponly; Secure; SameSite=Strict;", token))
            .status(reply.status)
            .body(serde_json::to_string(&reply).unwrap()).unwrap())
    } else {
        Ok(warp::http::Response::builder()
            .status(reply.status)
            .body(serde_json::to_string(&reply).unwrap()).unwrap())
    }
}

pub async fn handle_register(
    _user_auth: Option<String>, // TODO: consider writing a filter that converts this to Option<MinstrelUserId>?
    mstate: MusicAdapter,
    tokens: Arc<Mutex<BiHashMap<MinstrelUserId, String>>>,
    body: RegisterRequest,
) -> Result<impl warp::Reply, Infallible> {

    // TODO: actually validate username/password requirements against a regex

    let auth = usermgmt::AuthType::UserAuth(body.username, body.password);
    let info = usermgmt::UserInfo { displayname: body.displayname, icon: body.icon };

    let (status, error, userinfo , auth_token) = if let Some(link) = body.link {
        let resp = mstate.user.user_link(link, auth).await;
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
    } else {
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

    let reply = UserInfo {
        status: status.as_u16(),
        error,
        userinfo,
    };

    if let Some(token) = auth_token {
        Ok(warp::http::Response::builder()
            // TODO: probably set an expiry for these
            .header("Set-Cookie", format!("auth_token={}; httponly; Secure; SameSite=Strict;", token))
            .status(reply.status)
            .body(serde_json::to_string(&reply).unwrap()).unwrap())
    } else {
        Ok(warp::http::Response::builder()
            .status(reply.status)
            .body(serde_json::to_string(&reply).unwrap()).unwrap())
    }
}

pub async fn handle_logout(
    user_auth: Option<String>,
    _mstate: MusicAdapter,
    tokens: Arc<Mutex<BiHashMap<MinstrelUserId, String>>>,
) -> Result<impl warp::Reply, Infallible> {

    if let Some(tok) = user_auth {
        let mut tokens = tokens.lock().await;
        if let Some(_) = tokens.remove_by_right(&tok) {
            Ok(warp::http::Response::builder()
                .header("Set-Cookie", r#"auth_token=""; httponly; Secure; SameSite=Strict;"#)
                .status(StatusCode::OK)
                .body(serde_json::to_string(&ReplyStatus::_ok()).unwrap()).unwrap())
        } else {
            Ok(warp::http::Response::builder()
                .header("Set-Cookie", r#"auth_token=""; httponly; Secure; SameSite=Strict;"#)
                .status(StatusCode::UNAUTHORIZED)
                .body(serde_json::to_string(&ReplyStatus::new(StatusCode::UNAUTHORIZED.as_u16().into(), "User not logged in, or invalid session ID")).unwrap()).unwrap())
        }
    } else {
        Ok(warp::http::Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(serde_json::to_string(&ReplyStatus::new(StatusCode::UNAUTHORIZED.as_u16().into(), "User not logged in, or invalid session ID")).unwrap()).unwrap())
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
            .body(serde_json::to_string(&ReplyStatus::new(StatusCode::UNAUTHORIZED.as_u16().into(), "User not logged in, or invalid session ID")).unwrap()).unwrap())
    };

    let tokens = tokens.lock().await;
    let user_id = if let Some(user_id) = tokens.get_by_right(&tok) {
        *user_id
    } else {
        return Ok(warp::http::Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(serde_json::to_string(&ReplyStatus::new(StatusCode::UNAUTHORIZED.as_u16().into(), "User not logged in, or invalid session ID")).unwrap()).unwrap())
    };
    drop(tokens); // No longer need to hold lock here

    let resp = mstate.user.create_link(user_id).await;
    let (status, error, link) = match resp {
        Ok(link) => (StatusCode::OK, "Link successfully created.".into(), Some(link)),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Something went really wrong internally: {e:?}"), None)
    };

    let reply = LinkInfo {
        status: status.as_u16(),
        error,
        link,
    };

    Ok(warp::http::Response::builder()
    // TODO: probably set an expiry for these
        .status(reply.status)
        .body(serde_json::to_string(&reply).unwrap()).unwrap())
}
