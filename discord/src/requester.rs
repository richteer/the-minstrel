use music::requester::*;
use serenity::model::user::User;
use serenity::model::id::UserId;


pub fn requester_from_user(user: &User) -> Requester {
    let id = muid_from_userid(&user.id);
    Requester {
        user: UserModels::Discord(user.clone()),
        id: id,
    }
}

pub fn muid_from_userid(userid: &UserId) -> MinstrelUserId {
    MinstrelUserId {
        0: userid.to_string(),
    }
}
