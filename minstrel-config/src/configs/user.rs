use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[allow(unused)]
pub struct UserConfig {
    pub link_timeout: u64,
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            link_timeout: 60 * 60,
        }
    }
}