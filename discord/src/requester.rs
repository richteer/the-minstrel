use model::{
    Requester,
    MinstrelUserId,
};
use serenity::model::user::User;
use serenity::model::id::{
    UserId,
    GuildId,
};
use serenity::client::Context;
use log::*;


pub async fn requester_from_user(ctx: &Context, gid: &Option<GuildId>, user: &User) -> Requester {
    let id = muid_from_userid(&user.id);
    let displayname = if let Some(gid) = gid {
        user.nick_in(&ctx.http, gid).await.unwrap_or_else(|| user.name.clone())
    } else {
        user.name.clone()
    };

    Requester {
        username: user.tag(),

        // TODO: this should probably use nick_in, perhaps create yet another wrapper to cache this?
        displayname,
        icon: user.face(),
        id,
    }
}

pub fn muid_from_userid(userid: &UserId) -> MinstrelUserId {
    userid.to_string()
}

pub async fn get_user_from_muid(ctx: &Context, muid: &MinstrelUserId) -> Option<User> {
    let uid = muid.parse::<u64>().unwrap();
    let uid = UserId(uid);

    match uid.to_user(&ctx.http).await {
        Ok(o) => Some(o),
        Err(e) => {
            warn!("lookup for muid = {} returned error: {:?}", muid, e);
            None
        },
    }
}
