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

// Keeping this one around so that the API is consistent internally,
//  don't want to mess around with using a mix of remote structs and webdata structs
impl Into<webdata::Requester> for Requester {
    fn into(self) -> webdata::Requester {
        webdata::Requester {
            username: self.username,
            displayname: self.displayname,
            icon: self.icon,
            id: self.id.0,
        }
    }
}