use super::autoplay::AutoplayState;
use super::song::Song;

use std::fmt;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;
use log::*;
use serde::Serialize;

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

use super::MusicPlayer;


// Higher level manager for playing music. In theory, should abstract out
//   a lot of the lower-level magic, so the commands can just operate on
//   this instead and make life easier.
pub struct MusicState<T: MusicPlayer> {
    pub player: Arc<Mutex<Box<T>>>,
    pub current_track: Option<Song>,
    pub status: MusicStateStatus,
    pub queue: VecDeque<Song>,
    pub history: VecDeque<Song>,
    pub autoplay: AutoplayState,
}

impl<T: MusicPlayer> fmt::Debug for MusicState<T> {
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

impl<T: MusicPlayer> MusicState<T> {

    pub fn new(player: T) -> MusicState<T> {
        MusicState {
            player: Arc::new(Mutex::new(Box::new(player))),
            current_track: None,
            queue: VecDeque::<Song>::new(),
            history: VecDeque::<Song>::new(),
            status: MusicStateStatus::Idle,
            autoplay: AutoplayState::new(),
        }
    }

    /// Start playing a song
    async fn play(&mut self, song: Song) -> Result<MusicOk, MusicError> {
        debug!("play called on song = {}", song);
        let mut player = self.player.lock().await;

        if self.current_track.is_some() {
            return Err(MusicError::AlreadyPlaying);
        }

        player.play(&song).await?;

        self.current_track = Some(song);
        self.status = MusicStateStatus::Playing;

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
            // TODO: Config this
            // TODO: probably reconsider where this needs to go
            if self.autoplay.enabled {
                self.autoplay.add_time_to_user(&song.requested_by.id, song.duration);
            }

            return Some(song);
        }

        if self.autoplay.enabled {
            return self.autoplay.next();
        }

        None
    }

    // Stop the current track, but don't signal to the event handler to actually cease playing
    // This is stupid, and I don't like it.
    // TODO: This is hella discord-specific. Rewrite this function to actually skip.
    pub async fn skip(&mut self) -> Result<MusicOk, MusicError> {
        let mut player = self.player.lock().await;

        player.stop().await?;

        Ok(MusicOk::SkippingSong)
    }

    /// Stop the current playing track (if any)
    pub async fn stop(&mut self) -> Result<MusicOk, MusicError> {
        self.status = MusicStateStatus::Stopping;

        let mut player = self.player.lock().await;

        if let Err(e) = player.stop().await {
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
    pub async fn leave(&mut self) {

        self.queue.clear();
        self.autoplay.enabled = false;
        self.autoplay.disable_all_users();

        if let Err(e) = self.stop().await {
            error!("{:?}", e);
        };

        let mut player = self.player.lock().await;
        // TODO: this assumes player stops on disconnect. Artifact of discordisms, since .stop() acts like skip sometimes
        //  explicitly stop the music first if this function actually remains here
        player.disconnect().await;
    }
}


