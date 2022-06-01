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

use model::MinstrelWebData;

mod components;
use components::*;

mod wsbus;
use wsbus::WsBus;

pub enum Msg {
    Data(MinstrelWebData),
}

struct Dash {
    data: Option<MinstrelWebData>,
    _recv: Box<dyn Bridge<WsBus>>,
}

async fn update_data() -> Msg {
    // TODO: consider using location/origin here too, might be needed for proper hosting
    let resp = Request::get("/api").send().await.unwrap();
    let json = resp.json::<MinstrelWebData>().await.unwrap();
    Msg::Data(json)
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
        let ws = WebSocket::open(&String::from(wsurl)).unwrap();
        let (_, mut ws_rx) = ws.split();

        // This needs to be called before the bridge call for some unknown reason.
        let mut wsbus = WsBus::dispatcher();

        // "Connect" to our websocket bus, keep this in scope else it falls out and disappears from the universe
        // TODO: update this when/if the websocket sends anything other than full mstates
        let recv = WsBus::bridge(ctx.link().callback(|data| Msg::Data(data)));

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
        }
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        if let Some(data) = self.data.clone() {
            html! {
            <div class="container">
                <div class="columns is-vcentered">
                    <div class="column is-half">
                    {
                        if let Some(np) = &data.current_track {
                            html! {
                            <NowPlaying song={np.clone()}/>
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
            </div>
            }
        } else {
            html! {
                <div>
                    { "Nothing currently playing" }
                </div>
            }
        }
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::start_app::<Dash>();
}