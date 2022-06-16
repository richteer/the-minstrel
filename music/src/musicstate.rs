use super::autoplay::{
    AutoplayState,
    AutoplayControlCmd,
    AutoplayOk,
    AutoplayError,
};

use std::{
    collections::VecDeque,
    fmt,
    fs::OpenOptions,
    io::Write,
};

use chrono::offset::Local;

use tokio::sync::{
    oneshot,
    mpsc,
    broadcast,
};

use log::*;

use crate::player::{
    MusicPlayerCommand,
    MPCMD,
};

use crate::adapters::{
    MusicAdapter,
    AutoplayAdapter,
};

use minstrel_config::read_config;
use model::{
    SongRequest,
    MinstrelBroadcast,
    MusicStateStatus,
};
use db::DbAdapter;

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
    Data(Box<model::MinstrelWebData>),
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


#[derive(Clone, Debug)]
pub enum MusicControlCmd {
    Play(SongRequest),
    Skip,
    Stop,
    Start,
    Enqueue(SongRequest),
    EnqueueAndPlay(SongRequest),
    ClearQueue,
    ClearHistory,
    Previous,
    SongEnded,
    GetData,
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
    bcast: broadcast::Sender<model::MinstrelBroadcast>,
    cmd_channel: (mpsc::Sender<MSCMD>, mpsc::Receiver<MSCMD>),

    current_track: Option<SongRequest>,
    songstarted: Option<std::time::Instant>,
    status: MusicStateStatus,
    queue: VecDeque<SongRequest>,
    history: VecDeque<SongRequest>,
    pub autoplay: AutoplayState,
    adapter: MusicAdapter, // To work around adapters possibly having unique state due to chained constructors
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

    pub async fn new(player: mpsc::Sender<MPCMD>, db: DbAdapter) -> MusicState {
        let bcast = broadcast::channel(2).0;
        let cmd_channel = mpsc::channel(10);

        MusicState {
            adapter: MusicAdapter::new(cmd_channel.0.clone(), bcast.clone(), db.clone()),
            // TODO: use a proper channel buffer sizes here
            player,
            bcast,
            cmd_channel,

            current_track: None,
            songstarted: None,
            queue: VecDeque::<SongRequest>::new(),
            history: VecDeque::<SongRequest>::new(),
            status: MusicStateStatus::Idle,
            autoplay: AutoplayState::new(db).await,
        }
    }

    async fn player_invoke(&self, cmd: MusicPlayerCommand) -> Result<(), MusicError> {
        let (tx, rx) = oneshot::channel();
        self.player.send((tx, cmd)).await.unwrap();

        match rx.await {
            Ok(r) => r,
            Err(e) => panic!("this shouldn't be hit, but handle it better anyway: {:?}", e),
        }
    }

    pub fn get_adapter(&self) -> MusicAdapter {
        self.adapter.clone()
    }

    pub async fn run(&mut self) {
        loop {
            if let Some((rettx, cmd)) = self.cmd_channel.1.recv().await {
                let ret = match cmd {
                    MusicControlCmd::Play(song) => self.play(song).await,
                    MusicControlCmd::Skip => self.skip().await,
                    MusicControlCmd::Stop => self.stop().await,
                    MusicControlCmd::Start => self.start().await,
                    MusicControlCmd::Enqueue(song) => self.enqueue(song), // TODO: probably just make this async...
                    MusicControlCmd::EnqueueAndPlay(song) => self.enqueue_and_play(song).await,
                    MusicControlCmd::ClearQueue => self.clear_queue(),
                    MusicControlCmd::ClearHistory => self.clear_history(),
                    MusicControlCmd::Previous => self.previous().await,
                    MusicControlCmd::SongEnded => { self.song_ended().await; Ok(MusicOk::Unimplemented) },
                    MusicControlCmd::GetData => Ok(MusicOk::Data(Box::new(self.get_webdata()))),
                    MusicControlCmd::AutoplayCmd(cmd) => {
                        // TODO: this is really excessive, but broadcast after every successful autoplay command
                        //  Autoplay has no way of broadcasting on its own, so just do it every time for now
                        //  A change in the broadcasts with the partial broadcast system might be nice, to allow
                        //  different components to have control over certain aspects.
                        //  e.g. autoplay sends "Upcoming" broadcasts, MusicState only queue/history/nowplaying, etc
                        let bcast = cmd != AutoplayControlCmd::Status;
                        let ret = AutoplayAdapter::handle_cmd(cmd, &mut self.autoplay).await;
                        if ret.is_ok() && bcast {
                            self.broadcast_update();
                        }
                        ret
                    },
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
    async fn play(&mut self, song: SongRequest) -> Result<MusicOk, MusicError> {
        debug!("play called on song = {}", song);

        if self.current_track.is_some() {
            return Err(MusicError::AlreadyPlaying);
        }

        let ret = self.player_invoke(MusicPlayerCommand::Play(song.song.clone())).await;

        if let Err(e) = ret {
            if self.bcast.receiver_count() > 0 {
                let errmsg = format!("Error playing track: {:?}", e);
                let ret = self.bcast.send(MinstrelBroadcast::Error(errmsg));
                if let Err(e) = ret {
                    error!("error broadcasting update: {:?}", e);
                }
            }

            // TODO: This is really gross. A song failed to play, so signal SongEnded so that the next song can play.
            // However, this can get explosively recursive if the next N songs all fail too, since directly calling
            //   .song_ended() will lead back here (via .next()).
            // Rather than create a loop, end the call to .play() and let the event loop handle the SongEnd event.
            let mut temp = self.get_adapter();
            tokio::spawn(async move {
                temp.song_ended().await;
            });

            return Err(e);
        }

        if read_config!(songlog.enabled) {
            log_song(&song);
        }

        self.current_track = Some(song);
        self.songstarted = Some(std::time::Instant::now());
        self.status = MusicStateStatus::Playing;

        self.broadcast_update();

        Ok(MusicOk::StartedPlaying)
    }

    /// Play the next song in the queue (autoplay?)
    async fn next(&mut self) -> Result<MusicOk, MusicError> {
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

    fn get_next_song(&mut self) -> Option<SongRequest> {
        if let Some(song) = self.queue.pop_front() {
            if self.autoplay.is_enabled() && read_config!(music.queue_adds_usertime) {
                self.autoplay.add_time_to_user(&song.requested_by.id, song.song.duration);
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

        self.player_invoke(MusicPlayerCommand::Stop).await?;

        Ok(MusicOk::SkippingSong)
    }

    /// Stop the current playing track (if any)
    pub async fn stop(&mut self) -> Result<MusicOk, MusicError> {
        self.status = MusicStateStatus::Stopping;

        if let Err(e) = self.player_invoke(MusicPlayerCommand::Stop).await {
            error!("Player encountered a problem stopping track: {:?}", e);
            return Err(e);
        }

        self.status = MusicStateStatus::Stopped;
        self.current_track = None;

        self.broadcast_update();

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
    pub fn enqueue(&mut self, song: SongRequest) -> Result<MusicOk, MusicError> {
        if self.queue.len() > read_config!(music.queue_length) {
            return Err(MusicError::QueueFull)
        }

        self.queue.push_back(song);

        self.broadcast_update();

        Ok(MusicOk::EnqueuedSong)
    }

    /// Enqueue a track, and start playing music if not already playing
    pub async fn enqueue_and_play(&mut self, song: SongRequest) -> Result<MusicOk, MusicError> {
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

    pub fn get_history(&self) -> VecDeque<SongRequest> {
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


    pub fn current_song(&self) -> Option<SongRequest> {
        self.current_track.clone()
    }

    pub fn clear_queue(&mut self) -> Result<MusicOk, MusicError> {
        self.queue.clear();

        self.broadcast_update();

        Ok(MusicOk::EmptyQueue)
    }

    pub fn clear_history(&mut self) -> Result<MusicOk, MusicError> {
        self.history.clear();

        self.broadcast_update();

        Ok(MusicOk::EmptyQueue)
    }

    pub fn is_queue_empty(&self) -> bool {
        self.queue.is_empty()
    }

    // TODO: These broadcasts should really be more robust.
    //   Probably allow partial updates, as well as intelligently send them whenever
    //   MusicState is mutated, rather than having to manually call
    fn broadcast_update(&self) {
        let out: model::MinstrelWebData = self.into();

        // TODO: keep an eye on how often this appears now that this is called on
        //  every single autoplay command
        debug!("sending broadcast");

        if self.bcast.receiver_count() > 0 {
            if let Err(e) = self.bcast.send(MinstrelBroadcast::MusicState(out)) {
                error!("error broadcasting update: {:?}", e);
            }
        }
    }

    pub fn get_webdata(&self) -> model::MinstrelWebData {
        self.into()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<MinstrelBroadcast> {
        self.bcast.subscribe()
    }

    /// Handler to be called by the player when a song ends
    // TODO: perhaps replace this with a message event loop as well, maybe over a select
    //   with a timeout set to slightly more than the song length
    pub async fn song_ended(&mut self) {
        if let Some(song) = &self.current_track.take() {
            self.history.push_front(song.clone());
            self.history.truncate(read_config!(music.history_count) as usize);
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
        debug!("Song End handler mstate.next() = {:?}", ret);
        match ret {
            Ok(MusicOk::EmptyQueue) => {
                self.status = MusicStateStatus::Stopped;
                debug!("EmptyQueue returned by next(), stopping.")
            },
            Ok(MusicOk::StartedPlaying) => debug!("started the next track"),
            Ok(o) => warn!("unexpect Ok response from next? {o}"),
            Err(e) => error!("{:?}", e),
        }

    }

    pub fn song_progress(&self) -> u64 {
        match &self.songstarted {
            Some(d) => d.elapsed().as_secs(),
            None => 0,
        }
    }
 }


impl From<&MusicState> for model::MinstrelWebData {
    fn from(other: &MusicState) -> Self {
        let upcoming = other.autoplay.prefetch(read_config!(music.upcoming_count))
        // TODO: Better handle when autoplay is not enabled, or no users are enrolled
        .unwrap_or_default().iter()
            .map(|e| e.clone().into())
            .collect();

        Self {
            current_track: other.current_track.clone().map(|ct| ct.into()),
            song_progress: other.song_progress(),
            status: other.status.clone(),
            queue: other.queue.iter().map(|e| e.clone().into()).collect(),
            upcoming,
            history: other.history.iter().map(|e| e.clone().into()).collect(),
            ap_enabled: other.autoplay.is_enabled(),
        }
    }
}

// Helper to write out song played to a CSV in theory
fn log_song(song: &SongRequest) {
    let path = &read_config!(songlog.path);

    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .append(true)
        .open(path);

    let mut file = match file {
            Ok(f) => f,
            Err(e) => {
                error!("could not open/create songlog file: {:?}", e);
                return;
            }
    };

    // TODO: consider using a real serializer or CSV library
    let ret = file.write(
        format!("{time}{s}{title}{s}{artist}{s}{url}{s}{requester}\n",
            s = read_config!(songlog.seperator),
            time = Local::now().to_rfc3339(),
            title = song.song.title,
            artist = song.song.artist,
            url = song.song.url,
            requester = song.requested_by.displayname,
        ).as_bytes());

    if let Err(e) = ret {
        error!("error writing to songlog file: {:?}", e);
    }
}