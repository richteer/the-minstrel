use yew::{
    prelude::*,
    function_component,
    html,
};

use model::Requester;

#[derive(Properties, PartialEq)]
pub struct RequesterTagProps {
    pub requester: Requester,
}

#[function_component(RequesterTag)]
pub fn requester_tag(props: &RequesterTagProps) -> Html {
    html! {
        <div class="columns is-vcentered is-gapless">
            <div class="column mr-2">
                { props.requester.displayname.clone() }
            </div>
            <div class="column">
                <figure class="image is-32x32">
                    <img class="is-rounded" src={ props.requester.icon.clone() } alt="temp" />
                </figure>
            </div>
        </div>
    }
}