use yew::{
    prelude::*,
    html
};
use gloo_net::http::Request;
use gloo_timers::callback::Interval;
use webdata::{
    MinstrelWebData
};

mod components;
use components::*;

enum Msg {
    Data(MinstrelWebData),
}

struct Dash {
    data: Option<MinstrelWebData>,
}

async fn update_data() -> Msg {
    let resp = Request::get("http://127.0.0.1:3030").send().await.unwrap();
    let json = resp.json::<MinstrelWebData>().await.unwrap();
    Msg::Data(json)
}


impl Component for Dash {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        ctx.link().send_future(update_data());

        // TODO: don't rely on polling, use SSE or websockets
        let link = ctx.link().clone();
        Interval::new(1000 * 10, move || {
            link.send_future(update_data())
        }).forget();

        Self {
            data: None,
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
            }
        }
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        if let Some(data) = self.data.clone() {
            let np = data.current_track.unwrap();
            html! {
                <div class="container">
                    <div class="nowplaying">
                        <SongRow song={np.clone()} />
                    </div>
                    <div>
                        { for data.upcoming.iter().map(|e| {
                                html! {
                                <div class="upcomingitem">
                                    <SongRow song={e.clone()} />
                                </div>
                                }
                            })
                        }
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