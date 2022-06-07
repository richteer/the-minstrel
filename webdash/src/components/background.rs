use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct BackgroundImageProps {
    pub url: String,
}

#[function_component(BackgroundImage)]
pub fn background_image(props: &BackgroundImageProps) -> Html {
    let url = props.url.clone();

    html! {
        <div class="background background-image" style={format!("background-image: url(\"{}\")", url)}/>
    }

}