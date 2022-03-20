use std::{
    sync::Arc,
};

use serenity::{
    prelude::Context,
    model::id::{
        GuildId,
        ChannelId,
    }
};

use songbird::{
    Event,
    EventContext,
    EventHandler as VoiceEventHandler,
    TrackEvent,
};

use async_trait::async_trait;

use log::*;
use crate::music::player::MusicPlayer;
use crate::music::Song;
use crate::music::*;

use crate::discord::commands::helpers::*;

/// Struct to maintain discord's music player state
pub struct DiscordPlayer {
    songcall: Option<Arc<tokio::sync::Mutex<songbird::Call>>>,
    songhandler: Option<songbird::tracks::TrackHandle>,
}

impl DiscordPlayer {
    pub async fn connect(ctx: &Context, guild_id: GuildId, channel_id: ChannelId) -> DiscordPlayer {
        let manager = songbird::get(ctx).await
            .expect("Songbird Voice client placed in at initialisation.").clone();

        let handler = manager.join(guild_id, channel_id).await.0;

        handler.lock().await.add_global_event(
            Event::Track(TrackEvent::End),
            TrackEndNotifier {
                ctx: ctx.clone()
            },
        );

        DiscordPlayer {
            songcall: Some(handler),
            songhandler: None,
        }
    }
}

#[async_trait]
impl MusicPlayer for DiscordPlayer {


    async fn init(&self) -> Result<(), MusicError> {
        Ok(())
    }

    async fn play(&mut self, song: &Song) -> Result<(), MusicError> {

        let mut handler = self.songcall.as_ref().unwrap().lock().await;

        let source = match songbird::ytdl_ffmpeg_args(&song.url, &[], &["-af", "loudnorm=I=-16:TP=-1.5:LRA=11"]).await {
            Ok(source) => source,
            Err(why) => {
                error!("Err starting source: {:?}", why);
                self.songhandler = None;
                return Err(MusicError::UnknownError);
            },
        };

        self.songhandler = Some(handler.play_source(source));

        Ok(())
    }

    async fn stop(&mut self) -> Result<(), MusicError> {
        if let Some(thandle) = &self.songhandler {
            // TODO: probably actually error handle this
            thandle.stop().ok();
            self.songhandler = None
        }

        Ok(())
    }

    async fn disconnect(&mut self) {
        if let Some(call) = &mut self.songcall.take() {
            let mut call = call.lock().await;

            match call.leave().await {
                Ok(()) => info!("left channel"),
                Err(e) => error!("failed to disconnect: {}", e),
            };

            if let Err(e) = self.stop().await {
                error!("Error stopping song: {:?}", e);
            }

            call.remove_all_global_events();
        }
    }


}


/* Possible mess for queue support */


pub struct TrackEndNotifier {
    pub ctx: Context,
}

#[async_trait]
impl VoiceEventHandler for TrackEndNotifier {

    // TODO: somehow make this a signaling thing so we don't have to await here
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        debug!("TrackEndNotifier fired");
        let mstate = mstate_get(&self.ctx).await.unwrap();
        let mut mstate = mstate.lock().await;

        if let Some(song) = &mstate.current_track.take() {
            mstate.history.push_front(song.clone());
            mstate.history.truncate(10); // TODO: config max history buffer length
        }
        else {
            debug!("TrackEnd handler somehow called with mstate.current_track = None");
        }

        match mstate.status {
            MusicStateStatus::Stopping | MusicStateStatus::Stopped => {
                debug!("stopping music play via event handler");
                return None; // We're done here
            }
            _ => {}
        };

        let ret = mstate.next().await;
        if let Ok(_) = ret {
            debug!("TrackEnd handler mstate.next() = {:?}", ret);
        }
        else if let Err(e) = ret {
            error!("{:?}", e);
        }

        if let Some(sticky) = &mstate.sticky {
            sticky.channel_id.edit_message(&self.ctx.http, sticky, |m| {
                m.set_embeds(vec![get_queuestate_embed(&mstate), get_nowplay_embed(&mstate)])
            }).await.unwrap();
        }

        None
    }
}