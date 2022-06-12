use tokio::sync::{
    oneshot,
    broadcast,
    mpsc,
};

use crate::{
    MusicOk, MusicError,
    musicstate::{
        MusicControlCmd,
        MSCMD,
    },
};

use model::{
    SongRequest,
};

use db::DbAdapter;

use super::AutoplayAdapter;
use super::UserMgmt;

/// Ergonomic adapter for communicating with the MusicState/Controller without needing
/// to manually do the message passing or wrapping it.
#[derive(Debug, Clone)]
pub struct MusicAdapter {
    pub autoplay: AutoplayAdapter,
    pub db: DbAdapter,
    pub user: UserMgmt,
    bcast: broadcast::Sender<model::MinstrelBroadcast>,
    tx: mpsc::Sender<MSCMD>,
}

impl MusicAdapter {
    pub fn new(tx: mpsc::Sender<MSCMD>, bcast: broadcast::Sender<model::MinstrelBroadcast>, db: DbAdapter) -> Self {
        Self {
            autoplay: AutoplayAdapter::new(tx.clone()),
            user: UserMgmt::new(db.clone()),
            db,
            tx,
            bcast,
        }
    }

    async fn invoke(&self, cmd: MusicControlCmd) -> Result<MusicOk, MusicError> {
        let (tx, rx) = oneshot::channel();
        self.tx.send((tx, cmd)).await.unwrap();

        match rx.await {
            Ok(r) => r,
            Err(e) =>  panic!("this shouldn't be hit, but handle it better anyway: {:?}", e),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<model::MinstrelBroadcast> {
        self.bcast.subscribe()
    }

    /// Start playing a song
    pub async fn play(&mut self, song: SongRequest) -> Result<MusicOk, MusicError> {
        self.invoke(MusicControlCmd::Play(song)).await
    }

    pub async fn skip(&mut self) -> Result<MusicOk, MusicError> {
        self.invoke(MusicControlCmd::Skip).await
    }

    /// Stop the current playing track (if any)
    pub async fn stop(&mut self) -> Result<MusicOk, MusicError> {
        self.invoke(MusicControlCmd::Stop).await
    }

    /// Helper to play music if state has been stopped or enqueued without playing
    pub async fn start(&mut self) -> Result<MusicOk, MusicError> {
        self.invoke(MusicControlCmd::Start).await
    }

    /// Only enqueue a track to be played, do not start playing
    pub async fn enqueue(&mut self, song: SongRequest) -> Result<MusicOk, MusicError> {
        self.invoke(MusicControlCmd::Enqueue(song)).await
    }

    /// Enqueue a track, and start playing music if not already playing
    pub async fn enqueue_and_play(&mut self, song: SongRequest) -> Result<MusicOk, MusicError> {
        self.invoke(MusicControlCmd::EnqueueAndPlay(song)).await
    }

    pub async fn clear_queue(&mut self) -> Result<MusicOk, MusicError> {
        self.invoke(MusicControlCmd::ClearQueue).await
    }

    pub async fn clear_history(&mut self) -> Result<MusicOk, MusicError> {
        self.invoke(MusicControlCmd::ClearHistory).await
    }

    pub async fn get_webdata(&self) -> model::MinstrelWebData {
        match self.invoke(MusicControlCmd::GetData).await {
            Ok(MusicOk::Data(d)) => *d,
            _ => panic!("get_webdata invoke failed, should never happen"),
        }
    }

    /// Handler to be called by the player when a song ends
    // Ignore the result from invoke, there is no meaningful response here
    pub async fn song_ended(&mut self) {
        self.invoke(MusicControlCmd::SongEnded).await.unwrap();
    }

    pub async fn previous(&mut self) -> Result<MusicOk, MusicError> {
        self.invoke(MusicControlCmd::Previous).await
    }

}
