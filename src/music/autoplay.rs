use super::song::Song;
use super::MusicError;

use std::collections::HashMap;
use std::collections::BinaryHeap;
use std::cmp::Ordering;
use rand::Rng;

use youtube_dl::YoutubeDlOutput;

use serenity::{
    model::user::User,
};

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

// TODO: perhaps have passthrough functions to mstate, or maybe just put this all in mstate?
pub struct AutoplayState {
    // TODO: consider just using UserId here for the index?
    // TODO: consider Arc'ing the userlist so AutoplayState can be cloned when prefetching songs
    userlists: HashMap<User, std::boxed::Box<youtube_dl::Playlist>>,
    usertime: BinaryHeap<UserTime>,
    pub enabled: bool,
}

impl AutoplayState {
    pub fn new() -> AutoplayState {
        AutoplayState {
            userlists: HashMap::new(),
            usertime: BinaryHeap::new(),
            enabled: false,
        }
    }

    /// Get the next song to play and increment the play state
    pub fn next(&mut self) -> Option<Song> {
        let mut ut = match self.usertime.pop() {
            Some(ut) => ut,
            None => return None, // No users
        };

        let playlist = match self.userlists.get(&ut.user) {
            Some(p) => p,
            None => panic!("usertime contains user not in userlist"),
        };

        let playlist = playlist.entries.as_ref().unwrap();

        // TODO: implement a separate playlist randomizer logic, especially one that avoids
        //  repeating songs too much
        let mut rng = rand::thread_rng();
        let song = playlist.get(rng.gen_range(0..playlist.len())).unwrap();

        let ret = Song::from_video(song.clone(), &ut.user);

        // This is absolutely required for autoplay to work, just panic if we have problems here
        // TODO: better handle a problematic song on a player's playlist
        let secs = ret.metadata.duration.as_ref().unwrap().as_f64().unwrap() as u64;

        ut.time += secs;
        self.usertime.push(ut);

        Some(ret)
    }

    pub fn register(&mut self, user: &User, url: &String) -> Result<(), MusicError> {
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
            return Err(MusicError::UnknownError);
        }

        // TODO: probably definitely just use time here, this is a lot of clones
        self.userlists.insert(user.clone(), data);
        self.usertime.push(UserTime { user: user.clone(), time: 0 });

        Ok(())
    }

    pub fn debug_get_usertime(&self) -> String {
        format!("{:?}", self.usertime)
    }
}