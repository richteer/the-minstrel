use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[allow(unused)]
pub struct SongLogConfig {
    pub enabled: bool,
    pub path: String,
    pub seperator: char,
}

impl Default for SongLogConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            path: "songlog.tsv".to_string(),
            seperator: '\t',
        }
    }
}