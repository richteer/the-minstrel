use config::{Config, ConfigError, File};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::sync::RwLock;

mod music;
use music::MusicConfig;

// TODO: feature this?
mod discord;
use discord::DiscordConfig;

mod web;
use web::WebConfig;

mod songlog;
use songlog::SongLogConfig;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[allow(unused)]
pub struct Configuration {
    pub music: MusicConfig,
    pub discord: DiscordConfig,
    pub web: WebConfig,
    pub songlog: SongLogConfig,
}

lazy_static! {
    pub static ref CONFIG: RwLock<Configuration> = RwLock::new(Configuration::new().unwrap());
}

impl Configuration {
    fn new() -> Result<Self, ConfigError> {

        let conf = Config::builder()
            .add_source(Config::try_from(&Configuration::default()).unwrap())
            .add_source(File::with_name("config.toml").required(false))
            .add_source(File::with_name("devel.toml").required(false))
            .build()?;

        conf.try_deserialize()
    }
}

#[macro_export]
macro_rules! read_config {
    ($($field:ident).+) => {
        minstrel_config::CONFIG.read().unwrap().$($field).+
    };
}