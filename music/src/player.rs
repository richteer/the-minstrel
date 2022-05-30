use crate::*;

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use tokio::sync::{
    oneshot,
    mpsc,
};

use log::*;

/// Interface for whatever is playing the music
#[async_trait]
pub trait MusicPlayer {
    // TODO: Perhaps make an enum for player errors at some point

    // For whatever initialization procedure might be needed
    async fn init(&self) -> Result<(), MusicError>;

    /// Start playing the supplied track
    async fn play(&mut self, song: &Song) -> Result<(), MusicError>;

    /// Stop playing the current track
    async fn stop(&mut self) -> Result<(), MusicError>;

    /// Temporary trait function for this current refactor step. This should be a player-specific feature
    async fn disconnect(&mut self);
}

#[derive(Clone, Debug)]
pub enum MusicPlayerCommand {
    Play(Song),
    Stop,
    Disconnect, // TODO: Hopefully not need this? Let this be frontend/application controller stuff
}

pub struct MusicPlayerTask<T: MusicPlayer> {
    // TODO: Figure out better ownership here
    player: Arc<Mutex<T>>,
    pub receiver: mpsc::Receiver<MPCMD>

}

pub type MPCMD = (oneshot::Sender<Result<(), MusicError>>, MusicPlayerCommand);

impl<T: MusicPlayer> MusicPlayerTask<T> {
    pub fn new(player: Arc<Mutex<T>>, receiver: mpsc::Receiver<MPCMD>) -> Self {
        Self {
            player,
            receiver,
        }
    }

    pub async fn run(&mut self) {
        trace!("playertask loop starting...");
        loop {
            let (rettx, cmd) = match self.receiver.recv().await {
                None => break,
                Some(cmd) => cmd,
            };
            trace!("got command: {:?}", cmd);

            let ret = {
                let mut player = self.player.lock().await;
                match cmd {
                    MusicPlayerCommand::Play(s) => player.play(&s).await,
                    MusicPlayerCommand::Stop => player.stop().await,
                    MusicPlayerCommand::Disconnect => {
                        player.disconnect().await;
                        Ok(())
                    },
                }
            };

            if let Err(e) = rettx.send(ret) {
                todo!("Apparently the receiver dropped? {:?}", e);
            }
        }
        warn!("exiting playertask loop, probably not intended yet?");
    }
}