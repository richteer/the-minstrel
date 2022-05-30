use super::autoplay::{
    AutoplayState,
    AutoplayControlCmd,
    AutoplayOk,
    AutoplayError,
};
use super::song::Song;

use std::fmt;
use std::collections::VecDeque;

use tokio::sync::{
    oneshot,
    mpsc,
    broadcast,
};

use log::*;
use serde::Serialize;

use crate::player::{
    MusicPlayerCommand,
    MPCMD,
};

use crate::musiccontroller::{
    MusicAdapter,
    AutoplayAdapter,
};

use minstrel_config::read_config;

#[allow(dead_code)]
#[non_exhaustive]
#[derive(Debug)]
pub enum MusicOk {
    StartedPlaying,
    StoppedPlaying,
    NotPlaying,
    EnqueuedSong,
    EmptyQueue,
    NothingToPlay,
    SkippingSong,
    AutoplayOk(AutoplayOk),
    Unimplemented
}

impl fmt::Display for MusicOk {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        #[allow(unreachable_patterns)]
        let ret = match self {
            MusicOk::StartedPlaying => "Started playing.",
            MusicOk::StoppedPlaying => "Stopped playing.",
            MusicOk::NotPlaying     => "Not currently playing.",
            MusicOk::EnqueuedSong   => "Enqueued song.",
            MusicOk::EmptyQueue     => "Queue is empty.",
            MusicOk::NothingToPlay  => "Nothing to play.",
            MusicOk::SkippingSong   => "Skipping song.",
            MusicOk::Unimplemented  => "Unimplemented Ok message",
            _ => "Unknown response, fill me in!",
        };

        write!(f, "{}", ret)
    }
}


#[allow(dead_code)]
#[non_exhaustive]
#[derive(Debug)] // TODO: maybe just implement Display here, so that error messages are automatic?
pub enum MusicError {
    UnknownError, // TODO: try to replace all UnknownError usages with better errors
    AlreadyPlaying,
    QueueFull,
    InvalidUrl,
    FailedToRetrieve,
    EmptyHistory,
    AutoplayError(AutoplayError),
}


#[non_exhaustive]
#[derive(Clone, Debug, Serialize)]
pub enum MusicStateStatus {
    Playing,
    Stopping,
    Stopped,
    Idle,
}


// TODO: delete this eventually when types are reconciled
impl From<MusicStateStatus> for webdata::MusicStateStatus {
    fn from(mss: MusicStateStatus) -> Self {
        match mss {
            MusicStateStatus::Idle => webdata::MusicStateStatus::Idle,
            MusicStateStatus::Playing => webdata::MusicStateStatus::Playing,
            MusicStateStatus::Stopping => webdata::MusicStateStatus::Stopping,
            MusicStateStatus::Stopped => webdata::MusicStateStatus::Stopped,
            #[allow(unreachable_patterns)]
            _ => todo!("unknown music state status obtained from music crate"),
        }
    }
}

#[derive(Clone, Debug)]
pub enum MusicControlCmd {
    Play(Song),
    Next,
    Skip,
    Stop,
    Start,
    Enqueue(Song),
    EnqueueAndPlay(Song),
    Previous,
    AutoplayCmd(AutoplayControlCmd),
}

pub type MusicResult = Result<MusicOk, MusicError>;
pub type MSCMD = (oneshot::Sender<MusicResult>, MusicControlCmd);

// Higher level manager for playing music. In theory, should abstract out
//   a lot of the lower-level magic, so the commands can just operate on
//   this instead and make life easier.
pub struct MusicState {
    player: mpsc::Sender<MPCMD>,
    // TODO: Perhaps put this in a higher level lock, so maybe it's automatic?
    bcast: broadcast::Sender<webdata::MinstrelWebData>,
    cmd_channel: (mpsc::Sender<MSCMD>, mpsc::Receiver<MSCMD>),

    current_track: Option<Song>,
    status: MusicStateStatus,
    queue: VecDeque<Song>,
    history: VecDeque<Song>,
    pub autoplay: AutoplayState,
}

impl fmt::Debug for MusicState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "MusicState {{ \
            player: {:?}, \
            status: {:?}, \
            queue: <{} songs>, \
            history: <{} songs>, \
            autoplay: ..., \
        }}",
            "player goes here",
            //&self.player,
            &self.status,
            &self.queue.len(),
            &self.history.len(),
            // Autoplay
        )
    }
}

impl MusicState {

    pub fn new(player: mpsc::Sender<MPCMD>) -> MusicState {
        MusicState {
            // TODO: use a proper channel buffer sizes here
            player,
            bcast: broadcast::channel(2).0,
            cmd_channel: mpsc::channel(10),

            current_track: None,
            queue: VecDeque::<Song>::new(),
            history: VecDeque::<Song>::new(),
            status: MusicStateStatus::Idle,
            autoplay: AutoplayState::new(),
        }
    }

    async fn invoke(&self, cmd: MusicPlayerCommand) -> Result<(), MusicError> {
        let (tx, rx) = oneshot::channel();
        self.player.send((tx, cmd)).await.unwrap();

        match rx.await {
            Ok(r) => r,
            Err(e) => todo!("this shouldn't be hit, but handle it better anyway: {:?}", e),
        }
    }

    pub fn get_adapter(&self) -> MusicAdapter {
        MusicAdapter::new(self.cmd_channel.0.clone(), self.bcast.clone())
    }

    pub async fn run(&mut self) {
        loop {
            if let Some((rettx, cmd)) = self.cmd_channel.1.recv().await {
                let ret = match cmd {
                    MusicControlCmd::Play(song) => self.play(song).await,
                    MusicControlCmd::Next => self.next().await,
                    MusicControlCmd::Skip => self.skip().await,
                    MusicControlCmd::Stop => self.stop().await,
                    MusicControlCmd::Start => self.start().await,
                    MusicControlCmd::Enqueue(song) => self.enqueue(song), // TODO: probably just make this async...
                    MusicControlCmd::EnqueueAndPlay(song) => self.enqueue_and_play(song).await,
                    MusicControlCmd::Previous => self.previous().await,
                    MusicControlCmd::AutoplayCmd(cmd) => AutoplayAdapter::handle_cmd(cmd, &mut self.autoplay),
                };

                if let Err(e) = rettx.send(ret) {
                    error!("oneshot return might have dropped, this shouldn't happen: {:?}", e);
                };
            } else {
                error!("MusicState commandloop exiting?");
            }
        }
    }

    /// Start playing a song
    async fn play(&mut self, song: Song) -> Result<MusicOk, MusicError> {
        debug!("play called on song = {}", song);

        if self.current_track.is_some() {
            return Err(MusicError::AlreadyPlaying);
        }

        self.invoke(MusicPlayerCommand::Play(song.clone())).await?;

        self.current_track = Some(song);
        self.status = MusicStateStatus::Playing;

        self.broadcast_update();

        Ok(MusicOk::StartedPlaying)
    }

    /// Play the next song in the queue (autoplay?)
    pub async fn next(&mut self) -> Result<MusicOk, MusicError> {
        let song = self.get_next_song();

        if let Some(song) = song {
            debug!("next song is {}", song);
            self.play(song).await
        }
        else {
            debug!("no next song, ending");
            Ok(MusicOk::EmptyQueue)
        }
    }

    fn get_next_song(&mut self) -> Option<Song> {
        if let Some(song) = self.queue.pop_front() {
            if self.autoplay.is_enabled() && read_config!(music.queue_adds_usertime) {
                self.autoplay.add_time_to_user(&song.requested_by.id, song.duration);
            }

            return Some(song);
        }

        if self.autoplay.is_enabled() {
            return self.autoplay.next();
        }

        None
    }

    // Stop the current track, but don't signal to the event handler to actually cease playing
    // This is stupid, and I don't like it.
    // TODO: This is hella discord-specific. Rewrite this function to actually skip.
    pub async fn skip(&mut self) -> Result<MusicOk, MusicError> {

        self.invoke(MusicPlayerCommand::Stop).await?;

        Ok(MusicOk::SkippingSong)
    }

    /// Stop the current playing track (if any)
    pub async fn stop(&mut self) -> Result<MusicOk, MusicError> {
        self.status = MusicStateStatus::Stopping;

        if let Err(e) = self.invoke(MusicPlayerCommand::Stop).await {
            error!("Player encountered a problem stopping track: {:?}", e);
            return Err(e);
        }

        self.status = MusicStateStatus::Stopped;
        self.current_track = None;

        Ok(MusicOk::StoppedPlaying)
    }

    /// Helper to play music if state has been stopped or enqueued without playing
    pub async fn start(&mut self) -> Result<MusicOk, MusicError> {
        if let MusicStateStatus::Playing = self.status {
            return Err(MusicError::AlreadyPlaying);
        };

        if let Some(song) = self.get_next_song() {
            self.play(song).await
        }
        else {
            Ok(MusicOk::NothingToPlay)
        }
    }

    /// Only enqueue a track to be played, do not start playing
    pub fn enqueue(&mut self, song: Song) -> Result<MusicOk, MusicError> {
        if self.queue.len() > read_config!(music.queue_length) {
            return Err(MusicError::QueueFull)
        }

        self.queue.push_back(song);

        Ok(MusicOk::EnqueuedSong)
    }

    /// Enqueue a track, and start playing music if not already playing
    pub async fn enqueue_and_play(&mut self, song: Song) -> Result<MusicOk, MusicError> {
        self.enqueue(song)?;

        match self.start().await {
            Ok(m) => Ok(m),
            Err(MusicError::AlreadyPlaying) => Ok(MusicOk::EnqueuedSong),
            Err(e) => Err(e),
        }
    }

    /// Get a display string for the queue
    pub fn show_queue(&self) -> String {
        let mut ret = String::from("Current play queue:\n");

        for (i,v) in self.queue.iter().enumerate() {
            ret += &format!("{}: {}\n", i+1, &v).to_owned();
        }

        ret
    }

    pub fn get_history(&self) -> VecDeque<Song> {
        self.history.clone()
    }

    pub async fn previous(&mut self) -> Result<MusicOk, MusicError> {

        if let Some(song) = self.history.pop_front() {
            self.enqueue_and_play(song).await
        }
        else {
            Err(MusicError::EmptyHistory)
        }

    }


    pub fn current_song(&self) -> Option<Song> {
        self.current_track.clone()
    }

    pub fn clear_queue(&mut self) -> Result<MusicOk, MusicError> {
        self.queue.clear();

        Ok(MusicOk::EmptyQueue)
    }

    pub fn is_queue_empty(&self) -> bool {
        self.queue.is_empty()
    }


    // TODO: this is slated for removal from MusicState, leaving for the scope of this refactor
    // TODO: This is definitely to be removed soon. Only discord has a concept of a "connection" that needs to be
    //   dropped without completely destroying the MusicPlayer, so this should be removed.
    pub async fn leave(&mut self) {

        self.queue.clear();
        self.autoplay.disable();
        self.autoplay.disable_all_users();

        if let Err(e) = self.stop().await {
            error!("{:?}", e);
        };

        // TODO: this assumes player stops on disconnect. Artifact of discordisms, since .stop() acts like skip sometimes
        //  explicitly stop the music first if this function actually remains here

        if let Err(e) =  self.invoke(MusicPlayerCommand::Disconnect).await {
            panic!("somehow disconnect responded with an Error: {:?}", e);
        };
    }

    // TODO: These broadcasts should really be more robust.
    //   Probably allow partial updates, as well as intelligently send them whenever
    //   MusicState is mutated, rather than having to manually call
    fn broadcast_update(&self) {
        let out: webdata::MinstrelWebData = self.into();

        if self.bcast.receiver_count() > 0 {
            if let Err(e) = self.bcast.send(out) {
                error!("error broadcasting update: {:?}", e);
            }
        }
    }

    pub fn get_webdata(&self) -> webdata::MinstrelWebData {
        self.into()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<webdata::MinstrelWebData> {
        self.bcast.subscribe()
    }

    /// Handler to be called by the player when a song ends
    // TODO: perhaps replace this with a message event loop as well, maybe over a select
    //   with a timeout set to slightly more than the song length
    pub async fn song_ended(&mut self) {
        if let Some(song) = &self.current_track.take() {
            self.history.push_front(song.clone());
            self.history.truncate(10); // TODO: config max history buffer length
        }
        else {
            warn!("Song End handler somehow called with mstate.current_track = None, history may be inaccurate");
        }

        // TODO: perhaps have a "continuous play" bool instead in state?
        match self.status {
            MusicStateStatus::Stopping | MusicStateStatus::Stopped => {
                debug!("MusicStateStatus requesting a stop, not enqueueing next track");
                return; // We're done here
            }
            _ => {}
        };

        let ret = self.next().await;
        if ret.is_ok() {
            debug!("Song End handler mstate.next() = {:?}", ret);
        }
        else if let Err(e) = ret {
            error!("{:?}", e);
        }

    }
 }

impl Into<webdata::MinstrelWebData> for &MusicState {
    fn into(self) -> webdata::MinstrelWebData {
        let upcoming = self.autoplay.prefetch(read_config!(discord.webdash_prefetch))
        // TODO: Better handle when autoplay is not enabled, or no users are enrolled
        .unwrap_or_default().iter()
            .map(|e| e.clone().into())
            .collect();

        webdata::MinstrelWebData {
            current_track: self.current_track.clone().map(|ct| ct.into()),
            status: self.status.clone().into(),
            queue: self.queue.iter().map(|e| e.clone().into()).collect(),
            upcoming,
            history: self.history.iter().map(|e| e.clone().into()).collect(),
        }
    }
}