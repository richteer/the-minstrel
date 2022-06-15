use serde::Serialize;
use yew::{
    prelude::*,
    function_component,
    html,
};

use yew_feather::{
    skip_back,
    skip_forward,
    play,
    square,
};

use gloo_net::http::Request;
use yew_hooks::prelude::*;

use yew_toast::*;

use model::{web::ReplyStatus, MusicStateStatus};

#[derive(Serialize, Clone)]
struct NoBody {}

fn gen_callback<T: Serialize + Clone + 'static>(path: &'static str, body: T, toast_string: Option<&'static str>, tdis: UseReducerDispatcher<ToastList>) -> Callback<MouseEvent> {
    let body = body.clone();
    let ahandle = use_async(async move {
        let resp = Request::post(format!("/api/{}", path).as_str())
            .json(&body).unwrap()
            .send().await.unwrap();
        if !resp.ok() {
            let resp = resp.json::<ReplyStatus>().await;
            if let Ok(msg) = resp {
                tdis.dispatch(toast_error!(msg.error));
            } else {
                log::error!("bad response from backend: {:?}", resp);
                tdis.dispatch(toast_error!("Bad data from API, check console".into()));
            }

            return Err(())
        }

        if let Some(toast) = toast_string {
            tdis.dispatch(toast_info!(toast.into()));
        }

        Ok(())
    });

    Callback::from(move |_| {
        ahandle.run();
    })
}

#[derive(Properties, PartialEq)]
pub struct PlayControlsProps {
    pub status: MusicStateStatus,
}

#[function_component(PlayControls)]
pub fn playcontrols(props: &PlayControlsProps) -> Html {
    let toast = use_context::<ToastContext>().unwrap();

    let onprev = gen_callback("previous", NoBody{}, Some("Enqueued previous track"), toast.dispatcher());
    let onskip = gen_callback("skip", NoBody{}, None, toast.dispatcher());
    let onstop = gen_callback("stop", NoBody{}, None, toast.dispatcher());
    let onplay = gen_callback("start", NoBody{}, None, toast.dispatcher());

    let iconclass = "column is-flex is-2 is-justify-content-center controlicon";

    html! {
            <div class="columns is-centered is-mobile">
                <div class={iconclass} onclick={onprev} title="Enqueue last played song">
                    <skip_back::SkipBack />
                </div>
                // TODO: probably have this switch back/forth between play/pause based on state
                {
                    match props.status {
                        MusicStateStatus::Playing => html! {
                            <div class={iconclass} onclick={onstop} title="Stop Playback">
                                <square::Square />
                            </div>
                        },
                        MusicStateStatus::Stopping | MusicStateStatus::Idle |
                        MusicStateStatus::Stopped => html! {
                                <div class={iconclass} onclick={onplay} title="Start Playback">
                                    <play::Play />
                                </div>
                            },
                        _ => html! {}
                    }
                }

                <div class={iconclass} onclick={onskip} title="Skip to the next track">
                    <skip_forward::SkipForward />
                </div>
            </div>
    }
}
