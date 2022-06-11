pub mod autoplay;
pub mod musicstate;
pub mod song;
pub mod player;
pub mod musiccontroller;

// Re-exports for the sake of making the imports prettier in main.rs
//  Probably not necessary, can be changed in the next big rework
pub use musicstate::MusicState as MusicState;
pub use musicstate::MusicOk as MusicOk;
pub use musicstate::MusicError as MusicError;

pub use player::MusicPlayer as MusicPlayer;