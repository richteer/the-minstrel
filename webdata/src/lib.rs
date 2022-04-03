use serde::{
    Serialize,
    Deserialize,
};

use std::collections::VecDeque;

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



// TODO: Remove this eventually. Copy-pasted from MusicState
#[non_exhaustive]
#[derive(Clone, Serialize, Eq, PartialEq, Deserialize, Debug)]
pub enum MusicStateStatus {
    Playing,
    Stopping,
    Stopped,
    Idle,
}

// Foreign impls to make conversion easier for web APIs



