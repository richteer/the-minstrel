use minstrel_config::read_config;
use super::{
    song::Song,
    requester::MinstrelUserId,
    requester::Requester,
};

use std::fmt;
use std::sync::{RwLock, Arc};
use std::collections::HashMap;
use priority_queue::PriorityQueue;
use std::cmp::Reverse;
use rand::seq::SliceRandom;
use log::*;

use pickledb::{PickleDb, PickleDbDumpPolicy};
use youtube_dl::YoutubeDlOutput;


#[allow(dead_code)]
#[non_exhaustive]
#[derive(Debug)]
pub enum AutoplayOk {
    RegisteredUser,
    UpdatedPlaylist,
    EnrolledUser,
    RemovedUser,
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
    UnknownError,
}


#[derive(Clone, Debug)]
struct UserPlaylist {
    index: usize, // For non-destructive randomization, keeping consistent
    list: Vec<Song>,
    url: String, // For refetching purposes
}

impl UserPlaylist {
    pub fn new(list: Vec<Song>, url: String) -> UserPlaylist {
        UserPlaylist {
            index: 0,
            list: list,
            url: url,
        }
    }

    // TODO: implement a better playlist randomizer logic, especially one that avoids
    //  repeating songs too much
    pub fn next(&mut self) -> Song {
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
}


// TODO: perhaps have passthrough functions to mstate, or maybe just put this all in mstate?
#[derive(Clone)]
pub struct AutoplayState {
    // TODO: consider just using UserId here for the index?
    // TODO: consider Arc'ing the userlist so AutoplayState can be cloned when prefetching songs
    userlists: HashMap<MinstrelUserId, UserPlaylist>,
    usertime: PriorityQueue<MinstrelUserId, Reverse<i64>>,
    usertimecache: HashMap<MinstrelUserId, i64>,
    pub enabled: bool,
    // TODO: make this a global db that all things can access. this is fine for now though.
    storage: Arc<RwLock<PickleDb>>,
}

impl AutoplayState {
    pub fn new() -> AutoplayState {
        // TODO: lock all this storage behind a feature
        let db = match PickleDb::load_json("autoplay.json", PickleDbDumpPolicy::AutoDump) {
            Err(_) => {
                info!("creating new autoplay db");
                PickleDb::new_json("autoplay.json", PickleDbDumpPolicy::AutoDump)
            },
            Ok(d) => d,
        };

        let users: Vec<(Requester, String)> = db.iter().map(|e|
                e.get_value::<(Requester, String)>().unwrap()
            ).collect();

        let mut ret = AutoplayState {
            userlists: HashMap::new(),
            usertime: PriorityQueue::new(),
            usertimecache: HashMap::new(),
            enabled: false,
            storage: Arc::new(RwLock::new(db)),
        };

        for (req, url) in users {
            // Panicking here is fine for now, if there's bad data in the json, let that be caught
            info!("loading setlist for user {} from storage", &req.user.get_name());
            ret.register(req, &url).unwrap();
        }

        ret
    }

    /// Get the next song to play and increment the play state
    pub fn next(&mut self) -> Option<Song> {
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

        time += song.duration;
        self.usertime.push(user.clone(), Reverse(time));
        self.usertimecache.insert(user, time);

        Some(song)
    }

    pub fn register(&mut self, requester: Requester, url: &String) -> Result<AutoplayOk, AutoplayError> {
        {
            if let Ok(mut lock) = self.storage.write() {
                match lock.set(&requester.id.0, &(&requester, url)) {
                    Ok(_) => (),
                    Err(e) => error!("Error writing to autoplay storage: {:?}", e),
                    // Continue on failure, storage isn't important
                }
            }
            else {
                error!("Failed to obtain lock on autoplay storage");
            }
        }

        let data = youtube_dl::YoutubeDl::new(url)
            .flat_playlist(true)
            .run();

        let data = match data {
            Ok(YoutubeDlOutput::Playlist(p)) => p,
            Ok(YoutubeDlOutput::SingleVideo(_)) => return Err(AutoplayError::UrlNotPlaylist),
            Err(e) => panic!("something broke: {:?}", e),
        };

        if data.entries.is_none() {
            error!("user playlist is none");
            return Err(AutoplayError::UnknownError);
        }

        let tmpdata = data.entries.unwrap();
        let tmpdata = tmpdata.iter()
                        .map(|e| Song::from_video(e.clone(), &requester))
                        .collect();

        let mut tmpdata = UserPlaylist::new(tmpdata, url.clone());
        tmpdata.shuffle();

        // TODO: probably definitely just use UserId here, this is a lot of clones
        self.userlists.insert(requester.id.clone(), tmpdata);
        self.usertime.push(requester.id.clone(), Reverse(0));
        self.usertimecache.insert(requester.id.clone(), 0);

        Ok(AutoplayOk::RegisteredUser)
    }

    pub fn prefetch(&self, num: u64) -> Option<Vec<Song>> {
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
            // TODO: maybe just set a default value here?
            error!("Somehow user was in userlist but not in usertimecache: {:?}", userid);
            return Err(AutoplayError::UnknownError);
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

    pub fn shuffle_user(&mut self, userid: &MinstrelUserId) -> Result<AutoplayOk, AutoplayError> {
        if let Some(list) = self.userlists.get(userid) {
            let mut list = list.clone();
            list.shuffle();
            self.userlists.insert(userid.clone(), list);
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

    pub fn update_userplaylist(&mut self, requester: &Requester) -> Result<AutoplayOk, AutoplayError> {

        let url = if let Some(ul) = &self.userlists.get(&requester.id) {
            ul.url.clone()
        }
        else {
            return Err(AutoplayError::UserNotRegistered);
        };

        match self.register(requester.clone(), &url) {
            Ok(AutoplayOk::RegisteredUser) => Ok(AutoplayOk::UpdatedPlaylist),
            Ok(o) => panic!("unknown ok response from register trying to update: {:?}", o),
            Err(e) => Err(e)
        }
    }

    pub fn advance_userplaylist(&mut self, userid: &MinstrelUserId, num: u64) -> Result<AutoplayOk, AutoplayError> {
        if let Some(ul) = self.userlists.get_mut(userid) {
            for _ in 0..num {
                ul.next();
            }

            // TODO: probably define a generic OK?
            Ok(AutoplayOk::UpdatedPlaylist)
        } else {
            Err(AutoplayError::UserNotRegistered)
        }
    }
}


