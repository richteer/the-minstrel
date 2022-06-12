use super::MusicError;

use youtube_dl::{YoutubeDl, YoutubeDlOutput, SingleVideo};

use model::{
    Requester,
    Song,
    SongRequest,
    SourceType,
};

macro_rules! get_duration {
    ($video:ident) => {
        $video.duration.as_ref().expect("Duration missing from video")
            .as_f64()
            .expect("Could not parse as an f64 for some reason")
            as i64
    };
}

pub fn song_from_video(video: SingleVideo) -> Song {
    let duration = get_duration!(video);
    let thumbnail = video.thumbnail
        .unwrap_or(format!("https://img.youtube.com/vi/{}/maxresdefault.jpg", video.id));
    let url = format!("https://www.youtube.com/watch?v={}", video.url.as_ref().unwrap());

    Song {
        title: video.title,
        artist: video.uploader.unwrap_or_else(|| String::from("Unknown")),
        url,
        thumbnail,
        duration,
    }
}

pub fn fetch_song_from_yt(url: String) -> Result<Song, MusicError> {
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

    let data = match data {
        YoutubeDlOutput::SingleVideo(v) => v,
        YoutubeDlOutput::Playlist(_) => return Err(MusicError::UnknownError),
    };

    Ok(song_from_video(*data))
}


// TODO: deprecate this function when proper loading from sources
/// Create a new song struct from an existing metadata struct
/// Mostly needed only for the autoplay playlist feature
pub fn song_request_from_video(video: SingleVideo, requester: &Requester) -> SongRequest {
    let song = song_from_video(video);

    SongRequest {
        song,
        requested_by: requester.clone(),
    }
}

pub fn fetch_songs_from_source(source: &SourceType) -> Vec<Song> {
    match source {
        SourceType::YoutubePlaylist(url) => {
            let data = youtube_dl::YoutubeDl::new(url)
                .flat_playlist(true)
                .run();

            let data = match data {
                Ok(YoutubeDlOutput::Playlist(p)) => p,
                Ok(YoutubeDlOutput::SingleVideo(_)) => todo!("handle incorrect source mapping somehow"),
                Err(e) => panic!("something broke: {:?}", e),
            };

            if data.entries.is_none() {
                panic!("playlist entries is none, this shouldn't happen");
            }

            let tmpdata = data.entries.unwrap();
            tmpdata.iter()
                            .map(|e| song_from_video(e.clone()))
                            .collect()
        },
    }
}

#[cfg(disabled)]
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


#[cfg(disabled)]
impl From<Song> for model::Song {
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