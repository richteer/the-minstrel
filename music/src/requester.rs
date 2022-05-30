use serde::{Deserialize, Serialize};

//#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub type MinstrelUserId = String;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Requester {
    pub username: String,
    pub displayname: String,
    pub icon: String, // url
    pub id: MinstrelUserId, // same as MinstrelId probably
}

// Keeping this one around so that the API is consistent internally,
//  don't want to mess around with using a mix of remote structs and webdata structs

impl From<Requester> for model::Requester {
    fn from(req: Requester) -> Self {
        Self {
            username: req.username,
            displayname: req.displayname,
            icon: req.icon,
            id: req.id,
        }
    }
}

// TODO: Just use the same Requester everywhere?
impl From<model::Requester> for Requester {
    fn from(req: model::Requester) -> Self {
        Self {
            username: req.username,
            displayname: req.displayname,
            icon: req.icon,
            id: req.id,
        }
    }
}