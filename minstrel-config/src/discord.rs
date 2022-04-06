use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[allow(unused)]
pub struct DiscordConfig {
    pub autoplay_upcoming_max: u64,
    pub queuestate_ap_count: u64,
    pub webdash_prefetch: u64,
}

impl Default for DiscordConfig {
    fn default() -> Self {
        Self {
            autoplay_upcoming_max: 10,
            queuestate_ap_count: 10,
            webdash_prefetch: 10,
        }
    }
}