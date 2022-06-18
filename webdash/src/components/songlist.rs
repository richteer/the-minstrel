use yew::{
    prelude::*,
    function_component,
    html,
};
use model::{
    MinstrelWebData,
};

use crate::components::{songrow::*, UserContext};


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
            (false, true)  => String::from("is-active"),
            (false, false) => String::from(""),
            (true,  true)  => String::from(""),
            (true,  false) => String::from("is-hidden"),
        }
    }

    let usercontext = use_context::<UserContext>().unwrap();
    let muid = usercontext.current_user.as_ref().map(|ui| ui.id);

    // Pre-index logged-in user's songs
    let mut upcoming = Vec::new();
    let mut i = 0;
    for up in props.data.upcoming.iter() {
        upcoming.push(match muid.map(|muid| muid == up.requested_by.id) {
            Some(true) => {
                let ret = (Some(i), up.clone());
                i += 1;
                ret
            },
            _ => (None, up.clone()),
        });
    }

    html! {
        <div class="tabview">
        <div class="tabs">
            <ul>
                <li class={get_class(*active, ActiveTab::ComingUp, false)} onclick={comingup_onclick}><a>{"Coming up"}</a></li>
                <li class={get_class(*active, ActiveTab::History, false)} onclick={history_onclick}><a>{"History"}</a></li>
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
                                <SongRow song={e.clone()} enqueued={true}/>
                                }
                            })
                        }
                        </>
                        <>
                        {
                            for upcoming.drain(..).map(|(i,e)| {
                                match i {
                                    Some(i) => html! {
                                        <SongRow song={e} index={i}/>
                                    },
                                    _ => html! {
                                        <SongRow song={e}/>
                                    }
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


