use yew::{
    prelude::*,
    function_component,
    html,
};

use model::Requester;

#[allow(dead_code)]
#[derive(PartialEq)]
pub enum RequesterSize {
    Small,
    Large,
}

#[derive(Properties, PartialEq)]
pub struct RequesterTagProps {
    pub requester: Requester,
    pub size: Option<RequesterSize>,
}

#[function_component(RequesterTag)]
pub fn requester_tag(props: &RequesterTagProps) -> Html {
    let iconsize = match props.size {
        None | Some(RequesterSize::Small) => "is-32x32",
        Some(RequesterSize::Large) => "is-48x48",
    };

    let namesize = match props.size {
        None | Some(RequesterSize::Small) => "is-size-6",
        Some(RequesterSize::Large) => "is-size-5",
    };

    html! {
        <div class="columns is-vcentered is-gapless is-mobile">
            <div class={format!("column mr-2 {}", namesize)}>
                { props.requester.displayname.clone() }
            </div>
            <div class="column">
                <figure class={format!("image {}", iconsize)}>
                    <img class="is-rounded" src={ props.requester.icon.clone() } alt="temp" />
                </figure>
            </div>
        </div>
    }
}