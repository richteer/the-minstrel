use yew::{
    prelude::*,
    function_component,
    html,
};

use yew_feather::{
    skip_back,
    skip_forward,
    play,
};

use gloo_net::http::Request;
use yew_hooks::prelude::*;

use yew_toast::*;

use model::web::ReplyStatus;

fn gen_callback(path: &'static str, toast_string: Option<&'static str>, tdis: UseReducerDispatcher<ToastList>) -> Callback<MouseEvent> {
    let ahandle = use_async(async move {
        let resp = Request::post(format!("/api/{}", path).as_str())
            .json("").unwrap()
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

#[function_component(PlayControls)]
pub fn playcontrols() -> Html {
    let toast = use_context::<ToastContext>().unwrap();

    let onplay = {
        let toast = toast.clone();
        Callback::from(move |_| {
            toast.dispatch(toast_warning!("Play/Pause functionality not currently implemented".into()))
        })
    };

    let onprev = gen_callback("previous", Some("Enqueued previous track"), toast.dispatcher());
    let onskip = gen_callback("skip", None, toast.dispatcher());

    let iconclass = "column is-flex is-2 is-justify-content-center controlicon";

    html! {
            <div class="columns is-centered is-mobile">
                <div class={iconclass} onclick={onprev} title="Enqueue last played song">
                    <skip_back::SkipBack />
                </div>
                // TODO: probably have this switch back/forth between play/pause based on state
                <div class={iconclass} onclick={onplay} style="cursor: not-allowed" title="Play/Pause function currently unsupported">
                    <play::Play />
                </div>
                <div class={iconclass} onclick={onskip} title="Skip to the next track">
                    <skip_forward::SkipForward />
                </div>
            </div>
    }
}
