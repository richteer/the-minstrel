use super::MusicError;

use std::fmt;

use youtube_dl::{YoutubeDl, YoutubeDlOutput, SingleVideo};

use serenity::{
    model::user::User,
};

#[derive(Clone, Debug)]
pub struct Song {
    pub url: String,
    // TODO: should metadata actually be an Option, or should this be mandatory for a song?
    pub metadata: Box<SingleVideo>,
    pub requested_by: User,
}

impl Song {
    /// Create a new song struct from a url and fetch the metadata via ytdl
    pub fn new(url: String, requester: &User) -> Result<Song, MusicError> {
        if !url.starts_with("http") {
            return Err(MusicError::InvalidUrl);
        }

        let data = YoutubeDl::new(&url)
            .run()
            .unwrap();

        match data {
            YoutubeDlOutput::SingleVideo(v) => Ok(Song { url: url, metadata: v, requested_by: requester.clone() }),
            YoutubeDlOutput::Playlist(_) => Err(MusicError::UnknownError),
        }
    }

    /// Create a new song struct from an existing metadata struct
    /// Mostly needed only for the autoplay playlist feature
    pub fn from_video(video: SingleVideo, requester: &User) -> Song {
        Song {
            url: format!("https://www.youtube.com/watch?v={}", video.url.as_ref().unwrap()),
            metadata: Box::new(video),
            requested_by: requester.clone(),
        }
    }
}

impl fmt::Display for Song {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let md = self.metadata.as_ref();

        // TODO: clean up the unwraps here, fail gracefully if for some reason
        //  there is no duration, or it cannot parse into f64
        let secs = md.duration.as_ref().unwrap().as_f64().unwrap();
        let mins = (secs / 60f64) as i64;
        let secs = secs as i64 % 60;

        write!(f, "**{0}** [{1}:{2:02}] _(requested by {3})_",
            md.title,
            mins, secs,
            self.requested_by.tag(), // TODO: use server nick here, may need context...
        )
    }
}
