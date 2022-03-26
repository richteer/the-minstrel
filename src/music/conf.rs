use serde::{Deserialize, Serialize};


#[derive(Debug, Deserialize, Serialize)]
#[allow(unused)]
pub struct MusicConfig {
    pub queue_length: usize,
    pub autoplay_prefetch_max: u64,
}

impl Default for MusicConfig {
    fn default() -> Self {
        Self {
            queue_length: 10,
            autoplay_prefetch_max: 10,
        }
    }
}