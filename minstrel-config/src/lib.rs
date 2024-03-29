use config::{Config, ConfigError, File};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::sync::RwLock;

mod configs;
use configs::*;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[allow(unused)]
pub struct Configuration {
    pub music: MusicConfig,
    pub discord: DiscordConfig,
    pub songlog: SongLogConfig,
    pub user: UserConfig,
    pub web: WebConfig,
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