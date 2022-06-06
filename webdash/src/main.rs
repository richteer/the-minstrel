use yew::{
    prelude::*,
    html
};

use model::{MinstrelWebData, MinstrelBroadcast};

mod components;
use components::*;

use yew_hooks::use_web_socket_with_options;


#[function_component(FDash)]
pub fn fdash() -> Html {
    let data: UseStateHandle<Option<MinstrelWebData>> = use_state(|| None);

    let toastbridge = use_toast();

    let _ws = {
        let data = data.clone();

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

        let tb_mess = toastbridge.clone();
        let tb_err = toastbridge.clone();

        use_web_socket_with_options(wsurl, yew_hooks::UseWebSocketOptions {
            //onopen:(),
            onmessage: Some(Box::new(move |message| {
                match serde_json::from_str::<MinstrelBroadcast>(&message).unwrap() {
                    MinstrelBroadcast::MusicState(newdata) => data.set(Some(newdata)),
                    MinstrelBroadcast::Error(err) =>{
                        log::info!("error from backend: {}", err);
                        tb_mess.send(ToastType::Error(err));
                    }
                };

            })),
            onmessage_bytes: Some(Box::new(move |_| {
                log::error!("received bytes from Ws for some reason");
            })),
            onerror: Some(Box::new(move |event|{
                log::error!("WS error: {:?}", event);
                tb_err.send(ToastType::Error(format!("Websocket lost connection")))
            })),
            //onclose: (),
            // TODO: probably figure out sane reconnect limit/intervals
            reconnect_limit: Some(10_000),
            reconnect_interval: Some(10_000),
            //manual: (),
            //protocols: ()
            ..Default::default()
        })
    };

    html! {
        <div class="container">
        <ToastTray />

        if let Some(data) = &*data.clone() {
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
                    <SongListTabs data={data.clone()} />
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

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::start_app::<FDash>();
}