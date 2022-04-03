use std::{
    sync::Arc,
};

use serenity::{
    prelude::{
        Context,
    },
    model::id::{
        GuildId,
        ChannelId,
    },
    model::channel::Message,
    model::voice::VoiceState,
};

use songbird::{
    Event,
    EventContext,
    EventHandler as VoiceEventHandler,
    TrackEvent,
};

use async_trait::async_trait;

use log::*;
use music::player::MusicPlayer;
use music::Song;
use music::*;

use crate::get_mstate;
use crate::commands::helpers::*;
use crate::requester::*;

/// Struct to maintain discord's music player state
pub struct DiscordPlayer {
    songcall: Option<Arc<tokio::sync::Mutex<songbird::Call>>>,
    songhandler: Option<songbird::tracks::TrackHandle>,
    pub sticky: Option<Message>,
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
            sticky: None,
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

        let player = if let Some(p) = &mstate.player {
            p.lock().await
        } else { return None; };

        let embed = get_nowplay_embed(&self.ctx, &mstate).await;

        if let Some(sticky) = &player.sticky {
            sticky.channel_id.edit_message(&self.ctx.http, sticky, |m| {
                m.set_embeds(vec![get_queuestate_embed(&mstate), embed])
            }).await.unwrap();
        }

        None
    }
}


// Autoplay auto-rebalance userlists
pub async fn autoplay_voice_state_update(ctx: Context, guildid: Option<GuildId>, old: Option<VoiceState>, new: VoiceState) {
    let bot = ctx.cache.current_user_id().await;
    let guild = ctx.cache.guild(guildid.unwrap()).await.unwrap(); // TODO: don't unwrap here, play nice
    let bot_voice = guild.voice_states.get(&bot);

    if bot_voice.is_none() {
        debug!("bot is not in voice, ignoring voice state change");
        return;
    }

    get_mstate!(mut, mstate, ctx);
    if !mstate.autoplay.enabled {
        debug!("autoplay is not enabled, ignoring voice state change");
        return;
    }

    let bot_voice = bot_voice.unwrap();
    let bot_chan = bot_voice.channel_id.unwrap();

    // Bot has joined a channel
    if new.member.as_ref().unwrap().user.id == bot {
        if let Some(chan) = new.channel_id {
            // Clear out current autoplay users
            mstate.autoplay.disable_all_users();

            // ...and enable only users in this new channel
            for (uid, vs) in guild.voice_states.iter() {
                if *uid == bot || vs.channel_id.unwrap() != chan {
                    continue;
                }

                let user = if let &Some(mem) = &vs.member.as_ref() {
                    debug!("vs.member not None, using from there");
                    mem.user.clone()
                } else {
                    // Use the cache lookup based on key, because voicestate.member may be None.
                    if let Some(user) = ctx.cache.user(uid).await {
                        debug!("obtained user from cache");
                        user
                    }
                    // If cache fails for some reason, rely on making a direct http request
                    else if let Ok(user) = ctx.http.get_user(*uid.as_u64()).await {
                        debug!("obtained user from http call");
                        user
                    }
                    // This may also fail and we'll be sad here
                    else {
                        panic!("failed to obtain user {:?} from both cache and http", uid);
                    }

                };

                match mstate.autoplay.enable_user(&muid_from_userid(&user.id)) {
                    Ok(o) => debug!("enrolling user {}: {:?}", user.tag(), o),
                    Err(e) => debug!("did not enroll user {}: {:?}", user.tag(), e),
                };
            }
        }

        return;
    }

    // Connect-to-voice check, enroll if in correct channel
    if let Some(chan) = new.channel_id {
        if chan == bot_chan {
            let user = new.member.unwrap().user;
            match mstate.autoplay.enable_user(&muid_from_userid(&user.id)) {
                Ok(o) => debug!("enrolling user {}: {:?}", user.tag(), o),
                Err(e) => debug!("did not enroll {}: {:?}", user.tag(), e)
            }
            return;
        }
        else {
            debug!("received voice connect for another channel, trying disconnect checks");
        }
    }

    // Disconnect from voice checks, unenroll if old voice state matches bot's channel

    // new has already been checked, so this is a join for another channel likely?
    if old.is_none() {
        debug!("join received for another channel, ignoring!");
        return;
    }

    let old_vs = old.unwrap();
    if old_vs.channel_id.is_none() {
        warn!("not sure, apparently no old state channel, but also no new state channel?");
        return;
    }
    let chan = old_vs.channel_id.unwrap();

    if chan == bot_chan {
        let user = new.member.unwrap().user;
        match mstate.autoplay.disable_user(&muid_from_userid(&user.id)) {
            Ok(o) => debug!("unenrolling user {}: {:?}", user.tag(), o),
            Err(e) => debug!("did not unenroll {}: {:?}", user.tag(), e)
        }
    }
    else {
        debug!("received voice disconnect for another channel, ignorning");
    }

    return;
}
