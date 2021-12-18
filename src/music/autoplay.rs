use super::song::Song;
use super::Requester;

use std::fmt;
use std::sync::{RwLock, Arc};
use std::collections::HashMap;
use priority_queue::PriorityQueue;
use std::cmp::Ordering;
use rand::seq::SliceRandom;
use log::*;

use pickledb::{PickleDb, PickleDbDumpPolicy};
use youtube_dl::YoutubeDlOutput;

use serenity::{
    model::user::User,
    prelude::*,
    model::id::GuildId,
    model::voice::VoiceState,
};

// TODO: perhaps get_mstate_should live higher?
use crate::music;
use crate::get_mstate;

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
    UserNotRegistered,
    UnknownError,
}


#[derive(Clone, Eq, PartialEq, Debug)]
struct UserTime {
    user: User,
    time: i64,
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
    usertime: PriorityQueue<User, i64>,
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
            enabled: false,
            storage: Arc::new(RwLock::new(db)),
        };

        for (req, url) in users {
            // Panicking here is fine for now, if there's bad date in the json, let that be caught
            info!("loading setlist for user {} from storage", &req.user.name);
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
        let (user, mut time) = ut;

        let up = match self.userlists.get_mut(&user) {
            Some(p) => p,
            None => panic!("usertime contains user not in userlist"),
        };

        let song = up.next();

        time += song.duration;
        self.usertime.push(user, time);

        Some(song)
    }

    pub fn register(&mut self, requester: Requester, url: &String) -> Result<AutoplayOk, AutoplayError> {
        {
            if let Ok(mut lock) = self.storage.write() {
                let write = (requester.clone(), url.clone());
                match lock.set(&requester.user.id.to_string(), &write) {
                    Ok(_) => (),
                    Err(e) => error!("Error writing to autoplay storage: {:?}", e),
                    // Continue on failure, storage isn't important
                }
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

        let mut tmpdata = UserPlaylist::new(tmpdata);
        tmpdata.shuffle();

        // TODO: probably definitely just use UserId here, this is a lot of clones
        self.userlists.insert(requester.user.clone(), tmpdata);
        self.usertime.push(requester.user.clone(), 0);

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
        let songs = self.prefetch(num);
        if songs.is_none() {
            return String::from("No users enrolled in Autoplay\n");
        }
        let songs = songs.unwrap();

        let mut ret = String::from("Upcoming Autoplay songs:\n");

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
            return Err(AutoplayError::UserNotRegistered);
        }

        if self.usertime.get(user).is_some() {
            // user already enabled
            return Err(AutoplayError::AlreadyEnrolled);
        }

        let time = match self.usertime.peek() {
            Some(tmp) => tmp.1 - 1,
            None => 0,
        };

        self.usertime.push(user.clone(), time);

        Ok(AutoplayOk::EnrolledUser)
    }

    pub fn disable_user(&mut self, user: &User) -> Result<AutoplayOk, AutoplayError> {
        match self.usertime.remove(user) {
            Some(_) => Ok(AutoplayOk::RemovedUser),
            None => Err(AutoplayError::UserNotEnrolled),
        }
    }

    pub fn disable_all_users(&mut self) {
        self.usertime.clear();
    }

    /// Reset all usertime scores to zero
    pub fn reset_usertime(&mut self) {
        // TODO: there might be a more efficient way to do this
        self.usertime = self.usertime.clone()
            .into_iter()
            .map(|e| (e.0, 0))
            .collect();
    }

    pub fn debug_get_usertime(&self) -> String {
        format!("{:?}", self.usertime)
    }

    pub fn shuffle_user(&mut self, user: &User) -> Result<AutoplayOk, AutoplayError> {
        if let Some(list) = self.userlists.get(user) {
            let mut list = list.clone();
            list.shuffle();
            self.userlists.insert(user.clone(), list);
            // TODO: shuffled ok
            Ok(AutoplayOk::EnrolledUser)
        }
        else {
            Err(AutoplayError::UnknownError)
        }


    }

    pub fn add_time_to_user(&mut self, user: &User, delta: i64) {
        self.usertime.change_priority_by(user, |v| *v += delta);
    }
}


pub async fn autoplay_voice_state_update(ctx: Context, guildid: Option<GuildId>, old: Option<VoiceState>, new: VoiceState) {
    let bot = ctx.cache.current_user_id().await;
    let guild = ctx.cache.guild(guildid.unwrap()).await.unwrap(); // TODO: don't unwrap here, play nice
    let bot_voice = guild.voice_states.get(&bot);

    if bot_voice.is_none() {
        debug!("bot is not in voice, ignoring voice state change");
        return;
    }

    get_mstate!(mut, mstate, ctx);
    if !mstate.autoplay.enabled {
        debug!("autoplay is not enabled, ignoring voice state change");
        return;
    }

    let bot_voice = bot_voice.unwrap();
    let bot_chan = bot_voice.channel_id.unwrap();

    if new.member.as_ref().unwrap().user.id == bot {
        if let Some(chan) = new.channel_id {
            mstate.autoplay.disable_all_users();

            for (uid, vs) in guild.voice_states.iter() {
                if *uid == bot || vs.channel_id.unwrap() != chan {
                    continue;
                }
                // Use the cache lookup based on key, because voicestate.member may be None.
                // Find a non-async way instead if possible
                let user = ctx.cache.user(uid).await.unwrap();
                match mstate.autoplay.enable_user(&user) {
                    Ok(o) => debug!("enrolling user {}: {:?}", user.tag(), o),
                    Err(e) => debug!("did not enroll user {}: {:?}", user.tag(), e),
                };
            }
        }

        return;
    }

    // Connect-to-voice check, enroll if in correct channel
    if let Some(chan) = new.channel_id {
        if chan == bot_chan {
            let user = new.member.unwrap().user;
            match mstate.autoplay.enable_user(&user) {
                Ok(o) => debug!("enrolling user {}: {:?}", user.tag(), o),
                Err(e) => debug!("did not enroll {}: {:?}", user.tag(), e)
            }
            return;
        }
        else {
            debug!("received voice connect for another channel, trying disconnect checks");
        }
    }

    // Disconnect from voice checks, unenroll if old voice state matches bot's channel
    if old.is_none() {
        debug!("join received for another channel, ignoring!");
        return;
    }

    let old_vs = old.unwrap();
    if old_vs.channel_id.is_none() {
        warn!("not sure, apparently no old state channel, but also no new state channel?");
        return;
    }
    let chan = old_vs.channel_id.unwrap();

    if chan == bot_chan {
        let user = new.member.unwrap().user;
        match mstate.autoplay.disable_user(&user) {
            Ok(o) => debug!("enrolling user {}: {:?}", user.tag(), o),
            Err(e) => debug!("did not enroll {}: {:?}", user.tag(), e)
        }
    }
    else {
        debug!("received voice disconnect for another channel, ignorning");
    }

    return;
}
