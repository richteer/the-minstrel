/// For all specifically web-related shared models, state structs, etc since the
/// web-frontend(s) and backend need more tight sharing of structs

use serde::{
    Deserialize,
    Serialize,
};

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