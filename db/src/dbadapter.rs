use std::collections::{
    HashMap,
    hash_map::Entry,
};

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
    pub db: SqlitePool,
}

/// Clone-friendly abstracted handle to the storage backend.
/// All return types from this must be types from the model crate.
///
/// All functions should use a prefix corresponding to its CRUD behavior like so:
///  - create_ -> Create new entries
///  - get_ -> Read entries
///  - update_ -> Update entries
///  - delete -> Delete entries
impl DbAdapter {

    pub fn new(db: SqlitePool) -> Self {
        Self {
            db,
        }
    }

    /// Get all userids and their associated sources
    /// TODO: eventually probably don't use this, this is mostly for autoplay refactoring
    pub async fn get_active_sources(&self) -> Result<HashMap<MinstrelUserId, Vec<minstrelmodel::Source>>, ()> {
        let resp = sqlx::query_as!(Source, r#"SELECT * FROM source WHERE active = TRUE"#)
            .fetch_all(&self.db).await;

        let resp = resp.unwrap();

        let mut ret: HashMap<i64, Vec<minstrelmodel::Source>> = HashMap::new();
        for row in resp {
            match ret.entry(row.user_id) {
                Entry::Occupied(mut e) => { e.get_mut().push(row.into()); },
                Entry::Vacant(e)   => { e.insert(vec![row.into()]); },
            }
        }

        Ok(ret)
    }

    pub async fn get_sources_from_userid(&self, user_id: MinstrelUserId, active: bool) -> Result<Vec<minstrelmodel::Source>, ()> {
        let resp = match active {
            true => sqlx::query_as!(Source, r#"SELECT * FROM source WHERE active = TRUE AND user_id = ?"#, user_id)
                .fetch_all(&self.db).await,
            false => sqlx::query_as!(Source, "SELECT * FROM source WHERE user_id = ?", user_id)
                .fetch_all(&self.db).await,
        };

        let mut resp = resp.unwrap();
        let resp = resp.drain(..).map(|e| e.into()).collect();

        Ok(resp)
    }

    pub async fn create_source(&self, user_id: MinstrelUserId, srctype: &minstrelmodel::SourceType, active: bool) -> Result<(),()> {
        let (path, srctype) = match srctype {
            minstrelmodel::SourceType::YoutubePlaylist(path) => (path, 1), // TODO: actually implement a source enum
        };

        let resp = sqlx::query!("INSERT INTO source (path, active, source_type, user_id) VALUES (?, ?, ?, ?)",
            path, active, srctype, user_id).execute(&self.db).await;

        match resp {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }

    pub async fn delete_source(&self, source_id: SourceId) -> Result<bool, ()> {
        let resp = sqlx::query!("DELETE FROM source WHERE id = ? RETURNING id", source_id)
            .fetch_optional(&self.db).await;

        match resp {
            Ok(Some(_)) => Ok(true),
            Ok(None) => Ok(false),
            Err(_) => Err(()),
        }
    }

    /// Get a model::Requester struct from a MinstrelUserId
    pub async fn get_requester(&self, muid: MinstrelUserId) -> Result<minstrelmodel::Requester, ()> {
        let resp = sqlx::query!("SELECT * FROM user WHERE user.id = ?", muid)
            .fetch_one(&self.db).await;

        let resp = resp.unwrap();

        Ok(minstrelmodel::Requester {
            displayname: resp.displayname,
            icon: resp.icon.unwrap_or_else(|| "".into()),
            id: resp.id,
        })
    }

    pub async fn get_userid_from_discordid(&self, discordid: u64) -> Result<Option<minstrelmodel::MinstrelUserId>, ()> {
        let discordid = discordid as i64;
        let resp = sqlx::query!("SELECT user_id FROM discord_user WHERE discord_id = ?", discordid)
            .fetch_optional(&self.db).await;

        let resp = resp.unwrap();
        if let Some(row) = resp {
            Ok(Some(row.user_id))
        } else {
            Ok(None)
        }
    }

    pub async fn get_discordid_from_userid(&self, userid: MinstrelUserId) -> Result<Option<u64>, ()> {
        let resp = sqlx::query!("SELECT discord_id FROM discord_user WHERE user_id = ?", userid)
            .fetch_one(&self.db).await;

        let resp = resp.unwrap();

        Ok(Some(resp.discord_id as u64))
    }

    pub async fn create_user(&self, displayname: String, icon: Option<String>) -> Result<MinstrelUserId, ()> {

        let id = match icon {
            Some(icon) =>
                sqlx::query!("INSERT INTO user (displayname, icon) VALUES (?, ?) RETURNING id", displayname, icon)
                    .fetch_one(&self.db).await.unwrap().id,
            None =>
                sqlx::query!("INSERT INTO user (displayname) VALUES (?) RETURNING id", displayname)
                    .fetch_one(&self.db).await.unwrap().id,
        };

        Ok(id)
    }

    pub async fn delete_user(&self, user_id: MinstrelUserId) -> Result<Option<MinstrelUserId>, ()> {
        let resp = sqlx::query!("DELETE FROM user WHERE id = ?", user_id)
            .execute(&self.db).await;

        match resp {
            Ok(_) => Ok(Some(user_id)),
            // TODO: handle the actual db errors, like connection/etc
            Err(_) => Ok(None),
        }
    }

    pub async fn create_user_auth(&self, user_id: MinstrelUserId, username: &String, password: &String) -> Result<(), ()> {
        sqlx::query!("INSERT INTO user_auth (username, password, user_id) VALUES (?, ?, ?)",
             username, password, user_id)
            .execute(&self.db).await.unwrap();

        Ok(())
    }

    pub async fn get_user_auth_by_username(&self, username: &String) -> Result<Option<(MinstrelUserId, String)>, ()> {
        let resp = sqlx::query_as!(UserAuth, "SELECT * FROM user_auth WHERE username = ?", username)
            .fetch_optional(&self.db).await;

        let resp = resp.unwrap();

        if let Some(resp) = resp {
            Ok(Some((resp.user_id, resp.password)))
        } else {
            Ok(None)
        }
    }

    pub async fn create_discord_user(&self, user_id: MinstrelUserId, discord_id: u64) -> Result<(), ()> {
        let discord_id = discord_id as i64;
        sqlx::query!("INSERT INTO discord_user (user_id, discord_id) VALUES (?, ?)",
             user_id, discord_id).execute(&self.db).await.unwrap();

        Ok(())
    }

    pub async fn exists_user_by_id(&self, user_id: MinstrelUserId) -> Result<bool, ()> {
        let resp = sqlx::query!("SELECT id FROM user WHERE id = ?", user_id)
        .fetch_optional(&self.db).await;

        match resp {
            Ok(Some(_)) => Ok(true),
            Ok(None) => Ok(false),
            Err(_) => Err(()),
        }
    }

    pub async fn exists_user_auth_by_user_id(&self, user_id: MinstrelUserId) -> Result<bool, ()> {
        let resp = sqlx::query!("SELECT id FROM user_auth WHERE user_id = ?", user_id)
            .fetch_optional(&self.db).await;

        match resp {
            Ok(Some(_)) => Ok(true),
            Ok(None) => Ok(false),
            Err(_) => Err(()),
        }
    }

    pub async fn exists_user_auth_by_username(&self, username: &String) -> Result<bool, ()> {
        let resp = sqlx::query!("SELECT id FROM user_auth WHERE username = ?", username)
        .fetch_optional(&self.db).await;

        match resp {
            Ok(Some(_)) => Ok(true),
            Ok(None) => Ok(false),
            Err(_) => Err(()),
        }
    }

    pub async fn exists_discord_user_by_user_id(&self, user_id: MinstrelUserId) -> Result<bool, ()> {
        let resp = sqlx::query!("SELECT id FROM discord_user WHERE user_id = ?", user_id)
        .fetch_optional(&self.db).await;

        match resp {
            Ok(Some(_)) => Ok(true),
            Ok(None) => Ok(false),
            Err(_) => Err(()),
        }
    }

    pub async fn exists_discord_user_by_discord_id(&self, discord_id: u64) -> Result<bool, ()> {
        let discord_id = discord_id as i64;
        let resp = sqlx::query!("SELECT id FROM discord_user WHERE discord_id = ?", discord_id)
        .fetch_optional(&self.db).await;

        match resp {
            Ok(Some(_)) => Ok(true),
            Ok(None) => Ok(false),
            Err(_) => Err(()),
        }
    }
 }
