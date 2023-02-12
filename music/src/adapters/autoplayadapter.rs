use tokio::sync::{
    oneshot,
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
    Requester,
    MinstrelUserId,
};

use log::*;


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

    pub async fn handle_cmd(cmd: AutoplayControlCmd, ap: &mut AutoplayState) -> Result<MusicOk, MusicError> {
        let ret = match cmd {
            AutoplayControlCmd::Enable => { ap.enable(); Ok(AutoplayOk::Status(true)) },
            AutoplayControlCmd::Disable => { ap.disable(); Ok(AutoplayOk::Status(false)) },
            AutoplayControlCmd::Status => { Ok(AutoplayOk::Status(ap.is_enabled())) },
            AutoplayControlCmd::EnableUser(uid) => ap.enable_user(&uid),
            AutoplayControlCmd::DisableUser(uid) => ap.disable_user(&uid),
            AutoplayControlCmd::DisableAllUsers => { ap.disable_all_users(); Ok(AutoplayOk::RemovedUser) },
            AutoplayControlCmd::ShuffleUser(uid) => ap.shuffle_user(&uid),
            AutoplayControlCmd::Rebalance => { ap.reset_usertime(); Ok(AutoplayOk::Status(true)) }, // bs Ok, ignored anyway
            AutoplayControlCmd::UpdatePlaylist(req) => ap.update_userplaylist(&req).await,
            AutoplayControlCmd::AdvancePlaylist((uid, num)) => ap.advance_userplaylist(&uid, num),
            AutoplayControlCmd::BumpPlaylist((uid, ind)) => ap.bump_userplaylist(&uid, ind),
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

    pub async fn enable_user(&mut self, userid: &MinstrelUserId) -> Result<AutoplayOk, AutoplayError> {
        self.invoke(AutoplayControlCmd::EnableUser(*userid)).await
    }

    pub async fn disable_user(&mut self, userid: &MinstrelUserId) -> Result<AutoplayOk, AutoplayError> {
        self.invoke(AutoplayControlCmd::DisableUser(*userid)).await
    }

    // This function does not have a return, ignore result from invoke
    pub async fn disable_all_users(&mut self) {
        self.invoke(AutoplayControlCmd::DisableAllUsers).await.unwrap();
    }

    pub async fn shuffle_user(&mut self, userid: &MinstrelUserId) -> Result<AutoplayOk, AutoplayError> {
        self.invoke(AutoplayControlCmd::ShuffleUser(*userid)).await
    }

    // This function does not have a return, ignore result from invoke
    pub async fn reset_usertime(&mut self) {
        self.invoke(AutoplayControlCmd::Rebalance).await.unwrap();
    }

    pub async fn update_userplaylist(&mut self, requester: &Requester) -> Result<AutoplayOk, AutoplayError> {
        self.invoke(AutoplayControlCmd::UpdatePlaylist(requester.clone())).await
    }

    pub async fn advance_userplaylist(&mut self, userid: &MinstrelUserId, num: u64) -> Result<AutoplayOk, AutoplayError> {
        self.invoke(AutoplayControlCmd::AdvancePlaylist((*userid, num))).await
    }

    pub async fn bump_userplaylist(&mut self, userid: &MinstrelUserId, index: usize) -> Result<AutoplayOk, AutoplayError> {
        self.invoke(AutoplayControlCmd::BumpPlaylist((*userid, index))).await
    }
}