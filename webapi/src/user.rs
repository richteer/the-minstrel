use std::sync::Arc;
use bimap::BiHashMap;
use tokio::sync::Mutex;
use music::{
    adapters::MusicAdapter,
};
use model::{
    web::{
        LoginRequest,
        UserInfo,
    }, MinstrelUserId,
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

            tokens.lock().await.insert(id, token.clone());

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