use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[allow(unused)]
pub struct WebConfig {
    pub bind_address: String,
    pub port: u64,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1".to_string(),
            port: 3030,
        }
    }
}