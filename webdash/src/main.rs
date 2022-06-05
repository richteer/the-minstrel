use yew::{
    prelude::*,
    html
};

use yew_agent::{
    Dispatched,
    Bridge,
    Bridged,
};

use gloo_net::http::Request;
use gloo_net::websocket::{
    futures::WebSocket,
    Message,
};
use futures_util::StreamExt;
use wasm_bindgen_futures::spawn_local;

use model::{MinstrelWebData, MinstrelBroadcast};

mod components;
use components::*;

mod wsbus;
use wsbus::WsBus;

pub enum Msg {
    Data(MinstrelWebData),
    SetError(String),
    ClearError,
}

struct Dash {
    data: Option<MinstrelWebData>,
    error: Option<String>,
    _recv: Box<dyn Bridge<WsBus>>,
}

async fn update_data() -> Msg {
    // TODO: consider using location/origin here too, might be needed for proper hosting
    let resp = Request::get("/api").send().await.unwrap();

    match resp.json::<MinstrelWebData>().await {
        Ok(data) => Msg::Data(data),
        Err(e) => Msg::SetError(format!("Error fetching data: {}", e))
    }
}


impl Component for Dash {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        ctx.link().send_future(update_data());

        // TODO: have some method of reconnecting to the websocket if connection lost
        let window = web_sys::window().unwrap();
        let protocol = window.location().protocol();
        let protocol = match protocol {
            Ok(p) => match p.as_str() {
                "https:" => Some("wss:"),
                "http:" => Some("ws:"),
                _ => {
                    log::error!("unknown protocol reported by window.location() = {}", p);
                    None
                },
            },
            _ => {
                log::error!("could not get protocol from window.location() = {:?}", protocol);
                None
            },
        }.unwrap();

        let wsurl = format!("{}//{}/ws", protocol, window.location().host().unwrap());
        log::debug!("connecting to websocket at {}", &wsurl);
        let ws = WebSocket::open(&wsurl).unwrap();
        let (_, mut ws_rx) = ws.split();

        // This needs to be called before the bridge call for some unknown reason.
        let mut wsbus = WsBus::dispatcher();

        // "Connect" to our websocket bus, keep this in scope else it falls out and disappears from the universe
        let recv = WsBus::bridge(ctx.link().callback(|data|
            match data {
                MinstrelBroadcast::MusicState(data) => Msg::Data(data),
                MinstrelBroadcast::Error(err) => Msg::SetError(err),
            }
        ));

        // Listen on the websocket for data, pump it through the WsBus to send it back to us as MinstrelWebData
        spawn_local(async move {
            while let Some(msg) = ws_rx.next().await {
                match msg {
                    Ok(Message::Text(data)) => {
                        wsbus.send(data);
                    },
                    Ok(Message::Bytes(_)) =>
                        log::error!("received unexpected binary data from the websocket"),
                    Err(e) => log::error!("error reading from websocket: {:?}", e),
                };
            }
        });

        Self {
            data: None,
            error: None,
            _recv: recv,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Data(json) => {
                if let Some(data) = &self.data {
                    if data == &json {
                        log::debug!("fetched data did not change, ignoring");
                        return false
                    }
                }

                log::debug!("updating data");
                self.data = Some(json);

                true
            },
            Msg::SetError(err) => {
                self.error = Some(err);
                true
            }
            Msg::ClearError => {
                self.error = None;
                true
            },
        }
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
            html! {
            <div class="container">
                {
                    if let Some(error) = &self.error {
                        let onclick = _ctx.link().callback(|_| Msg::ClearError);
                        let scope = _ctx.link().clone();
                        gloo_timers::callback::Timeout::new(10_000, move || {
                            scope.send_message(Msg::ClearError);
                        }).forget();
                        html! {
                            <div class="notification is-danger errornotif">
                                <div class="delete" {onclick}/>
                                {error}
                            </div>
                        }
                    } else {
                        html! {}
                    }
                }
                if let Some(data) = self.data.clone() {
                // m-0 set to override the negative margins set by columns
                //  no idea why columns is like that, but centers the main div to the container->viewport
                    <div class="columns is-vcentered m-0">
                        <div class="column is-half">
                        {
                            if let Some(np) = &data.current_track {
                                html! {
                                    <>
                                    <div class="columns is-multiline is-centered">
                                        <div class="column is-full">
                                            <NowPlaying song={np.clone()}/>
                                        </div>
                                        <div class="column is-full">
                                            <PlayControls/>
                                        </div>
                                    </div>
                                    </>
                                }
                            } else {
                                html! {
                                <span><i>{"Nothing currently playing"}</i></span>
                                }
                            }
                        }
                        </div>
                        <div class="column is-half fullheight">
                            <SongListTabs data={data} />
                        </div>
                    </div>
                } else {
                    <div>
                    { "Nothing currently playing" }
                    </div>
                }
            </div>
        }
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::start_app::<Dash>();
}