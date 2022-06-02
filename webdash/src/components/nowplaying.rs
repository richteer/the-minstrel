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
use crate::components::songrow::*;


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
            <div class="columns">
                <div class="column is-narrow"><span>{ format!("{} / {}", duration_text(self.time), duration_text(song.duration)) }</span></div>
                <div class="column"><progress value={self.time.to_string()} max={song.duration.to_string()}/></div>
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
            <div class="column is-full">
                <figure class="image">
                <a href={song.url.clone()} target="_blank" rel="noopener noreferrer">
                    <img src={song.thumbnail.clone()} alt="temp" />
                </a>
                </figure>
            </div>
            <div class="column is-full">
                <div class="columns is-multiline">
                    // TODO: don't depend on SongText here probably, should be made private and internal to songrow
                    <div class="column is-full"><SongText song={song.clone()}/></div>
                    <div class="column is-full"><NowPlayingProgress song={song.clone()}/></div>
                </div>
            </div>
        </div>
        </>
    }
}