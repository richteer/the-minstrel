use super::autoplay::AutoplayState;
use super::song::Song;

use std::fmt;
use std::sync::Arc;
use std::collections::VecDeque;

use songbird::{
    Event,
    EventContext,
    EventHandler as VoiceEventHander,
    TrackEvent,
    tracks::TrackHandle,
};

use serenity::{
    prelude::*,
};

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
#[derive(Clone, Debug)]
pub enum MusicStateStatus {
    Playing,
    Stopping,
    Stopped,
    Initialized,
    Uninitialized,
}

use serenity::{
    async_trait,
    client::{ClientBuilder},
};

// Higher level manager for playing music. In theory, should abstract out
//   a lot of the lower-level magic, so the commands can just operate on
//   this instead and make life easier.
pub struct MusicState {
    songcall: Option<Arc<tokio::sync::Mutex<songbird::Call>>>,
    pub current_track: Option<(TrackHandle, Song)>,
    status: MusicStateStatus,
    queue: VecDeque<Song>,
    pub autoplay: AutoplayState,
}

// TODO: Make this a config setting probably
const MAX_QUEUE_LEN: usize = 10;

impl MusicState {

    // Initialize the MusicState for the given context and voice channel
    //   Also joins the channel
    pub async fn init(
        &mut self,
        ctx: &Context,
        guild_id: serenity::model::id::GuildId,
        channel_id: serenity::model::id::ChannelId
    ) {
        // Bot is not in voice, so join caller's.
        let manager = songbird::get(ctx).await
        .expect("Songbird Voice client placed in at initialisation.").clone();

        let handler = manager.join(guild_id, channel_id).await.0;

        handler.lock().await.add_global_event(
            Event::Track(TrackEvent::End),
            TrackEndNotifier {
                ctx: ctx.clone()
            },
        );

        self.songcall = Some(handler.clone());
        self.current_track = None;
        self.status = MusicStateStatus::Initialized;

    }

    pub fn new() -> MusicState {
        MusicState {
            songcall: None,
            current_track: None,
            queue: VecDeque::<Song>::new(),
            status: MusicStateStatus::Uninitialized,
            autoplay: AutoplayState::new(),
        }
    }

    /// Start playing a song
    async fn play(&mut self, song: Song) -> Result<MusicOk, MusicError> {
        println!("play called");
        if self.songcall.is_none() {
            return Err(MusicError::UnknownError);
        }

        if self.current_track.is_some() {
            return Err(MusicError::AlreadyPlaying);
        }

        let mut handler = self.songcall.as_ref().unwrap().lock().await;

        let source = match songbird::ytdl(&song.url).await {
            Ok(source) => source,
            Err(why) => {
                println!("Err starting source: {:?}", why);
                return Err(MusicError::UnknownError);
            },
        };

        let thandle = handler.play_source(source);
        self.current_track = Some((thandle, song));

        self.status = MusicStateStatus::Playing;

        Ok(MusicOk::StartedPlaying)
    }

    /// Play the next song in the queue (autoplay?)
    async fn next(&mut self) -> Result<MusicOk, MusicError> {
        println!("next called: curr = {:?}", &self.current_track);
        let song = self.get_next_song();

        if let Some(song) = song {
            self.play(song).await
        }
        else {
            Ok(MusicOk::EmptyQueue)
        }
    }

    fn get_next_song(&mut self) -> Option<Song> {
        let ret = self.queue.pop_front();

        if ret.is_some() {
            return ret;
        }

        if self.autoplay.enabled {
            return self.autoplay.next();
        }

        None
    }

    // Stop the current track, but don't signal to the event handler to actually cease playing
    // This is stupid, and I don't like it.
    pub async fn skip(&mut self) -> Result<MusicOk, MusicError> {
        if let Some((thandle, _)) = &self.current_track {
            thandle.stop().ok();
        }

        Ok(MusicOk::SkippingSong)
    }

    /// Stop the current playing track (if any)
    pub async fn stop(&mut self) -> Result<MusicOk, MusicError> {
        self.status = MusicStateStatus::Stopping;

        if let Some((thandle, _)) = &self.current_track {
            if thandle.stop().is_err() {
                return Err(MusicError::UnknownError);
            }
        }
        else {
            self.status = MusicStateStatus::Stopped;
            return Ok(MusicOk::NotPlaying);
        }

        Ok(MusicOk::StoppedPlaying)
    }

    /// Helper to play music if state has been stopped or enqueued without playing
    pub async fn start(&mut self) -> Result<MusicOk, MusicError> {
        match self.status {
            MusicStateStatus::Playing => return Err(MusicError::AlreadyPlaying),
            _ => (),
        };

        if let Some(song) = self.get_next_song() {
            self.play(song).await
        }
        else {
            Ok(MusicOk::NothingToPlay)
        }
    }

    /// Only enqueue a track to be played, do not start playing
    pub async fn enqueue(&mut self, song: Song) -> Result<MusicOk, MusicError> {
        if self.queue.len() > MAX_QUEUE_LEN {
            return Err(MusicError::QueueFull)
        }

        self.queue.push_back(song);

        Ok(MusicOk::EnqueuedSong)
    }

    /// Enqueue a track, and start playing music if not already playing
    pub async fn enqueue_and_play(&mut self, song: Song) -> Result<MusicOk, MusicError> {
        self.queue.push_back(song);

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
        match &self.current_track {
            Some((_, song)) => Some(song.clone()),
            None => None,
        }
    }

    pub fn clear_queue(&mut self) -> Result<MusicOk, MusicError> {
        self.queue.clear();

        Ok(MusicOk::EmptyQueue)
    }

    pub fn is_queue_empty(&self) -> bool {
        self.queue.is_empty()
    }

}

/* Possible mess for queue support */

struct TrackEndNotifier {
    ctx: Context,
}

#[async_trait]
impl VoiceEventHander for TrackEndNotifier {

    // TODO: somehow make this a signaling thing so we don't have to await here
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        let mstate = get(&self.ctx).await.unwrap();
        let mut mstate = mstate.lock().await;

        mstate.current_track = None;

        let s = mstate.status.clone();
        mstate.status = MusicStateStatus::Stopped;
        match s {
            MusicStateStatus::Stopping => return None, // We're done here
            _ => {}
        };

        let ret = mstate.next().await;
        if let Ok(_) = ret {

        }
        else if let Err(e) = ret {
            println!("{:?}", e);
        }

        None
    }
}


/* Enter mess to make the singleton magic via serenity here */
pub struct MusicStateKey;

impl TypeMapKey for MusicStateKey {
    type Value = Arc<Mutex<MusicState>>;
}

pub trait MusicStateInit {
    fn register_musicstate(self) -> Self;
}

fn register(client_builder: ClientBuilder) -> ClientBuilder {
    let tmp = Arc::new(Mutex::new(MusicState::new()));
    client_builder
        .type_map_insert::<MusicStateKey>(tmp.clone())
}

impl MusicStateInit for ClientBuilder<'_> {
    fn register_musicstate(self) -> Self {
        register(self)
    }
}

pub async fn get(ctx: &Context) -> Option<Arc<Mutex<MusicState>>> {
    let data = ctx.data.read().await;

    let mstate = data.get::<MusicStateKey>().cloned();

    mstate
}
