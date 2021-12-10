pub mod autoplay;
pub mod musicstate;
pub mod song;
pub mod requester;

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
    Unimplemented
}

use std::fmt;

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

// Re-exports for the sake of making the imports prettier in main.rs
//  Probably not necessary, can be changed in the next big rework
pub use musicstate::get as get;
pub use musicstate::MusicState as MusicState;
pub use musicstate::MusicStateInit as MusicStateInit;
pub use song::Song as Song;
pub use requester::Requester as Requester;
