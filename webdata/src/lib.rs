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

// Keeping this one around so that the API is consistent internally,
//  don't want to mess around with using a mix of remote structs and webdata structs
impl Into<crate::Requester> for music::Requester {
    fn into(self) -> crate::Requester {
        crate::Requester {
            username: self.username,
            displayname: self.displayname,
            icon: self.icon,
            id: self.id.0,
        }
    }
}

impl Into<crate::Song> for music::Song {
    fn into(self) -> crate::Song {
        let url = self.metadata.url.unwrap(); // Panic here if this isn't set. It should be.
        crate::Song {
            title: self.metadata.title,
            artist: self.metadata.uploader.unwrap_or(String::from("Unknown")),
            url: url.clone(),
            thumbnail: self.metadata.thumbnail.unwrap_or(format!("https://img.youtube.com/vi/{}/maxresdefault.jpg", self.metadata.id)),
            duration: self.duration,
            requested_by: self.requested_by.into(),
        }
    }
}

// TODO: delete this eventually when types are reconciled
impl Into<crate::MusicStateStatus> for music::MusicStateStatus {
    fn into(self) -> crate::MusicStateStatus {
        match self {
            music::MusicStateStatus::Idle => crate::MusicStateStatus::Idle,
            music::MusicStateStatus::Playing => crate::MusicStateStatus::Playing,
            music::MusicStateStatus::Stopping => crate::MusicStateStatus::Stopping,
            music::MusicStateStatus::Stopped => crate::MusicStateStatus::Stopped,
            _ => todo!("unknown music state status obtained from music crate"),
        }
    }
}