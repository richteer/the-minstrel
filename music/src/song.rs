use super::MusicError;
use super::Requester;
use serde::Serialize;

use std::fmt;

use youtube_dl::{YoutubeDl, YoutubeDlOutput, SingleVideo};

#[derive(Clone, Debug, Serialize)]
pub struct Song {
    pub url: String,
    // TODO: should metadata actually be an Option, or should this be mandatory for a song?
    pub metadata: Box<SingleVideo>,
    pub requested_by: Requester,
    pub duration: i64,
}

macro_rules! get_duration {
    ($video:ident) => {
        $video.duration.as_ref().expect("Duration missing from video")
            .as_f64()
            .expect("Could not parse as an f64 for some reason")
            as i64
    };
}

impl Song {
    /// Create a new song struct from a url and fetch the metadata via ytdl
    pub fn new(url: String, requester: Requester) -> Result<Song, MusicError> {
        if !url.starts_with("http") {
            return Err(MusicError::InvalidUrl);
        }

        let data = YoutubeDl::new(&url)
            .run()
            .map_err(|e|
                match e {
                    // TODO: Probably actually handle the cases here
                    _ => MusicError::FailedToRetrieve,
                }
            )?;

        match data {
            YoutubeDlOutput::SingleVideo(v) => {
                let duration = get_duration!(v);
                Ok(Song {
                    url: url,
                    metadata: v,
                    requested_by: requester.clone(),
                    duration: duration
                })
            },
            YoutubeDlOutput::Playlist(_) => Err(MusicError::UnknownError),
        }
    }

    /// Create a new song struct from an existing metadata struct
    /// Mostly needed only for the autoplay playlist feature
    pub fn from_video(video: SingleVideo, requester: &Requester) -> Song {
        let duration = get_duration!(video);
        Song {
            url: format!("https://www.youtube.com/watch?v={}", video.url.as_ref().unwrap()),
            metadata: Box::new(video),
            requested_by: requester.clone(),
            duration: duration,
        }
    }
}

impl fmt::Display for Song {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let secs = self.duration;
        let mins = secs / 60;
        let secs = secs % 60;

        write!(f, "**{0}** [{1}:{2:02}] _(requested by {3})_",
            self.metadata.title,
            mins, secs,
            &self.requested_by.displayname,
        )
    }
}



impl Into<webdata::Song> for Song {
    fn into(self) -> webdata::Song {
        let url = self.metadata.url.unwrap(); // Panic here if this isn't set. It should be.
        webdata::Song {
            title: self.metadata.title,
            artist: self.metadata.uploader.unwrap_or(String::from("Unknown")),
            url: url.clone(),
            thumbnail: self.metadata.thumbnail.unwrap_or(format!("https://img.youtube.com/vi/{}/maxresdefault.jpg", self.metadata.id)),
            duration: self.duration,
            requested_by: self.requested_by.into(),
        }
    }
}