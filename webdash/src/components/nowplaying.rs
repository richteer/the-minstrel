use yew::{
    prelude::*,
    function_component,
    html,
};
use model::{
    Song,
};

use gloo_timers::callback::Interval;

use crate::components::helpers::duration_text;
use crate::components::requester::*;


pub enum NpMsg {
    IncrementNowplaying,
}

pub struct NowPlayingProgress {
    pub time: i64,
    pub interval: Option<Interval>,
}

#[derive(Properties, PartialEq)]
pub struct NowPlayingProgressProps {
    pub song: Song,
}

impl Component for NowPlayingProgress {
    type Message = NpMsg;
    type Properties = NowPlayingProgressProps;

    fn create(ctx: &Context<Self>) -> Self {
        let link = ctx.link().clone();
        let interval = Interval::new(1000, move || {
            link.send_message(Self::Message::IncrementNowplaying);
        });

        Self {
            time: 0,
            interval: Some(interval),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Self::Message::IncrementNowplaying => {
                self.time += 1;
                if self.time >= ctx.props().song.duration {
                    self.interval.take().unwrap().cancel();
                }

                true
            },
        }
    }

    fn changed(&mut self, ctx: &Context<Self>) -> bool {
        let link = ctx.link().clone();

        if let Some(interval) = self.interval.take() {
            interval.cancel();
        }

        self.time = 0;
        self.interval = Some(Interval::new(1000, move || {
            link.send_message(Self::Message::IncrementNowplaying);
        }));

        true
    }

    fn destroy(&mut self, _ctx: &Context<Self>) {
        if let Some(interval) = self.interval.take() {
            interval.cancel();
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let song = &ctx.props().song;
        html! {
            <div class="columns is-multiline">
                <div class="column is-full py-0"><span>{ format!("{} / {}", duration_text(self.time), duration_text(song.duration)) }</span></div>
                <div class="column is-full pt-1 pb-0"><progress class="progress is-primary" value={self.time.to_string()} max={song.duration.to_string()}/></div>
            </div>
        }
    }
}


#[derive(Properties, PartialEq)]
pub struct NowPlayingProps {
    pub song: Song,
}

#[function_component(NowPlaying)]
pub fn nowplaying(props: &NowPlayingProps) -> Html {
    let song = props.song.clone();

    html! {
        <>
        <div class="columns is-multiline is-centered">
            // TODO: probably break this up into more subcomponents
            // Album Art
            <div class="column is-full">
                <figure class="image">
                <a href={song.url.clone()} target="_blank" rel="noopener noreferrer">
                    <img src={song.thumbnail.clone()} alt="temp" />
                </a>
                </figure>
            </div>
            // Text
            <div class="column is-full">
                <div class="columns">
                    // Song Title/Artist
                    <div class="column ml-2 is-flex is-clipped">
                        <div class="columns is-multiline is-gapless" style="min-width: 0;"> // min-width needed here for proper clipping/ellipsing
                            <span class="column is-full songtitle songoverflow">{song.title.clone()}</span>
                            <span class="column is-full songartist songoverflow">{song.artist.clone()}</span>
                        </div>
                    </div>
                    // Requested by
                    <div class="column is-narrow is-flex is-flex-direction-column is-justify-content-end mr-2">
                        <RequesterTag requester={song.requested_by.clone()} />
                    </div>
                </div>
            </div>
            // Progress bar
            <div class="column is-full"><NowPlayingProgress song={song.clone()}/></div>
        </div>
        </>
    }
}