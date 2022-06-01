use yew::{
    prelude::*,
    function_component,
    html,
    Children,
};
use model::{
    Song, MinstrelWebData,
};

use gloo_timers::callback::Interval;

use yew_feather::{
    external_link,
    skip_back,
    skip_forward,
    play,
};


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
                            <external_link::ExternalLink color="white" size={if np { "48"} else { "28" }}/>
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
            <div class="columns">
                <div class="column is-narrow"><span>{ format!("{} / {}", duration_text(self.time), duration_text(song.duration)) }</span></div>
                <div class="column"><progress value={self.time.to_string()} max={song.duration.to_string()}/></div>
            </div>
        }
    }
}

#[derive(Properties, PartialEq)]
pub struct SongListTabsProps {
    pub data: MinstrelWebData,
}

#[derive(PartialEq, Copy, Clone)]
enum ActiveTab {
    ComingUp,
    History,
}

#[function_component(SongListTabs)]
pub fn songlisttabs(props: &SongListTabsProps) -> Html {
    let active = use_state(|| ActiveTab::ComingUp );
    let comingup_onclick = {
        let active = active.clone();
        Callback::from(move |_| { active.set(ActiveTab::ComingUp) })
    };

    let history_onclick = {
        let active = active.clone();
        Callback::from(move |_| active.set(ActiveTab::History))
    };

    fn get_class(active: ActiveTab, target: ActiveTab, content: bool) -> String {
        match (content, active == target) {
            (false, true)  => String::from("tabs is-active"),
            (false, false) => String::from("tabs"),
            (true,  true)  => String::from("is-active"),
            (true,  false) => String::from("is-hidden"),
        }
    }

    html! {
        <div class="tabview">
        <div class="tabs">
            <ul>
                <li><a class={get_class(*active, ActiveTab::ComingUp, false)} onclick={comingup_onclick}>{"Coming up"}</a></li>
                <li><a class={get_class(*active, ActiveTab::History, false)} onclick={history_onclick}>{"History"}</a></li>
            </ul>
        </div>
        <div class="songlist">
            <div class={get_class(*active, ActiveTab::ComingUp, true)}>
            {
                if props.data.upcoming.is_empty() {
                    html! {<i>{"Nothing coming up"}</i>}
                } else {
                    html! {
                    {
                        for props.data.upcoming.iter().map(|e| {
                            html! {
                            <SongRow song={e.clone()} />
                            }
                        })
                    }
                    }
                }
            }
            </div>
            <div class={get_class(*active, ActiveTab::History, true)}>
            {
                if props.data.history.is_empty() {
                    html! {<i>{"History is empty"}</i>}
                } else {
                    html! {
                    {
                        for props.data.history.iter().map(|e| {
                            html! {
                                <SongRow song={e.clone()} />
                            }
                        })
                    }
                    }
                }
            }
            </div>
        </div>
        </div>
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
                    <div class="column is-full"><SongText song={song.clone()}/></div>
                    <div class="column is-full"><NowPlayingProgress song={song.clone()}/></div>
                </div>
            </div>
            <div class="column is-full">
                <div class="columns is-centered">
                    <div class="column is-2 is-flex" style="justify-content: start">
                        <div>
                            <skip_back::SkipBack />
                        </div>
                    </div>
                    // TODO: probably have this switch back/forth between play/pause based on state
                    <div class="column is-2 is-flex" style="justify-content: center">
                        <div>
                            <play::Play />
                        </div>
                    </div>
                    <div class="column is-2 is-flex" style="justify-content: end;">
                        <div>
                            <skip_forward::SkipForward />
                        </div>
                    </div>
                </div>
            </div>
        </div>
        </>
    }
}