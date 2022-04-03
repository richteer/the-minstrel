use serde::{Deserialize, Serialize};

// TODO: Either move this somewhere else, or reconsider how this can be done more
//  dynamically. Templates and traitbounds could work, or just use dyn Trait
//  as the entries to enum? Or just make that be the struct field, and let the
//  caller figure that out?
#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub enum UserModels {
    Discord(serenity::model::user::User),
}

impl UserModels {
    pub fn get_name(&self) -> &String {
        match self {
            UserModels::Discord(u) => &u.name,
            #[allow(unreachable_patterns)]
            _ => todo!("Unknown user model, implement me!"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct MinstrelUserId(pub String);

pub trait ToRequester<T> {
    fn to_requester(self) -> Requester;
}

/// Struct to hold requested-by information for MusicState and friends
/// Fill with anything that is not in the User struct that might be useful
//#[derive(Clone, Debug, Serialize, Deserialize)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Requester {
    pub user: UserModels,
    // User nickname in server or name without discriminator
    //pub name: String,
    pub id: MinstrelUserId,
}

