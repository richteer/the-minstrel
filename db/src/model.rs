use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub displayname: String,
    pub icon: String, // Url for now
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserAuth {
    pub id: i64,
    pub userid: i64, // Points to User
    pub username: String,
    pub password: String,
    // TODO: auth token?
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiscordUser {
    pub id: i64,
    pub userid: i64, // Points to User
    // TODO: determine good (de)serialization of discordid
    pub discordid: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Source {
    pub id: i64,
    pub path: String,     // playlist url
    pub active: i64,      // bool, is active or not
    pub source_type: i64, // enum, type of source
    pub user_id: i64,     // Points to User
}

impl From<Source> for minstrelmodel::Source {
    fn from(src: Source) -> Self {
        // TODO: match on row.source_type
        minstrelmodel::Source {
            id: src.id,
            path: minstrelmodel::SourceType::YoutubePlaylist(src.path)
        }
    }
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Song {
    pub id: i64,
    pub path: String,
    pub title: String,
    pub artist: String,
    pub thumbnail_url: Option<String>,
    pub duration: i64,
    pub available: i64, // actually a bool
}

impl From<Song> for minstrelmodel::Song {
    fn from(song: Song) -> Self {
        Self {
            title: song.title,
            artist: song.artist,
            url: song.path,
            thumbnail: song.thumbnail_url.unwrap_or_else(|| "".into()),
            duration: song.duration,
        }
    }
}

// TODO: PlaylistCache, ThumbnailCache