use yew::{
    prelude::*,
    function_component,
    html,
    Children,
};
use webdata::{
    Song,
};

use gloo_timers::callback::Interval;

fn duration_text(dur: i64) -> String {
    let min = dur / 60;
    let secs = dur % 60;

    format!("{}:{:02}", min, secs)
}

#[derive(Properties, PartialEq)]
pub struct SongTextProps {
    pub song: Song,
    pub children: Option<Children>,
}

#[function_component(SongText)]
pub fn song_text(props: &SongTextProps) -> Html {
    let song = &props.song;
    html! {
        <div class="songdata">
            <span class="songtitle songoverflow">{song.title.clone()}</span>
            <span class="songartist songoverflow">{song.artist.clone()}</span>
            {
                // TODO: there's probably a cleaner way to do this
                if let Some(children) = &props.children {
                    html! {
                        <>
                            { children.clone() }
                        </>
                    }
                } else {
                    html! {}
                }
            }
        </div>
    }
}


#[derive(Properties, PartialEq)]
pub struct SongRowProps {
    pub song: Song,
    pub nowplaying: Option<bool>,
}

#[function_component(SongRow)]
pub fn song_row(props: &SongRowProps) -> Html {
    let song = &props.song;
    let np = if let Some(np) = props.nowplaying {
        np
    } else { false };

    html! {
        <div class={ if np { "nowplaying" } else {"upcomingitem"} }>
            <div class="songicon">
                <a href={song.url.clone()} target="_blank" rel="noopener noreferrer">
                    <img src={song.thumbnail.clone()} alt="temp" />
                    <div class="songicon-overlay">
                        <div class="songicon-overlay-content">
                            <img src="data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSIyNCIgaGVpZ2h0PSIyNCIgdmlld0JveD0iMCAwIDI0IDI0IiBmaWxsPSJub25lIiBzdHJva2U9ImN1cnJlbnRDb2xvciIgc3Ryb2tlLXdpZHRoPSIyIiBzdHJva2UtbGluZWNhcD0icm91bmQiIHN0cm9rZS1saW5lam9pbj0icm91bmQiIGNsYXNzPSJmZWF0aGVyIGZlYXRoZXItZXh0ZXJuYWwtbGluayI+PHBhdGggZD0iTTE4IDEzdjZhMiAyIDAgMCAxLTIgMkg1YTIgMiAwIDAgMS0yLTJWOGEyIDIgMCAwIDEgMi0yaDYiPjwvcGF0aD48cG9seWxpbmUgcG9pbnRzPSIxNSAzIDIxIDMgMjEgOSI+PC9wb2x5bGluZT48bGluZSB4MT0iMTAiIHkxPSIxNCIgeDI9IjIxIiB5Mj0iMyI+PC9saW5lPjwvc3ZnPg=="/>
                        </div>
                    </div>
                </a>
            </div>
            <SongText song={song.clone()}>
                {
                    if np {
                        html! {
                            <NowPlayingProgress song={song.clone()}/>
                        }
                    } else {
                        html! {
                            <div><span class="songduration">{duration_text(song.duration)}</span></div>
                        }
                    }
                }
            </SongText>
            <div class="user">
                <span class="username">{ song.requested_by.displayname.clone() }</span>
                <img src={ song.requested_by.icon.clone() } alt="temp" />
            </div>
        </div>
    }
}


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
            <>
                <span>{ format!("{} / {}", duration_text(self.time), duration_text(song.duration)) }</span>
                <progress value={self.time.to_string()} max={song.duration.to_string()}/>
            </>
        }
    }
}
