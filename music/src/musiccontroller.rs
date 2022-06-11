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
    autoplay::{
        AutoplayState,
        AutoplayOk,
        AutoplayError,
        AutoplayControlCmd,
    },
};

use model::{
    Source,
    SongRequest,
    Requester,
    MinstrelUserId,
};

use db::DbAdapter;

use log::*;


/// Ergonomic adapter for communicating with the MusicState/Controller without needing
/// to manually do the message passing or wrapping it.
#[derive(Debug, Clone)]
pub struct MusicAdapter {
    pub autoplay: AutoplayAdapter,
    pub db: DbAdapter,
    bcast: broadcast::Sender<model::MinstrelBroadcast>,
    tx: mpsc::Sender<MSCMD>,
}

impl MusicAdapter {
    pub fn new(tx: mpsc::Sender<MSCMD>, bcast: broadcast::Sender<model::MinstrelBroadcast>, db: DbAdapter) -> Self {
        Self {
            autoplay: AutoplayAdapter::new(tx.clone()),
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

#[derive(Debug, Clone)]
pub struct AutoplayAdapter {
    tx: mpsc::Sender<MSCMD>,
}

impl AutoplayAdapter {
    pub fn new(tx: mpsc::Sender<MSCMD>) -> Self {
        Self {
            tx,
        }
    }

    pub fn handle_cmd(cmd: AutoplayControlCmd, ap: &mut AutoplayState) -> Result<MusicOk, MusicError> {
        let ret = match cmd {
            AutoplayControlCmd::Enable => { ap.enable(); Ok(AutoplayOk::Status(true)) },
            AutoplayControlCmd::Disable => { ap.disable(); Ok(AutoplayOk::Status(false)) },
            AutoplayControlCmd::Status => { Ok(AutoplayOk::Status(ap.is_enabled())) },
            AutoplayControlCmd::Register((req, source)) => ap.register(req, &source),
            AutoplayControlCmd::EnableUser(uid) => ap.enable_user(&uid),
            AutoplayControlCmd::DisableUser(uid) => ap.disable_user(&uid),
            AutoplayControlCmd::DisableAllUsers => { ap.disable_all_users(); Ok(AutoplayOk::RemovedUser) },
            AutoplayControlCmd::ShuffleUser(uid) => ap.shuffle_user(&uid),
            AutoplayControlCmd::Rebalance => { ap.reset_usertime(); Ok(AutoplayOk::Status(true)) }, // bs Ok, ignored anyway
            AutoplayControlCmd::UpdatePlaylist(req) => ap.update_userplaylist(&req),
            AutoplayControlCmd::AdvancePlaylist((uid, num)) => ap.advance_userplaylist(&uid, num),
        };

        match ret {
            Ok(o) => Ok(MusicOk::AutoplayOk(o)),
            Err(e) => Err(MusicError::AutoplayError(e)),
        }
    }

    async fn invoke(&mut self, cmd: AutoplayControlCmd) -> Result<AutoplayOk, AutoplayError> {
        let (tx, rx) = oneshot::channel();

        if let Err(e) = self.tx.send((tx, MusicControlCmd::AutoplayCmd(cmd))).await {
            error!("Failed to send Autoplay command to mstate = {:?}", e);
            return Err(AutoplayError::UnknownError);
        }

        match rx.await {
            Ok(r) => {
                match r {
                    Ok(MusicOk::AutoplayOk(o)) => Ok(o),
                    Err(MusicError::AutoplayError(e)) => Err(e),
                    _ => {
                        error!("somehow got a non-AutoplayOk/Error response from mstate, probably a bug");
                        Err(AutoplayError::UnknownError)
                    }
                }
            },
            Err(e) => {
                error!("Error on autoplay blocking receive, shouldn't happen: {:?}", e);
                Err(AutoplayError::UnknownError)
            },
        }
    }

    pub async fn enable(&mut self) -> Result<AutoplayOk, AutoplayError> {
        self.invoke(AutoplayControlCmd::Enable).await
    }

    pub async fn disable(&mut self) -> Result<AutoplayOk, AutoplayError> {
        self.invoke(AutoplayControlCmd::Disable).await
    }

    pub async fn is_enabled(&mut self) -> bool {
        match self.invoke(AutoplayControlCmd::Status).await {
            Ok(AutoplayOk::Status(s)) => s,
            Ok(_) | Err(_) => panic!("unexpected return from Autoplay Status command"),
        }
    }

    pub async fn register(&mut self, requester: Requester, source: &Source) -> Result<AutoplayOk, AutoplayError> {
        self.invoke(AutoplayControlCmd::Register((requester, source.clone()))).await
    }

    pub async fn enable_user(&mut self, userid: &MinstrelUserId) -> Result<AutoplayOk, AutoplayError> {
        self.invoke(AutoplayControlCmd::EnableUser(userid.clone())).await
    }

    pub async fn disable_user(&mut self, userid: &MinstrelUserId) -> Result<AutoplayOk, AutoplayError> {
        self.invoke(AutoplayControlCmd::DisableUser(userid.clone())).await
    }

    // This function does not have a return, ignore result from invoke
    pub async fn disable_all_users(&mut self) {
        self.invoke(AutoplayControlCmd::DisableAllUsers).await.unwrap();
    }

    pub async fn shuffle_user(&mut self, userid: &MinstrelUserId) -> Result<AutoplayOk, AutoplayError> {
        self.invoke(AutoplayControlCmd::ShuffleUser(userid.clone())).await
    }

    // This function does not have a return, ignore result from invoke
    pub async fn reset_usertime(&mut self) {
        self.invoke(AutoplayControlCmd::Rebalance).await.unwrap();
    }

    pub async fn update_userplaylist(&mut self, requester: &Requester) -> Result<AutoplayOk, AutoplayError> {
        self.invoke(AutoplayControlCmd::UpdatePlaylist(requester.clone())).await
    }

    pub async fn advance_userplaylist(&mut self, userid: &MinstrelUserId, num: u64) -> Result<AutoplayOk, AutoplayError> {
        self.invoke(AutoplayControlCmd::AdvancePlaylist((userid.clone(), num))).await
    }
}