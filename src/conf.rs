use config::{Config, ConfigError, File};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::sync::RwLock;

use crate::music::conf::MusicConfig;
use crate::discord::conf::DiscordConfig;

#[derive(Debug, Default, Deserialize, Serialize)]
#[allow(unused)]
pub struct Configuration {
    pub music: MusicConfig,
    pub discord: DiscordConfig,
}

lazy_static! {
    pub static ref CONFIG: RwLock<Configuration> = RwLock::new(Configuration::new().unwrap());
}

impl Configuration {
    fn new() -> Result<Self, ConfigError> {

        let conf = Config::builder()
            .add_source(Config::try_from(&Configuration::default()).unwrap())
            .add_source(File::with_name("config.toml"))
            .add_source(File::with_name("devel.toml"))
            .build()?;

        conf.try_deserialize()
    }
}

#[macro_export]
macro_rules! read_config {
    ($($field:ident).+) => {
        crate::conf::CONFIG.read().unwrap().$($field).+
    };
}