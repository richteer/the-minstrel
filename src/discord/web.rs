use serenity::Client;
use std::convert::Infallible;
use warp::Filter;
use crate::discord::client::MusicStateKey;

use std::sync::Arc;
use tokio::sync::Mutex;

use crate::music::MusicState;
use crate::music::MusicStateStatus;
use crate::music::Song;
use crate::music::requester::*;
use crate::discord::player::DiscordPlayer;

async fn show_state(
    mstate: Arc<Mutex<MusicState<DiscordPlayer>>>
) -> Result<impl warp::Reply, Infallible> {
    let ret = {
        let mstate = mstate.lock().await;

        webdata::MinstrelWebData {
            current_track: match mstate.current_track.clone() {
                Some(s) => Some(s.into()),
                None => None,
            },
            status: mstate.status.clone().into(),
            queue: mstate.queue.iter().map(|e| e.clone().into()).collect(),
            upcoming: mstate.autoplay.prefetch(10).unwrap().iter().map(|e| e.clone().into()).collect(),
            history: mstate.history.iter().map(|e| e.clone().into()).collect(),
        }
    };

    Ok(warp::reply::json(&ret))
}

pub async fn start_webserver(client: &Client) {
    let mstate = client.data.read().await.get::<MusicStateKey>().cloned().unwrap();

    tokio::spawn(async move {
        let mstate = warp::any().map(move || { mstate.clone() });

        let dash = warp::get()
            .and(mstate)
            .and_then(show_state);

        warp::serve(dash)
            .run(([127,0,0,1], 3030))
            .await;
    });
}

impl Into<webdata::Requester> for Requester {
    fn into(self) -> webdata::Requester {
        let id = self.id.0.clone();

        match self.user {
            UserModels::Discord(user) => {
                webdata::Requester {
                    username: user.tag(),

                    // TODO: this should probably use nick_in, perhaps create yet another wrapper to cache this?
                    displayname: user.name.clone(),
                    icon: user.face(),
                    id: id,
                }
            },
            #[allow(unreachable_patterns)]
            _ => panic!("Only implemented for discord users, template this later"),
        }
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

// TODO: delete this eventually when types are reconciled
impl Into<webdata::MusicStateStatus> for MusicStateStatus {
    fn into(self) -> webdata::MusicStateStatus {
        match self {
            MusicStateStatus::Idle => webdata::MusicStateStatus::Idle,
            MusicStateStatus::Playing => webdata::MusicStateStatus::Playing,
            MusicStateStatus::Stopping => webdata::MusicStateStatus::Stopping,
            MusicStateStatus::Stopped => webdata::MusicStateStatus::Stopped,
        }
    }
}