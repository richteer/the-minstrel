use std::collections::HashMap;

use minstrelmodel::MinstrelUserId;
use sqlx::SqlitePool;
use crate::model::*;

pub async fn init_db() -> DbAdapter {
    // TODO: config this path, use sane default
    // TODO: consider db connection options, consider single connection
    let db = SqlitePool::connect("sqlite://minstrel.db?mode=rwc").await.unwrap();

    // TODO: figure out if this is enough
    sqlx::migrate!().run(&db).await.unwrap();

    DbAdapter::new(db)
}

pub type UserId = i64;
pub type DiscordId = String;
pub type SourceId = i64;


#[derive(Clone, Debug)]
pub struct DbAdapter {
    db: SqlitePool,
}

/// Clone-friendly abstracted handle to the storage backend.
/// All return types from this must be types from the model crate.
impl DbAdapter {

    pub fn new(db: SqlitePool) -> Self {
        Self {
            db,
        }
    }

    /// Get all userids and their associated sources
    /// TODO: eventually probably don't use this, this is mostly for autoplay refactoring
    pub async fn get_active_sources(&self) -> Result<HashMap<MinstrelUserId, Vec<minstrelmodel::Source>>, ()> {
        let resp = sqlx::query_as!(Source, r#"SELECT * FROM source WHERE active = "true""#)
            .fetch_all(&self.db).await;

        let resp = resp.unwrap();

        let mut ret: HashMap<i64, Vec<minstrelmodel::Source>> = HashMap::new();
        for row in resp {
            if ret.contains_key(&row.user_id) {
                ret.get_mut(&row.user_id).unwrap().push(row.into());
            } else {
                ret.insert(row.user_id, vec![row.into()]);
            }
        }

        Ok(ret)
    }

    /// Get a model::Requester struct from a MinstrelUserId
    pub async fn get_requester(&self, muid: MinstrelUserId) -> Result<minstrelmodel::Requester, ()> {
        let resp = sqlx::query!("SELECT user_auth.username, user.displayname, user.icon, user.id  FROM user INNER JOIN user_auth ON user.id=user_auth.user_id AND user.id = ?", muid)
            .fetch_one(&self.db).await;

        let resp = resp.unwrap();

        Ok(minstrelmodel::Requester {
            displayname: resp.displayname.unwrap_or_else(|| resp.username.clone()),
            username: resp.username,
            icon: resp.icon.unwrap_or_else(|| "".into()),
            id: resp.id,
        })
    }

    pub async fn get_userid_from_discordid(&self, discordid: u64) -> Result<minstrelmodel::MinstrelUserId, ()> {
        let discordid = discordid as i64;
        let resp = sqlx::query!("SELECT user_id FROM discord_user WHERE discord_id = ?", discordid)
            .fetch_one(&self.db).await;

        let resp = resp.unwrap();

        Ok(resp.user_id)
    }

    pub async fn get_discordid_from_userid(&self, userid: MinstrelUserId) -> Result<Option<u64>, ()> {
        let resp = sqlx::query!("SELECT discord_id FROM discord_user WHERE user_id = ?", userid)
            .fetch_one(&self.db).await;

        let resp = resp.unwrap();

        Ok(Some(resp.discord_id as u64))
    }
}

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn test_models() {
        todo!()
    }
}