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
    pub fn new(url: String, requester: &Requester) -> Result<Song, MusicError> {
        if !url.starts_with("http") {
            return Err(MusicError::InvalidUrl);
        }

        let data = YoutubeDl::new(&url)
            .run()
            .map_err(|e| {
                    log::error!("youtube_dl error: {:?}", e);
                    MusicError::FailedToRetrieve
                }
            )?;

        match data {
            YoutubeDlOutput::SingleVideo(v) => {
                let duration = get_duration!(v);
                Ok(Song {
                    url,
                    metadata: v,
                    requested_by: requester.clone(),
                    duration
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
            duration,
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


impl From<Song> for webdata::Song {
    fn from(song: Song) -> Self {
        Self {
            title: song.metadata.title,
            artist: song.metadata.uploader.unwrap_or_else(|| String::from("Unknown")),
            url: song.url,
            thumbnail: song.metadata.thumbnail.unwrap_or(format!("https://img.youtube.com/vi/{}/maxresdefault.jpg", song.metadata.id)),
            duration: song.duration,
            requested_by: song.requested_by.into(),
        }
    }
}