use yew::{
    prelude::*,
    function_component,
    html,
};

use crate::components::UserContext;

#[derive(Properties, PartialEq)]
pub struct IsLoggedInProps {
    pub children: Children,
}

#[function_component(IsLoggedIn)]
pub fn is_logged_in(props: &IsLoggedInProps) -> Html {
    let usercontext = use_context::<UserContext>().unwrap();

    if usercontext.current_user.is_some(){
        html! {
            <>
                { for props.children.iter() }
            </>
        }
    } else {
        html! {}
    }

}