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
    pub id: MinstrelUserId, // same as MinstrelId probably
}

pub type MinstrelUserId = i64;

#[derive(Clone, Serialize, Eq, PartialEq, Deserialize, Debug)]
pub struct Song {
    pub title: String,
    pub artist: String,
    pub url: String,
    pub thumbnail: String,
    pub duration: i64,
}

#[derive(Clone, Serialize, Eq, PartialEq, Deserialize, Debug)]
pub struct SongRequest {
    pub song: Song,
    pub requested_by: Requester,
}

impl SongRequest {
    /// Convenience constructor
    pub fn new(song: Song, requested_by: Requester) -> Self {
        Self {
            song,
            requested_by,
        }
    }
}

#[derive(Clone, Serialize, Eq, PartialEq, Deserialize, Debug)]
/// Path to a source of music, to be used in autoplay.
/// May be a playlist, or just a single song.
/// TODO: Support other Source types
pub enum Source {
    YoutubePlaylist(String),
}

#[derive(Clone, Serialize, Eq, PartialEq, Deserialize, Debug)]
pub struct MinstrelWebData {
    pub current_track: Option<SongRequest>,
    pub song_progress: u64,
    pub status: MusicStateStatus,
    pub queue: VecDeque<SongRequest>,
    pub upcoming: Vec<SongRequest>,
    pub history: VecDeque<SongRequest>,
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
impl fmt::Display for SongRequest {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let secs = self.song.duration;
        let mins = secs / 60;
        let secs = secs % 60;

        write!(f, "**{0}** [{1}:{2:02}] _(requested by {3})_",
            self.song.title,
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

    pub fn get_history(&self) -> VecDeque<SongRequest> {
        self.history.clone()
    }

    pub fn current_song(&self) -> Option<SongRequest> {
        self.current_track.clone()
    }

    pub fn is_queue_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, Serialize, Deserialize)]
// TODO: Add other types of partial broadcasts here after MusicState gets broken up
pub enum MinstrelBroadcast {
    MusicState(MinstrelWebData),
    // TODO: This should probably be an enum, so that frontends can display errors as they choose
    Error(String),
}