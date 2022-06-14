/// For all specifically web-related shared models, state structs, etc since the
/// web-frontend(s) and backend need more tight sharing of structs

use serde::{
    Deserialize,
    Serialize,
};

use crate::Requester;

// TODO: Definitely make this way more robust, consider enuming and consider allowing
//   payload returns
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplyStatus {
    pub status: u64,
    // TODO: Consider using MusicOk/MusicError here, and allowing frontends
    //  to implement their own Display functions
    pub error: String,
}

impl ReplyStatus {
    pub fn new(status: u64, error: &str) -> Self {
        Self {
            status,
            error: String::from(error)
        }
    }

    pub fn _ok() -> Self {
        Self {
            status: 200,
            error: "ok".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserInfo {
    pub status: u16,
    pub userinfo: Option<Requester>,
    pub error: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub displayname: String,
    pub icon: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LinkRequest {
    pub username: String,
    pub password: String,
    pub link: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LinkInfo {
    pub status: u16,
    pub link: Option<u64>,
    pub error: String,
}