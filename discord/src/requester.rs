use model::{
    Requester,
    MinstrelUserId,
};
use music::musiccontroller::MusicAdapter;
use serenity::model::user::User;
use serenity::model::id::{
    UserId,
    GuildId,
};
use serenity::client::Context;
use log::*;


pub async fn requester_from_user(ctx: &Context, mstate: &MusicAdapter, gid: &Option<GuildId>, user: &User) -> Requester {
    let id = muid_from_userid(mstate, &user.id).await;
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

pub async fn muid_from_userid(mstate: &MusicAdapter, userid: &UserId) -> MinstrelUserId {
    // TODO: error handle this
    mstate.db.get_userid_from_discordid(userid.0).await.unwrap()
}

pub async fn get_user_from_muid(ctx: &Context, mstate: &MusicAdapter, muid: &MinstrelUserId) -> Option<User> {
    let discordid = mstate.db.get_discordid_from_userid(*muid).await.unwrap();
    let discordid = if let Some(d) = discordid {
        d
    } else {
        return None
    };

    let uid = UserId(discordid);


    match uid.to_user(&ctx.http).await {
        Ok(o) => Some(o),
        Err(e) => {
            warn!("lookup for muid = {} returned error: {:?}", muid, e);
            None
        },
    }
}
