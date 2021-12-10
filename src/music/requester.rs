use serenity::model::user::User;
use serenity::model::channel::Message;
use serenity::prelude::*;

/// Struct to hold requested-by information for MusicState and friends
/// Fill with anything that is not in the User struct that might be useful
#[derive(Clone, Debug)]
pub struct Requester {
    pub user: User,
    // User nickname in server or name without discriminator
    pub name: String,
}

impl Requester {
    pub async fn from_msg(ctx: &Context, msg: &Message) -> Requester {
        // TODO: Perhaps macro this and let this async nonsense be done in the calling command?
        let name = msg.author
            .nick_in(&ctx.http, msg.guild_id.unwrap())
            .await
            .unwrap_or(msg.author.name.clone());

        Requester {
            name: name,
            user: msg.author.clone(),
        }
    }
}