use async_trait::async_trait;
use model::{
    Requester,
    MinstrelUserId,
};
use music::adapters::MusicAdapter;
use serenity::model::user::User;
use serenity::model::id::{
    UserId,
};
use serenity::client::Context;
use log::*;

#[async_trait]
pub trait UserConv {
    async fn requester_from_user(&self, user: &User) -> Requester;
    async fn muid_from_userid(&self, userid: &UserId) -> MinstrelUserId;
    async fn get_user_from_muid(&self, ctx: &Context, muid: &MinstrelUserId) -> Option<User>;
}

#[async_trait]
impl UserConv for MusicAdapter {
    async fn requester_from_user(&self, user: &User) -> Requester {
        let id = self.muid_from_userid(&user.id).await;
        self.db.get_requester(id).await.unwrap()
    }

    async fn muid_from_userid(&self, userid: &UserId) -> MinstrelUserId {
        self.db.get_userid_from_discordid(userid.0).await.unwrap()
    }

    async fn get_user_from_muid(&self, ctx: &Context, muid: &MinstrelUserId) -> Option<User> {
        let discordid = self.db.get_discordid_from_userid(*muid).await.unwrap();
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
}
