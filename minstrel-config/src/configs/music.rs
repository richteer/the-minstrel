use serde::{Deserialize, Serialize};


#[derive(Clone, Debug, Deserialize, Serialize)]
#[allow(unused)]
pub struct MusicConfig {
    pub queue_length: usize,
    pub queue_adds_usertime: bool,
    pub autoplay_prefetch_max: u64,
}

impl Default for MusicConfig {
    fn default() -> Self {
        Self {
            queue_length: 10,
            queue_adds_usertime: true,
            autoplay_prefetch_max: 50,
        }
    }
}