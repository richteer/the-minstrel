use serde::{
    Serialize,
    Deserialize,
};

use std::collections::VecDeque;
use std::fmt;

pub mod web;

// Literal copy of what is in music::Requester
//  Subject to deletion if/when all the structs in music:: become "web compatible"
#[derive(Clone, Serialize, Eq, PartialEq, Deserialize, Debug)]
pub struct Requester {
    pub username: String,
    pub displayname: String,
    pub icon: String, // url
    pub id: String, // same as MinstrelId probably
}

#[derive(Clone, Serialize, Eq, PartialEq, Deserialize, Debug)]
pub struct Song {
    pub title: String,
    pub artist: String,
    pub url: String,
    pub thumbnail: String,
    pub duration: i64,
    pub requested_by: Requester,
}

#[derive(Clone, Serialize, Eq, PartialEq, Deserialize, Debug)]
pub struct MinstrelWebData {
    pub current_track: Option<Song>,
    pub status: MusicStateStatus,
    pub queue: VecDeque<Song>,
    pub upcoming: Vec<Song>,
    pub history: VecDeque<Song>,
}



#[non_exhaustive]
#[derive(Clone, Serialize, Eq, PartialEq, Deserialize, Debug)]
pub enum MusicStateStatus {
    Playing,
    Stopping,
    Stopped,
    Idle,
}


// TODO: Probably don't depend on this. Force frontends to format it themselves
impl fmt::Display for Song {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let secs = self.duration;
        let mins = secs / 60;
        let secs = secs % 60;

        write!(f, "**{0}** [{1}:{2:02}] _(requested by {3})_",
            self.title,
            mins, secs,
            &self.requested_by.displayname,
        )
    }
}

impl MinstrelWebData {
    /// Get a display string for the queue
    pub fn show_queue(&self) -> String {
        let mut ret = String::from("Current play queue:\n");

        for (i,v) in self.queue.iter().enumerate() {
            ret += &format!("{}: {}\n", i+1, &v).to_owned();
        }

        ret
    }

    pub fn get_history(&self) -> VecDeque<Song> {
        self.history.clone()
    }

    pub fn current_song(&self) -> Option<Song> {
        self.current_track.clone()
    }

    pub fn is_queue_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
// TODO: Add other types of partial broadcasts here after MusicState gets broken up
pub enum MinstrelBroadcast {
    MusicState(MinstrelWebData),
    // TODO: This should probably be an enum, so that frontends can display errors as they choose
    Error(String),
}