use minstrel_config::read_config;
use crate::song::*;

use model::{
    Requester,
    SongRequest,
    MinstrelUserId, Source,
};

use db::DbAdapter;

use std::fmt;
use std::collections::HashMap;
use priority_queue::PriorityQueue;
use std::cmp::Reverse;
use rand::seq::SliceRandom;
use log::*;


#[allow(dead_code)]
#[non_exhaustive]
#[derive(Debug)]
pub enum AutoplayOk {
    Status(bool),
    RegisteredUser,
    UpdatedPlaylist,
    EnrolledUser,
    RemovedUser,
    Ok,
}

impl fmt::Display for AutoplayOk {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        #[allow(unreachable_patterns)]
        let ret = match self {
            AutoplayOk::RegisteredUser => "Registered user and playlist for autoplay",
            AutoplayOk::UpdatedPlaylist => "Refreshed playlist, upcoming songs have been shuffled",
            AutoplayOk::EnrolledUser => "Enrolled user for current autoplay",
            AutoplayOk::RemovedUser => "Removed user from current autoplay",
            _ => "Unknown response, fill me in!",
        };

        write!(f, "{}", ret)
    }
}

#[allow(dead_code)]
#[non_exhaustive]
#[derive(Debug)]
pub enum AutoplayError {
    AlreadyEnrolled,
    UserNotEnrolled,
    UrlNotPlaylist,
    UserNotRegistered,
    ExcessiveSize,
    UnknownError,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AutoplayControlCmd {
    Enable,
    Disable,
    Status,
    //Register((Requester, Source)),
    EnableUser(MinstrelUserId),
    DisableUser(MinstrelUserId),
    DisableAllUsers,
    ShuffleUser(MinstrelUserId),
    Rebalance,
    UpdatePlaylist(Requester),
    AdvancePlaylist((MinstrelUserId, u64)),
    BumpPlaylist((MinstrelUserId, usize)),
}


#[derive(Clone, Debug)]
// TODO: consider maybe Song here, and appened to a Request later
struct UserPlaylist {
    index: usize, // For non-destructive randomization, keeping consistent
    list: Vec<SongRequest>,
}

impl UserPlaylist {
    pub fn new(list: Vec<SongRequest>) -> UserPlaylist {
        UserPlaylist {
            index: 0,
            list,
        }
    }

    // TODO: implement a better playlist randomizer logic, especially one that avoids
    //  repeating songs too much
    pub fn next(&mut self) -> SongRequest {
        let ret = self.list.get(self.index);
        self.index += 1;

        let ret = ret.unwrap().clone();

        if self.index >= self.list.len() {
            self.shuffle();
        }

        ret
    }

    /// Re-randomize the user's playlist
    pub fn shuffle(&mut self) {
        let mut rng = rand::thread_rng();

        self.index = 0;
        self.list.shuffle(&mut rng);
    }

    pub fn push_to_end(&mut self, index: usize) -> Result<(), AutoplayError> {
        // Adjust for the internal non-destructive sequencing
        let index = self.index + index;

        if index >= self.list.len() {
            Err(AutoplayError::ExcessiveSize)
        } else {
            let elem = self.list.remove(index);
            self.list.push(elem);

            Ok(())
        }
    }
}


// TODO: perhaps have passthrough functions to mstate, or maybe just put this all in mstate?
#[derive(Clone)]
pub struct AutoplayState {
    // TODO: consider just using UserId here for the index?
    // TODO: consider Arc'ing the userlist so AutoplayState can be cloned when prefetching songs
    userlists: HashMap<MinstrelUserId, UserPlaylist>,
    usertime: PriorityQueue<MinstrelUserId, Reverse<i64>>,
    usertimecache: HashMap<MinstrelUserId, i64>,
    enabled: bool,
    // TODO: make this a global db that all things can access. this is fine for now though.
    db: DbAdapter,
}

// TODO: reconsider the new() constructor here, Default doesn't feel like the right place to load the autoplay.json cache
// TODO: optimize this entire thing to only request data when actually needed. take advantage of everything being cached.
#[allow(clippy::new_without_default)]
impl AutoplayState {
    pub async fn new(db: DbAdapter) -> AutoplayState {

        let users = db.get_active_sources().await.unwrap();

        let mut ret = AutoplayState {
            userlists: HashMap::new(),
            usertime: PriorityQueue::new(),
            usertimecache: HashMap::new(),
            enabled: false,
            db,
        };

        for (reqid, srcs) in users {
            // Panicking here is fine for now, if there's bad data in the json, let that be caught
            let req = ret.db.get_requester(reqid).await.unwrap();

            debug!("loading setlists for user {} from storage", &req.displayname);
            ret.load_sources_for_requester(&req, &srcs).unwrap();

            ret.usertimecache.insert(reqid, 0);
        }

        ret
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get the next song to play and increment the play state
    #[allow(clippy::should_implement_trait)] // TODO: actually make autoplay iterable
    pub fn next(&mut self) -> Option<SongRequest> {
        let ut = match self.usertime.pop() {
            Some(ut) => ut,
            None => return None, // No users
        };
        let (user, Reverse(mut time)) = ut;

        let up = match self.userlists.get_mut(&user) {
            Some(p) => p,
            None => panic!("usertime contains user not in userlist"),
        };

        let song = up.next();

        time += song.song.duration;
        self.usertime.push(user.clone(), Reverse(time));
        self.usertimecache.insert(user, time);

        Some(song)
    }

    pub fn load_sources_for_requester(&mut self, requester: &Requester, sources: &Vec<Source>) -> Result<AutoplayOk, AutoplayError> {
        let mut tmpdata = Vec::new();
        for src in sources {
            let mut tmp = fetch_songs_from_source(&src.path)
                .iter().map(|e| SongRequest::new(e.clone(), requester.clone())).collect();
            tmpdata.append(&mut tmp);
        }

        // If a user has no sources to load (possibly deleted the last one), remove them from the userlists
        if tmpdata.is_empty(){
            self.userlists.remove(&requester.id);

            return Ok(AutoplayOk::RemovedUser)
        }

        let mut tmpdata = UserPlaylist::new(tmpdata);
        tmpdata.shuffle();

        self.userlists.insert(requester.id, tmpdata);

        Ok(AutoplayOk::UpdatedPlaylist)
    }

    pub fn prefetch(&self, num: u64) -> Option<Vec<SongRequest>> {
        let num = if num > read_config!(music.autoplay_prefetch_max) {
            read_config!(music.autoplay_prefetch_max)
        } else {
            num
        };

        let mut ap = self.clone();
        let mut ret = Vec::new();

        for _ in 0..num {
            if let Some(song) = ap.next() {
                ret.push(song);
            }
            else {
                return None; // TODO: return an error here
            }
        }

        Some(ret)
    }



    /// Enable a user that already has a registered setlist in the autoplay system
    /// Sets the user's playtime to the current minimum value
    pub fn enable_user(&mut self, userid: &MinstrelUserId) -> Result<AutoplayOk, AutoplayError> {
        if !self.userlists.contains_key(userid) {
            return Err(AutoplayError::UserNotRegistered);
        }

        let prevtime = if let Some(p) = self.usertimecache.get(userid) {
            p
        } else {
            debug!("Somehow user {} was in userlist but not in usertimecache", userid);
            debug!("defaulting to 0 for their user score, will likely be bumped if lowest");
            &0
        };

        if self.usertime.get(userid).is_some() {
            // user already enabled
            return Err(AutoplayError::AlreadyEnrolled);
        }

        let time = match self.usertime.peek() {
            Some((_, Reverse(lowest))) => {
                debug!("user re-enabling with a cached score of {}, lowest is {}", prevtime, lowest);
                if prevtime >= lowest {
                    *prevtime
                }
                else {
                    lowest - 1
                }
            }
            None => 0,
        };
        debug!("user re-enabled with a score of {}", time);

        self.usertime.push(userid.clone(), Reverse(time));
        self.usertimecache.insert(userid.clone(), time);

        Ok(AutoplayOk::EnrolledUser)
    }

    pub fn disable_user(&mut self, userid: &MinstrelUserId) -> Result<AutoplayOk, AutoplayError> {
        match self.usertime.remove(userid) {
            Some((user, Reverse(time))) => {
                self.usertimecache.insert(user, time);
                Ok(AutoplayOk::RemovedUser)
            },
            None => Err(AutoplayError::UserNotEnrolled),
        }
    }

    /// Remove all users from the PriorityQueue, and set all cached scores to 0.
    pub fn disable_all_users(&mut self) {
        self.usertimecache.iter_mut().for_each(|(_, time)| *time = 0);
        self.usertime.clear();
    }

    /// Reset all usertime scores to zero
    pub fn reset_usertime(&mut self) {
        // TODO: there might be a more efficient way to do this
        self.usertime = self.usertime.clone()
            .into_iter()
            .map(|e| (e.0, Reverse(0)))
            .collect();
        self.usertimecache.iter_mut().for_each(|(_, time)| *time = 0);
    }

    pub fn debug_get_usertime(&self) -> String {
        format!("{:?}", self.usertime)
    }

    pub fn debug_enable_all_users(&mut self) {
        self.disable_all_users();
        let users: Vec<MinstrelUserId> = self.userlists.iter().map(|(u,_)| u.clone()).collect();

        for u in users {
            self.enable_user(&u).unwrap();
        }
    }

    pub fn shuffle_user(&mut self, userid: &MinstrelUserId) -> Result<AutoplayOk, AutoplayError> {
        if let Some(list) = self.userlists.get_mut(userid) {
            list.shuffle();
            // TODO: shuffled ok
            Ok(AutoplayOk::EnrolledUser)
        }
        else {
            Err(AutoplayError::UnknownError)
        }


    }

    pub fn add_time_to_user(&mut self, userid: &MinstrelUserId, delta: i64) {
        self.usertime.change_priority_by(userid, |Reverse(v)| *v += delta);
        let us = self.usertimecache.entry(userid.clone()).or_insert(0);
        *us += delta;
    }

    pub async fn update_userplaylist(&mut self, requester: &Requester) -> Result<AutoplayOk, AutoplayError> {
        let sources = self.db.get_sources_from_userid(requester.id, true).await.unwrap();

        self.load_sources_for_requester(requester, &sources)
    }

    pub fn advance_userplaylist(&mut self, userid: &MinstrelUserId, num: u64) -> Result<AutoplayOk, AutoplayError> {
        if let Some(ul) = self.userlists.get_mut(userid) {
            for _ in 0..num {
                ul.next();
            }

            Ok(AutoplayOk::Ok)
        } else {
            Err(AutoplayError::UserNotRegistered)
        }
    }

    /// Remove a song from a user's upcoming songs
    pub fn bump_userplaylist(&mut self, userid: &MinstrelUserId, index: usize) -> Result<AutoplayOk, AutoplayError> {
        if index > read_config!(music.autoplay_prefetch_max) as usize {
            // TODO: replace with a better error
            return Err(AutoplayError::ExcessiveSize)
        }

        if let Some(ul) = self.userlists.get_mut(userid) {
            ul.push_to_end(index).map_err(|_| AutoplayError::ExcessiveSize)?;
        }

        Ok(AutoplayOk::Ok)
    }
}


