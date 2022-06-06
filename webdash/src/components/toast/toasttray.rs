use std::collections::BTreeMap;

use yew::{
    prelude::*,
    html
};
use yew_agent::{Bridge, Bridged};

use gloo_timers::callback::Timeout;

use super::ToastType;
use super::ToastBus;


#[derive(Properties, PartialEq)]
pub struct ToastProps {
    message: ToastType,
    fade: bool,
    ondeath: Callback<()>,
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
        let ondeath = props.ondeath.clone();
        Callback::from(move |_| ondeath.emit(()))
    };

    let class = {
        let fade = if props.fade {
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

struct InternalToast {
    pub _timeout: Timeout,
    pub fade: bool,
    pub toast: ToastType,
}


pub struct ToastTray {
    toasts: BTreeMap<usize, InternalToast>,
    counter: usize, // increment for each toast, probably a bad idea in the long run
    _producer: Box<dyn Bridge<ToastBus>>,
}

pub enum Msg {
    Add(ToastType),
    Fade(usize),
    Delete(usize),
}

impl Component for ToastTray {

    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        Self {
            toasts: BTreeMap::new(),
            counter: 0,
            _producer: ToastBus::bridge(ctx.link().callback(Msg::Add)),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Add(toast) => {
                // TODO: Determine if there's a much cleaner way to handle this kind of fade->deletion
                // Create a timeout to delete the toast after a certain amount of time
                let _timeout = {
                    let link = ctx.link().clone();
                    let tid = self.counter;

                    // First timeout, to start fading the message...
                    Timeout::new(5_000, move || {
                    link.send_message(Self::Message::Fade(tid));

                        // Second, internal timeout to actually issue the deletion.
                        Timeout::new(300, move || {
                            link.send_message(Self::Message::Delete(tid));
                        }).forget();
                    })
                };

                self.toasts.insert(self.counter, InternalToast{ _timeout, fade: false, toast});
                self.counter += 1;
            },
            Msg::Fade(index) => {
                if let Some(int_toast) = self.toasts.get_mut(&index) {
                    int_toast.fade = true;
                } else {
                    log::warn!("fade called on non-existing toast, possibly a bug");
                }
            },
            Msg::Delete(index) => {
                if self.toasts.remove(&index).is_none() {
                    log::debug!("attempted to delete toast {} it vanished", index);
                }
            },
        };

        true
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        html! {
            <div class="toasttray">
                {
                    for self.toasts.iter().rev()
                        .map(|(i,int_toast)| {
                            let message = int_toast.toast.clone();
                            let i = *i;
                            html! {
                                <ToastPopup {message} fade={int_toast.fade} ondeath={_ctx.link().callback(move |_|
                                    Msg::Delete(i)
                                )}/>
                            }
                        })
                }
            </div>
        }
    }
}