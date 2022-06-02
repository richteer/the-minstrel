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


#[function_component(PlayControls)]
pub fn playcontrols() -> Html {

    let skip = use_async(async move {
        log::error!("actually called: {}", "skip");
        let resp = Request::post(format!("/api/{}", "skip").as_str())
            .json("").unwrap()
            .send().await.unwrap();
        if !resp.ok() {
            log::error!("bad response from backend: {:?}", resp);
            return Err(())
        }
        log::info!("resp = {:?}", resp);
        Ok(())
    });

    let onclick = {
        let skip = skip.clone();
        Callback::from(move |_| {
            skip.run();
        })
    };

    html! {
        <div class="column is-full">
            <div class="columns is-centered is-mobile">
                <div class="column is-2 is-flex">
                    <div class="controlicon">
                        <skip_back::SkipBack />
                    </div>
                </div>
                // TODO: probably have this switch back/forth between play/pause based on state
                <div class="column is-2 is-flex">
                    <div class="controlicon">
                        <play::Play />
                    </div>
                </div>
                <div class="column is-2 is-flex">
                    <div class="controlicon" onclick={onclick}>
                        <skip_forward::SkipForward />
                    </div>
                </div>
            </div>
        </div>
    }
}
