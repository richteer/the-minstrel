use yew::{
    prelude::*,
    html
};

use gloo_timers::callback::Timeout;
use yew_hooks::use_is_first_mount;

use super::*;


#[derive(Properties, PartialEq)]
pub struct ToastProps {
    message: ToastType,
    fading: bool,
    tid: usize,
    dispatch: UseReducerDispatcher<ToastList>,
}

#[function_component(ToastPopup)]
pub fn toast_popup(props: &ToastProps) -> Html {

    let (string, flavor) = match &props.message {
        ToastType::Info(string)    => (string.clone(), "is-info"),
        ToastType::Success(string) => (string.clone(), "is-success"),
        ToastType::Warning(string) => (string.clone(), "is-warning"),
        ToastType::Error(string)   => (string.clone(), "is-danger"),
    };


    let onclick = {
        let tid = props.tid;
        let dispatch = props.dispatch.clone();
        Callback::from(move |_| dispatch.dispatch(ToastAction::Delete(tid)))
    };

    if use_is_first_mount() {
        let tid = props.tid;

        let dispatch = props.dispatch.clone();
        Timeout::new(5_000, move|| {
            dispatch.dispatch(ToastAction::Fade(tid));
            Timeout::new(300, move || {
                dispatch.dispatch(ToastAction::Delete(tid));
            }).forget();
        }).forget();
    }

    let class = {
        let fade = if props.fading {
            "toastclosing"
        } else { "" };

        format!("notification toast {} {} mb-4", fade, flavor)
    };

    html! {
        <div {class}>
            <div class="delete" {onclick}/>
            { string }
        </div>
    }
}



#[function_component(ToastTray)]
pub fn toast_try_helper() -> Html {
    let toasts = use_context::<ToastContext>().unwrap();

    html! {
        <div class="toasttray">
            {
                for toasts.toasts.iter().rev()
                .map(|(tid, (int_toast, fading))| {
                    let dispatch = toasts.dispatcher();
                    let message = int_toast.clone();
                    let fading = fading.clone();
                    let tid = tid.clone();
                    html! {
                        <ToastPopup key={tid} {message} {fading} {tid} {dispatch} />
                    }})
            }
        </div>
    }
}