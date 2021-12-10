pub mod autoplay;
pub mod musicstate;
pub mod song;

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