use crate::music::requester::*;
use serenity::model::user::User;
use serenity::model::id::UserId;
use serenity::model::channel::Message;


impl Into<Requester> for User {
    fn into(self) -> Requester {
        let id = self.id.into();
        Requester {
            user: UserModels::Discord(self),
            id: id,
        }
    }
}

impl From<&User> for Requester {
    fn from(user: &User) -> Requester {
        Requester {
            user: UserModels::Discord(user.clone()),
            id: user.id.into(),
        }
    }
}

impl Into<MinstrelUserId> for UserId {
    fn into(self) -> MinstrelUserId {
        MinstrelUserId {
            0: self.to_string(),
        }
    }
}

impl From<&Message> for Requester {
    fn from(msg: &Message) -> Requester {
        Requester {
            user: UserModels::Discord(msg.author.clone()),
            id: msg.author.id.into(),
        }
    }
}