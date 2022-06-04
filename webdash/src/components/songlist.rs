use yew::{
    prelude::*,
    function_component,
    html,
};
use model::{
    MinstrelWebData,
};

use crate::components::songrow::*;


#[derive(Properties, PartialEq)]
pub struct SongListTabsProps {
    pub data: MinstrelWebData,
}

#[derive(PartialEq, Copy, Clone)]
enum ActiveTab {
    ComingUp,
    History,
}

#[function_component(SongListTabs)]
pub fn songlisttabs(props: &SongListTabsProps) -> Html {
    let active = use_state(|| ActiveTab::ComingUp );
    let comingup_onclick = {
        let active = active.clone();
        Callback::from(move |_| { active.set(ActiveTab::ComingUp) })
    };

    let history_onclick = {
        let active = active.clone();
        Callback::from(move |_| active.set(ActiveTab::History))
    };

    fn get_class(active: ActiveTab, target: ActiveTab, content: bool) -> String {
        match (content, active == target) {
            (false, true)  => String::from("tabs is-active has-text-weight-bold"),
            (false, false) => String::from("tabs"),
            (true,  true)  => String::from("is-active"),
            (true,  false) => String::from("is-hidden"),
        }
    }

    html! {
        <div class="tabview">
        <div class="tabs">
            <ul>
                <li><a class={get_class(*active, ActiveTab::ComingUp, false)} onclick={comingup_onclick}>{"Coming up"}</a></li>
                <li><a class={get_class(*active, ActiveTab::History, false)} onclick={history_onclick}>{"History"}</a></li>
            </ul>
        </div>
        <div class="songlist">
            <div class={get_class(*active, ActiveTab::ComingUp, true)}>
            {
                if props.data.queue.is_empty() && props.data.upcoming.is_empty() {
                    html! {<i>{"Nothing coming up"}</i>}
                } else {
                    html! {
                        <>
                        <>
                        {
                            for props.data.queue.iter().map(|e| {
                                html! {
                                <SongRow song={e.clone()} />
                                }
                            })
                        }
                        </>
                        <>
                        {
                            for props.data.upcoming.iter().map(|e| {
                                html! {
                                <SongRow song={e.clone()} />
                                }
                            })
                        }
                        </>
                        </>
                    }
                }
            }
            </div>
            <div class={get_class(*active, ActiveTab::History, true)}>
            {
                if props.data.history.is_empty() {
                    html! {<i>{"History is empty"}</i>}
                } else {
                    html! {
                    {
                        for props.data.history.iter().map(|e| {
                            html! {
                                <SongRow song={e.clone()} />
                            }
                        })
                    }
                    }
                }
            }
            </div>
        </div>
        </div>
    }
}


