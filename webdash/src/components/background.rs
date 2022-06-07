use yew::prelude::*;
use yew_hooks::use_previous;

#[derive(Properties, PartialEq)]
pub struct BackgroundImageProps {
    pub url: String,
}

#[function_component(BackgroundImage)]
pub fn background_image(props: &BackgroundImageProps) -> Html {
    let url = props.url.clone();

    let previous_url = use_previous(url.clone());

    if *previous_url != url {
        let prev = &*previous_url.previous();

        html! {
            <>
            // TODO: Figure out a way to clear the old div when it fades away. Keeping it around is bad for performance probably.
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