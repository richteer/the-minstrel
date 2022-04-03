use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct MinstrelUserId(pub String);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Requester {
    pub username: String,
    pub displayname: String,
    pub icon: String, // url
    pub id: MinstrelUserId, // same as MinstrelId probably
}