use gloo_net::http::Request;
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_hooks::prelude::*;
use yew_feather::log_in;
use model::web::LoginRequest;

#[derive(Properties, PartialEq)]
pub struct LoginFormProps {
    pub open_handle: UseToggleHandle<bool>
}

#[function_component(LoginForm)]
pub fn login_form(props: &LoginFormProps) -> Html {
    let username_noderef = use_node_ref();
    let password_noderef = use_node_ref();

    // Callback to actually attempt the login
    //   Needs references to the modal open state, and noderefs for the inputs
    let post_login = {
        let open = props.open_handle.clone();
        let username_noderef = username_noderef.clone();
        let password_noderef = password_noderef.clone();

        use_async(async move {
            let username = username_noderef.cast::<HtmlInputElement>().unwrap().value();
            let password = password_noderef.cast::<HtmlInputElement>().unwrap().value();

            let resp = Request::post("/api/login")
                .json(&LoginRequest { username, password }).unwrap()
                .send().await.unwrap();

            if resp.ok() {
                open.toggle();
                Ok(())
            } else {
                Err(())
            }
        })
    };

    // Callback for clicking the login button
    let click_login = {
        let post_login = post_login.clone();
        Callback::from(move |_| {
            post_login.run();
        })
    };


    // Handlers for allowing users to press "Enter" to login. Probably could be better
    {
        let post_login = post_login.clone();
        let handler = move |e: KeyboardEvent| {
            // TODO: probably use keycode for cheaper check
            if e.key() == "Enter" {
                post_login.run();
            }
        };

        use_event(username_noderef.clone(), "keypress", handler.clone());
        use_event(password_noderef.clone(), "keypress", handler);
    }


    html! {
        <>
        <div class="field">
            <input class="input" ref={username_noderef} type="text" name="username" maxlength="64" autocomplete="off" placeholder="Username"/>
        </div>
        <div class="field">
            <input class="input" ref={password_noderef} type="password" name="password" maxlength="1024" placeholder="Password"/>
        </div>
        <div class="field">
            <button onclick={click_login} class="button is-link">{"Log In"}</button>
        </div>
        </>
    }
}

#[derive(Properties, PartialEq)]
pub struct LoginCardProps {
    pub open_handle: UseToggleHandle<bool>
}

#[derive(PartialEq, Copy, Clone)]
enum ActiveTab {
    Login,
    Register,
}

#[function_component(LoginCard)]
pub fn login_card(props: &LoginCardProps) -> Html {
    let toggle_modal = {
        let open = props.open_handle.clone();
        Callback::from(move |_| {
            open.toggle();
        })
    };

    let active_tab = use_state(|| ActiveTab::Login);
    let login_onclick = {
        let active_tab = active_tab.clone();
        Callback::from(move |_| { active_tab.set(ActiveTab::Login) })
    };

    let register_onclick = {
        let active_tab = active_tab.clone();
        Callback::from(move |_| active_tab.set(ActiveTab::Register))
    };


    fn get_class(active: ActiveTab, target: ActiveTab, content: bool) -> String {
        match (content, active == target) {
            (false, true)  => String::from("is-active"),
            (false, false) => String::from(""),
            (true,  true)  => String::from(""),
            (true,  false) => String::from("is-hidden"),
        }
    }

    html! {
        <div class="modal is-active">
            <div class="modal-background" onclick={toggle_modal.clone()} />
            <div class="modal-card">
                <div class="modal-card-body">
                    <div class="tabs is-fullwidth is-centered">
                        <ul>
                            <li class={get_class(*active_tab, ActiveTab::Login, false)} onclick={login_onclick}><a>{"Log In"}</a></li>
                            <li class={get_class(*active_tab, ActiveTab::Register, false)} onclick={register_onclick}><a>{"Register"}</a></li>
                        </ul>
                    </div>
                    <div class={get_class(*active_tab, ActiveTab::Login, true)}>
                        <LoginForm open_handle={props.open_handle.clone()}/>
                    </div>
                    <div class={get_class(*active_tab, ActiveTab::Register, true)}>
                        {"TODO!"}
                    </div>
                </div>
            </div>
            <button class="modal-close" aria-label="close" onclick={toggle_modal}/>
        </div>
    }
}


#[function_component(Login)]
pub fn login() -> Html {
    let open = use_bool_toggle(false);

    let toggle_modal = {
        let open = open.clone();
        Callback::from(move |_| {
            open.toggle();
        })
    };


    html! {
        <>
        <div class="logintray">
            <div class="controlicon" onclick={toggle_modal.clone()}>
                <log_in::LogIn />
            </div>
        </div>
        {
            if *open {
                html! {
                    <LoginCard open_handle={open.clone()}/>
                }
            } else {
                html! {}
            }
        }
        </>
    }
}