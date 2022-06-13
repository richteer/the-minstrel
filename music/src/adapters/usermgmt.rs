use db::DbAdapter;
use minstrel_config::read_config;
use model::{
    MinstrelUserId,
    UserMgmtError,
};

use std::time;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use pbkdf2::{
    password_hash::{
        rand_core::OsRng,
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
        Error as PwError,
    },
    Pbkdf2
};

#[derive(Clone, Debug)]
pub enum AuthType {
    UserAuth(String, String),
    Discord(u64),
}

pub struct UserInfo {
    pub displayname: String,
    pub icon: Option<String>,
}

fn hash_password(password: &String) -> Result<String, UserMgmtError> {
    let salt = SaltString::generate(&mut OsRng);

    let pw_hash = Pbkdf2.hash_password(password.as_bytes(), &salt);
    let pw_hash = match pw_hash {
        Ok(h) => h.to_string(),
        Err(e) => {
            log::error!("Failed to hash password, this should probably never happen: {:?}", e);
            return Err(UserMgmtError::UnknownError)
        },
    };

    // Sanity check
    if !verify_password(password, &pw_hash)? {
        log::error!("new password failed to verify against itself, probably a bug!");
        Err(UserMgmtError::UnknownError)
    } else {
        Ok(pw_hash)
    }
}

fn verify_password(input: &String, hash: &String) -> Result<bool, UserMgmtError> {
    let parsed = match PasswordHash::new(&hash) {
        Ok(p) => p,
        Err(e) => {
            log::error!("Failed to verify password, this also should probably never happen: {:?}", e);
            return Err(UserMgmtError::UnknownError)
        }
    };

    match Pbkdf2.verify_password(input.as_bytes(), &parsed) {
        Ok(_) => Ok(true),
        Err(PwError::Password) => Ok(false),
        Err(e) => {
            log::error!("failed to verify password: {:?}", e);
            Err(UserMgmtError::UnknownError)
        }
    }
}


/// Higher level functions for user management.
#[derive(Clone, Debug)]
pub struct UserMgmt {
    db: DbAdapter,
    linktable: Arc<Mutex<HashMap<u64, (MinstrelUserId, time::Instant)>>>,
}

impl UserMgmt {
    pub fn new(db: DbAdapter) -> Self {
        let linktable = Arc::new(Mutex::new(HashMap::new()));

        Self {
            db,
            linktable,
        }
    }

    /// Create a new User and Auth from the specified information
    pub async fn user_create(&self, auth: AuthType, info: UserInfo) -> Result<MinstrelUserId, UserMgmtError> {
        // Check if the user has already registered, error if so
        let exists = match &auth {
            AuthType::UserAuth(username, _) =>
                self.db.exists_user_auth_by_username(username).await,
            AuthType::Discord(did) =>
                self.db.exists_discord_user_by_discord_id(*did).await,
        }.map_err(|_| UserMgmtError::DbError)?;
        if exists {
            return Err(UserMgmtError::UserExists)
        }

        let uid = self.db.create_user(info.displayname, info.icon).await.unwrap();

        let resp = match &auth {
            AuthType::UserAuth(username, password) => {
                // TODO: Encode password here
                let hashed_password = hash_password(password)?;

                self.db.create_user_auth(uid, username, &hashed_password).await
            },
            AuthType::Discord(did) => {
                self.db.create_discord_user(uid, *did).await
            },
        };

        match resp {
            Ok(_) => Ok(uid),
            Err(e) => {
                log::error!("Error attempting to create user {:?}: {:?}", &auth, e);
                log::error!("Attempting to delete partial user, there may be a forthcoming panic");
                self.db.delete_user(uid).await.unwrap();
                Err(UserMgmtError::DbError)
            }
        }
    }

    /// Create a new auth struct that points to an existing User
    ///  Returns the id of the User that has been linked
    pub async fn user_link(&self, link: u64, newauth: AuthType) -> Result<MinstrelUserId, UserMgmtError> {
        let user = {
            let mut links = self.linktable.lock().await;

            let (user, time) = *links.get(&link).ok_or(UserMgmtError::InvalidLink)?;

            let link_timeout = read_config!(user.link_timeout);
            if time.elapsed().as_secs() >= link_timeout {
                links.remove(&link);

                return Err(UserMgmtError::InvalidLink);
            }

            // Link is no longer needed, remove it here.
            // Users will have to regenerate a link if an error occurs later on
            links.remove(&link);

            user
        };

        let exists = self.db.exists_user_by_id(user).await
            .map_err(|_| UserMgmtError::DbError)?;
        if !exists {
            return Err(UserMgmtError::UserDoesNotExist);
        }

        match &newauth {
            AuthType::UserAuth(username, password) => {
                if self.db.exists_user_auth_by_user_id(user).await.map_err(|_| UserMgmtError::DbError)? {
                    return Err(UserMgmtError::UserExists)
                }

                let hashed_password = hash_password(&password)?;
                self.db.create_user_auth(user, username, &hashed_password).await.map_err(|_| UserMgmtError::DbError)?;
            },
            AuthType::Discord(did) => {
                if self.db.exists_discord_user_by_user_id(user).await.map_err(|_| UserMgmtError::DbError)? {
                    return Err(UserMgmtError::UserExists)
                }

                self.db.create_discord_user(user, *did).await.map_err(|_| UserMgmtError::DbError)?;
            },
        };

        Ok(user)
    }

    /// Create a unique link code for different auth methods to refer to the same User
    ///  Returns a u64 that should (eventually) be used in .user_link by a different auth type than
    ///  the one calling this function.
    pub async fn create_link(&self, user_id: MinstrelUserId) -> Result<u64, UserMgmtError> {
        let mut links = self.linktable.lock().await;

        let link_timeout = read_config!(user.link_timeout);

        // First clear out any expired links...
        let mut expired = Vec::new();
        for (link, (_, time)) in links.iter() {
            if time.elapsed().as_secs() >= link_timeout {
                expired.push(*link);
            }
        }
        for e in expired {
            links.remove(&e);
        }

        // ...and now check if there already exists a link for this user...
        for (link, (user, _)) in links.iter() {
            if *user == user_id {
                return Ok(*link)
            }
        }

        // ...there is not, so generate one
        let mut link;

        for _ in 0..5 {
            link = rand::random::<u64>();

            // I fully acknowledge that due to the laws of randomness, this may never terminate.
            if !links.contains_key(&link) {
                links.insert(link, (user_id, time::Instant::now()));

                return Ok(link)
            };
        }

        log::error!("Uhh, might have just generated the same random number 5 times in a row?");
        Err(UserMgmtError::UnknownError)
    }

    /// "Authenticate" a user by username and password.
    ///  Returns Some(user_id) if valid, None if either doesn't exist or bad password.
    ///   Failure reason is intentionally obfuscated
    ///  Takes in a borrowed username, but takes ownership of password to ensure it is dropped.
    ///   password should not be cloned, so it should be dropped from memory.
    ///  TODO: consider making password a special struct that doesn't implement clone or something
    pub async fn user_authenticate(&self, username: &String, password: String) -> Result<Option<MinstrelUserId>, UserMgmtError> {
        let auth = self.db.get_user_auth_by_username(username).await;

        let auth = match auth {
            Ok(a) => a,
            Err(e) => {
                log::error!("Database threw an error: {:?}", e);
                return Err(UserMgmtError::DbError)
            }
        };

        if let Some((id, phash)) = auth {
            match verify_password(&password, &phash)? {
                true => Ok(Some(id)), // Success
                false => Ok(None),    // Bad Password
            }
        } else {
            Ok(None) // Username not found
        }
    }

    // TODO
    /// Merge two User entries with different auth methods into a single User
    ///  Metadata from auth1's user takes precedence over auth2
    ///  auth1 and auth2 CANNOT be the same auth t
    pub async fn user_merge(&self, _user: MinstrelUserId, _auth1: AuthType, _auth2: AuthType) -> Result<MinstrelUserId, UserMgmtError> {
        todo!()
    }
}