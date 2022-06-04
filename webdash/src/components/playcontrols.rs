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


fn gen_callback(path: &'static str) -> Callback<MouseEvent> {
    let ahandle = use_async(async move {
        let resp = Request::post(format!("/api/{}", path).as_str())
            .json("").unwrap()
            .send().await.unwrap();
        if !resp.ok() {
            log::error!("bad response from backend: {:?}", resp);
            return Err(())
        }
        log::info!("resp = {:?}", resp);
        Ok(())
    });

    Callback::from(move |_| {
        ahandle.run();
    })
}

#[function_component(PlayControls)]
pub fn playcontrols() -> Html {

    let onprev = gen_callback("previous");
    let onskip = gen_callback("skip");

    let iconclass = "column is-flex is-2 is-justify-content-center controlicon";

    html! {
            <div class="columns is-centered is-mobile">
                <div class={iconclass} onclick={onprev}>
                    <skip_back::SkipBack />
                </div>
                // TODO: probably have this switch back/forth between play/pause based on state
                <div class={iconclass}>
                    <play::Play />
                </div>
                <div class={iconclass} onclick={onskip}>
                    <skip_forward::SkipForward />
                </div>
            </div>
    }
}
