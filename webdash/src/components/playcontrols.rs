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

    html! {
        <div class="column is-full">
            <div class="columns is-centered is-mobile">
                <div class="column is-flex is-justify-content-end">
                    <div class="controlicon" onclick={onprev}>
                        <skip_back::SkipBack />
                    </div>
                </div>
                // TODO: probably have this switch back/forth between play/pause based on state
                <div class="column is-3 is-flex is-justify-content-center">
                    <div class="controlicon">
                        <play::Play />
                    </div>
                </div>
                <div class="column is-flex is-justify-content-start">
                    <div class="controlicon" onclick={onskip}>
                        <skip_forward::SkipForward />
                    </div>
                </div>
            </div>
        </div>
    }
}
