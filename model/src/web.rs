/// For all specifically web-related shared models, state structs, etc since the
/// web-frontend(s) and backend need more tight sharing of structs

use serde::{
    Deserialize,
    Serialize,
};

use crate::Requester;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ReplyData {
    UserInfo(Requester),
    LinkInfo(u64),
}

// TODO: Definitely make this way more robust, consider enuming and consider allowing
//   payload returns
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplyStatus {
    pub status: u16,
    // TODO: Consider using MusicOk/MusicError here, and allowing frontends
    //  to implement their own Display functions
    pub error: String,
    pub data: Option<ReplyData>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
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
pub struct ApBumpRequest {
    pub index: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ApToggleRequest {
    pub enabled: bool,
}