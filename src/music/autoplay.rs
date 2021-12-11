use super::song::Song;
use super::Requester;

use std::fmt;
use std::sync::{RwLock, Arc};
use std::collections::HashMap;
use std::collections::BinaryHeap;
use std::cmp::Ordering;
use rand::seq::SliceRandom;

use pickledb::{PickleDb, PickleDbDumpPolicy};
use youtube_dl::YoutubeDlOutput;

use serenity::{
    model::user::User,
};

#[allow(dead_code)]
#[non_exhaustive]
#[derive(Debug)]
pub enum AutoplayOk {
    RegisteredUser,
    EnrolledUser,
    RemovedUser,
}

impl fmt::Display for AutoplayOk {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        #[allow(unreachable_patterns)]
        let ret = match self {
            AutoplayOk::RegisteredUser => "Registered user and playlist for autoplay",
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
    UnknownError
}


#[derive(Clone, Eq, PartialEq, Debug)]
struct UserTime {
    user: User,
    time: u64,
}

impl Ord for UserTime {
    fn cmp(&self, other: &Self) -> Ordering {
        other.time.cmp(&self.time)
    }
}

impl PartialOrd for UserTime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug)]
struct UserPlaylist {
    index: usize, // For non-destructive randomization, keeping consistent
    list: Vec<Song>,
}

impl UserPlaylist {
    pub fn new(list: Vec<Song>) -> UserPlaylist {
        UserPlaylist {
            index: 0,
            list: list
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
    userlists: HashMap<User, UserPlaylist>,
    usertime: BinaryHeap<UserTime>,
    pub enabled: bool,
    // TODO: make this a global db that all things can access. this is fine for now though.
    storage: Arc<RwLock<PickleDb>>,
}

impl AutoplayState {
    pub fn new() -> AutoplayState {
        // TODO: lock all this storage behind a feature
        let db = match PickleDb::load_json("autoplay.json", PickleDbDumpPolicy::AutoDump) {
            Err(_) => {
                println!("creating new autoplay db");
                PickleDb::new_json("autoplay.json", PickleDbDumpPolicy::AutoDump)
            },
            Ok(d) => d,
        };

        let users: Vec<(Requester, String)> = db.iter().map(|e|
                e.get_value::<(Requester, String)>().unwrap()
            ).collect();

        println!("{:?}", users);

        let mut ret = AutoplayState {
            userlists: HashMap::new(),
            usertime: BinaryHeap::new(),
            enabled: false,
            storage: Arc::new(RwLock::new(db)),
        };

        for (req, url) in users {
            // Panicking here is fine for now, if there's bad date in the json, let that be caught
            println!("loading setlist for user {} from storage", &req.user.name);
            ret.register(req, &url).unwrap();
        }

        ret
    }

    /// Get the next song to play and increment the play state
    pub fn next(&mut self) -> Option<Song> {
        let mut ut = match self.usertime.pop() {
            Some(ut) => ut,
            None => return None, // No users
        };

        let up = match self.userlists.get_mut(&ut.user) {
            Some(p) => p,
            None => panic!("usertime contains user not in userlist"),
        };

        let song = up.next();

        // This is absolutely required for autoplay to work, just panic if we have problems here
        // TODO: better handle a problematic song on a player's playlist
        let secs = song.metadata.duration.as_ref().unwrap().as_f64().unwrap() as u64;

        ut.time += secs;
        self.usertime.push(ut);

        Some(song)
    }

    pub fn register(&mut self, requester: Requester, url: &String) -> Result<AutoplayOk, AutoplayError> {
        {
            if let Ok(mut lock) = self.storage.write() {
                let write = (requester.clone(), url.clone());
                match lock.set(&requester.user.id.to_string(), &write) {
                    Ok(_) => (),
                    Err(e) => println!("Error writing to autoplay storage: {:?}", e),
                    // Continue on failure, storage isn't important
                }
            }
        }

        let data = youtube_dl::YoutubeDl::new(url)
            .flat_playlist(true)
            .run();

        let data = match data {
            Ok(YoutubeDlOutput::SingleVideo(_)) => {
                //check_msg(msg.channel_id.say(&ctx.http, "Must provide link to a playlist, not a single video").await);
                todo!();
                //return Ok(()); // Not ok
            }
            Err(_e) => {
                todo!();
                //check_msg(msg.channel_id.say(&ctx.http, format!("Error fetching playlist: {:?}", e)).await);
                //return Ok(()); // Not ok
            }
            Ok(YoutubeDlOutput::Playlist(p)) => p,
        };

        if data.entries.is_none() {
            println!("user playlist is none");
            return Err(AutoplayError::UnknownError);
        }

        let tmpdata = data.entries.unwrap();
        let tmpdata = tmpdata.iter()
                        .map(|e| Song::from_video(e.clone(), &requester))
                        .collect();

        let mut tmpdata = UserPlaylist::new(tmpdata);
        tmpdata.shuffle();

        // TODO: probably definitely just use UserId here, this is a lot of clones
        self.userlists.insert(requester.user.clone(), tmpdata);
        self.usertime.push(UserTime { user: requester.user.clone(), time: 0 });

        Ok(AutoplayOk::RegisteredUser)
    }

    fn prefetch(&self, num: u64) -> Option<Vec<Song>> {
        // TODO: Config this, also probably return an error here
        if num > 10 {
            return None;
        }

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

    pub fn show_upcoming(&self, num: u64) -> String {
        let mut ret = String::from("Upcoming Autoplay songs:\n");

        let songs = self.prefetch(num).unwrap();

        for (i,v) in songs.iter().enumerate() {
            ret += &format!("{}: {}\n", i+1, &v).to_owned();
        }

        ret
    }

    /// Enable a user that already has a registered setlist in the autoplay system
    /// Sets the user's playtime to the current minimum value
    // TODO: implement an autoplay equiv to MusicOk/MusicError
    pub fn enable_user(&mut self, user: &User) -> Result<AutoplayOk, AutoplayError> {
        if !self.userlists.contains_key(user) {
            return Err(AutoplayError::UnknownError);
        }

        if self.usertime.iter()
            .fold(false, |acc, u| acc || (u.user.id == user.id)) {
            // user already enabled
            return Err(AutoplayError::AlreadyEnrolled);
        }

        let time = match self.usertime.peek() {
            Some(tmp) => tmp.time,
            None => 0,
        };

        self.usertime.push(UserTime { user: user.clone(), time: time });

        Ok(AutoplayOk::EnrolledUser)
    }

    pub fn disable_user(&mut self, user: &User) -> Result<AutoplayOk, AutoplayError> {
        let len = self.usertime.len();
        self.usertime = self.usertime.clone()
            .into_iter()
            .filter(|u| u.user.id != user.id)
            .collect();

        if len == self.usertime.len() {
            Err(AutoplayError::UserNotEnrolled)
        }
        else {
            Ok(AutoplayOk::RemovedUser)
        }
    }

    /// Reset all usertime scores to zero
    pub fn reset_usertime(&mut self) {
        self.usertime = self.usertime.clone()
            .into_iter()
            .map(|mut e| { e.time = 0; e })
            .collect();
    }

    pub fn debug_get_usertime(&self) -> String {
        format!("{:?}", self.usertime)
    }
}