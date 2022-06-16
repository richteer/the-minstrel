use yew::prelude::*;
use yew_hooks::{
    use_previous,
    use_update,
    use_timeout
};

#[derive(Properties, PartialEq)]
pub struct BackgroundImageProps {
    pub url: String,
}

#[function_component(BackgroundImage)]
pub fn background_image(props: &BackgroundImageProps) -> Html {
    let url = props.url.clone();

    let previous_url = use_previous(url.clone());

    // Trick prev == current by forcing a re-render after the transition time
    let update = use_update();
    let timeout = use_timeout(move || {
        update();
    }, 2020); // Keep this in sync with .old-background-image's transition speed

    timeout.cancel(); // Cancel the timeout immediately, we only want to re-render...

    if *previous_url != url {
        let prev = &*previous_url.previous();

        timeout.reset(); // ...here, X millis after we render both of these

        html! {
            <>
            <div key={url.clone()} class="background background-image" style={format!("background-image: url(\"{}\")", url.clone())}/>
            <div key={prev.clone()} class="background background-image old-background-image" style={format!("background-image: url(\"{}\")", prev.clone())}/>
            </>
        }
    } else {
        html! {
            <div key={url.clone()} class="background background-image" style={format!("background-image: url(\"{}\")", url)}/>
        }
    }
}