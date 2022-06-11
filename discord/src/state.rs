use std::sync::Arc;
use tokio::sync::Mutex;
use serenity::model::channel::Message;
use music::adapters::MusicAdapter;
use log::*;
use crate::player::DiscordPlayer;

pub struct DiscordState {
    pub sticky: Option<Message>,
    mstate: MusicAdapter,
    dplayer: Arc<Mutex<DiscordPlayer>>,
}

impl DiscordState {
    pub fn new(mstate: MusicAdapter, dplayer: Arc<Mutex<DiscordPlayer>>) -> Self {
        Self {
            sticky: None,
            mstate,
            dplayer,
        }
    }

    pub async fn leave(&mut self) {

        // TODO: perhaps this shouldn't burn everything down on leaving.
        //   Bot-only mode, yes, but not if it is just playing music via discord
        self.mstate.clear_queue().await.unwrap();
        self.mstate.autoplay.disable().await.unwrap();
        self.mstate.autoplay.disable_all_users().await;

        if let Err(e) = self.mstate.stop().await {
            error!("{:?}", e);
        };

        let mut dplayer = self.dplayer.lock().await;
        dplayer.disconnect().await;
    }
}