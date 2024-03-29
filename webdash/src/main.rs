use yew::{
    prelude::*,
    html
};

use model::{MinstrelWebData, MinstrelBroadcast};

mod components;
use components::*;

use yew_hooks::use_websocket_with_options;
use yew_toast::*;


#[function_component(FDash)]
pub fn fdash() -> Html {
    // TODO: fetch info on page load
    let data: UseStateHandle<Option<MinstrelWebData>> = use_state(|| None);

    let toastlist = use_reducer(|| ToastList::new());
    let userinfo = use_reducer(|| LoginStatus { current_user: None });

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

        let tb_mess = toastlist.dispatcher();
        let tb_err = toastlist.dispatcher();

        use_websocket_with_options(wsurl, yew_hooks::UseWebSocketOptions {
            //onopen:(),
            onmessage: Some(Box::new(move |message| {
                match serde_json::from_str::<MinstrelBroadcast>(&message).unwrap() {
                    MinstrelBroadcast::MusicState(newdata) => data.set(Some(newdata)),
                    MinstrelBroadcast::Error(err) =>{
                        log::info!("error from backend: {}", err);
                        tb_mess.dispatch(toast_error!(err));
                    }
                };

            })),
            onmessage_bytes: Some(Box::new(move |_| {
                log::error!("received bytes from Ws for some reason");
            })),
            onerror: Some(Box::new(move |event|{
                log::error!("WS error: {:?}", event);
                // TODO: probably handle different types of errors here
                tb_err.dispatch(toast_error!("Websocket lost connection".into()))
            })),
            //onclose: (),
            // TODO: probably figure out sane reconnect limit/intervals
            reconnect_limit: Some(10),
            reconnect_interval: Some(10_000),
            //manual: (),
            //protocols: ()
            ..Default::default()
        })
    };

    html! {
        <div class="container">
        <div class="background-noise" />
        <ContextProvider<UserContext> context={userinfo}>
        <ContextProvider<ToastContext> context={toastlist}>
        <ToastTray />

        if let Some(data) = &*data.clone() {
        // m-0 set to override the negative margins set by columns
        //  no idea why columns is like that, but centers the main div to the container->viewport
            <div class="columns is-vcentered m-0 is-text-shadowed">
                <div class="column is-half">
                    <div class="columns is-multiline is-centered">
                    {
                        if let Some(np) = &data.current_track {
                            html! {
                                <>
                                <BackgroundImage url={np.song.thumbnail.clone()} />
                                    <div class="column is-full">
                                        <NowPlaying song={np.clone()} progress={data.song_progress}/>
                                    </div>

                                </>
                            }
                        } else {
                            html! {
                            <span><i>{"Nothing currently playing"}</i></span>
                            }
                        }
                    }
                    <IsLoggedIn>
                        <div class="column is-full">
                            <PlayControls status={data.status.clone()} ap_enabled={data.ap_enabled}/>
                        </div>
                    </IsLoggedIn>
                    </div>
                </div>
                <div class="column container is-half fullheight">
                    <SongListTabs data={data.clone()} />
                    // TODO: consider a navbar, or somewhere better to put this
                    <Login />
                </div>
            </div>
        } else {
            <div>
            { "Nothing currently playing" }
            </div>
        }
        </ContextProvider<ToastContext>>
        </ContextProvider<UserContext>>
        </div>
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    // yew::start_app::<FDash>();
    yew::Renderer::<FDash>::new().render();
}