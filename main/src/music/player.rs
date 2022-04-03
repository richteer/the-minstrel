use crate::music::*;

use async_trait::async_trait;

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