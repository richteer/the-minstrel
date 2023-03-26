use std::rc::Rc;

use gloo_net::http::{Request, Response};
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_hooks::prelude::*;
use yew_feather::{
    log_in,
    log_out,
};
use model::{
    web::{
        LoginRequest,
        RegisterRequest, LinkRequest,
        ReplyStatus, ReplyData,
    },
    Requester
};
use yew_toast::{
    ToastContext,
    ToastList,
    toast_error,
    toast_info,
};

use crate::components::requester::*;

async fn login_update_usercontext(
    resp: &Response,
    usercontext: &UseReducerHandle<LoginStatus>,
    toastcontext: &UseReducerHandle<ToastList>
) {
    match resp.json::<ReplyStatus>().await {
        Ok(reply) => {
            if let Some(ReplyData::UserInfo(ui)) = reply.data {
                usercontext.dispatch(UserContextAction::UpdateInfo(ui));
                toastcontext.dispatch(toast_info!("Successfully logged in!".into()));
            } else {
                log::error!("Server sent back an empty requester field? {reply:?}");
                toastcontext.dispatch(toast_error!("Server sent back some garbage, check console".into()));
            };
        },
        Err(_) => {
            log::error!("Could not parse response from server: {resp:?}");
            toastcontext.dispatch(toast_error!("Server sent back some garbage, check console".into()));
        }
    };
}


#[derive(Properties, PartialEq)]
pub struct LoginFormProps {
    pub open_handle: UseToggleHandle<bool>
}

#[function_component(LoginForm)]
pub fn login_form(props: &LoginFormProps) -> Html {
    let username_noderef = use_node_ref();
    let password_noderef = use_node_ref();

    let toastcontext = use_context::<ToastContext>().unwrap();
    let usercontext = use_context::<UserContext>().unwrap();

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

            match resp.status() {
                200 => {
                    open.toggle();

                    login_update_usercontext(&resp, &usercontext, &toastcontext).await;

                    Ok(())
                },
                401 => {
                    // TODO: change the style of the form to indicate error
                    toastcontext.dispatch(toast_error!("Invalid username or password!".into()));
                    Err(())
                },
                _ => {
                    toastcontext.dispatch(toast_error!("An unknown error occurred...".into()));
                    log::error!("unhandled response code {} from login, fix me!", resp.status());
                    Err(())
                },
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

    html! {
        <form method="dialog" onsubmit={click_login}>
            <div class="field">
                <label class="label">{"Username"}</label>
                <input class="input" ref={username_noderef} type="text" name="username" placeholder="Username" maxlength="64" />
            </div>
            <div class="field">
                <label class="label">{"Password"}</label>
                <input class="input" ref={password_noderef} type="password" name="password" placeholder="Password" maxlength="1024" />
            </div>
            <div class="field">
                <input type="submit" class="button is-link" name="login" value="Log In"/>
            </div>
        </form>
    }
}

#[derive(Properties, PartialEq)]
pub struct RegisterFormProps {
    pub open_handle: UseToggleHandle<bool>
}

#[function_component(RegisterForm)]
pub fn register_form(props: &RegisterFormProps) -> Html {
    let toastcontext = use_context::<ToastContext>().unwrap();
    let usercontext = use_context::<UserContext>().unwrap();

    let username_noderef = use_node_ref();
    let password_noderef = use_node_ref();
    let password2_noderef = use_node_ref();
    let displayname_noderef = use_node_ref();
    let icon_noderef = use_node_ref();

    // Callback to actually attempt the login
    //   Needs references to the modal open state, and noderefs for the inputs
    let post_register = {
        let open = props.open_handle.clone();
        let username_noderef = username_noderef.clone();
        let password_noderef = password_noderef.clone();
        let password2_noderef = password2_noderef.clone();
        let displayname_noderef = displayname_noderef.clone();
        let icon_noderef = icon_noderef.clone();

        use_async(async move {
            let username = username_noderef.cast::<HtmlInputElement>().unwrap().value();
            let password = password_noderef.cast::<HtmlInputElement>().unwrap().value();
            let password2 = password2_noderef.cast::<HtmlInputElement>().unwrap().value();
            let displayname = displayname_noderef.cast::<HtmlInputElement>().unwrap().value();
            let icon: String = icon_noderef.cast::<HtmlInputElement>().unwrap().value();

            if password != password2 {
                // TODO: actually report validation errors to the user
                return Err(())
            }

            // TODO: consider validating client-side if icon actually points somewhere?
            //  or maybe validate server side?
            let icon = if icon.is_empty() {
                Some(icon)
            } else {
                None
            };

            let resp = Request::post("/api/register")
                .json(&RegisterRequest { username, password, displayname, icon }).unwrap()
                .send().await.unwrap();

            if resp.ok() {
                open.toggle();

                login_update_usercontext(&resp, &usercontext, &toastcontext).await;

                Ok(())
            } else {
                Err(())
            }
        })
    };

    // Callback for clicking the login button
    let click_register = {
        let post_register = post_register.clone();
        Callback::from(move |_| {
            post_register.run();
        })
    };


    html! {
        <form method="dialog" onsubmit={click_register}>
            <div class="field">
                <label class="label">{"Username"}</label>
                <input class="input" ref={username_noderef} type="text" name="username" placeholder="Username" maxlength="64" />
            </div>
            <div class="field">
                <label class="label">{"Password"}</label>
                // TODO: validate password requirements, length, etc
                <input class="input" ref={password_noderef} type="password" name="password" placeholder="Password" maxlength="1024" />
            </div>
            <div class="field">
                <label class="label">{"Password (again)"}</label>
                <input class="input" ref={password2_noderef} type="password" name="password" placeholder="Password" maxlength="1024" />
            </div>
            <div class="field">
                <label class="label">{"Display Name"}</label>
                <input class="input" ref={displayname_noderef} type="text" name="displayname" placeholder="Display Name (e.g. Steve)" maxlength="64" />
            </div>
            <div class="field">
                <label class="label">{"Icon URL (optional)"}</label>
                <input class="input" ref={icon_noderef} type="text" name="icon" placeholder="URL to icon" maxlength="64" />
            </div>
            <div class="field">
                <input type="submit" class="button is-link" name="register" value="Register"/>
            </div>
        </form>
    }
}


#[derive(Properties, PartialEq)]
pub struct LinkFormProps {
    pub open_handle: UseToggleHandle<bool>
}

#[function_component(LinkForm)]
pub fn link_form(props: &LinkFormProps) -> Html {
    let toastcontext = use_context::<ToastContext>().unwrap();
    let usercontext = use_context::<UserContext>().unwrap();

    let username_noderef = use_node_ref();
    let password_noderef = use_node_ref();
    let password2_noderef = use_node_ref();
    let link_noderef = use_node_ref();

    // Callback to actually attempt the login
    //   Needs references to the modal open state, and noderefs for the inputs
    let post_register = {
        let open = props.open_handle.clone();
        let username_noderef = username_noderef.clone();
        let password_noderef = password_noderef.clone();
        let password2_noderef = password2_noderef.clone();
        let link_noderef = link_noderef.clone();

        use_async(async move {
            let username = username_noderef.cast::<HtmlInputElement>().unwrap().value();
            let password = password_noderef.cast::<HtmlInputElement>().unwrap().value();
            let password2 = password2_noderef.cast::<HtmlInputElement>().unwrap().value();
            let link = link_noderef.cast::<HtmlInputElement>().unwrap().value();

            // TODO: validate link is actually a number, etc etc
            let link = link.parse::<u64>().map_err(|_| ())?;

            if password != password2 {
                // TODO: actually report validation errors to the user
                return Err(())
            }

            let resp = Request::post("/api/link")
                .json(&LinkRequest { username, password, link }).unwrap()
                .send().await.unwrap();

            if resp.ok() {
                open.toggle();

                login_update_usercontext(&resp, &usercontext, &toastcontext).await;

                Ok(())
            } else {
                match resp.json::<ReplyStatus>().await {
                    Ok(ui) => {
                        log::error!("Error returned from server: {:?}", ui);
                        toastcontext.dispatch(toast_error!(format!("Error: {:?}", ui.error)));
                    },
                    Err(e) => {
                        log::error!("Error {e:?}, server sent back garbage: {resp:?}");
                        toastcontext.dispatch(toast_error!("Server sent back some garbage, check console".into()));
                    },
                };

                Err(())
            }
        })
    };

    // Callback for clicking the login button
    let click_register = {
        let post_register = post_register.clone();
        Callback::from(move |_| {
            post_register.run();
        })
    };

    html! {
        <form method="dialog" onsubmit={click_register}>
            <div class="field">
                <label class="label">{"Username"}</label>
                <input class="input" ref={username_noderef} type="text" name="username" placeholder="Username" maxlength="64" />
            </div>
            <div class="field">
                <label class="label">{"Password"}</label>
                // TODO: validate password requirements, length, etc
                <input class="input" ref={password_noderef} type="password" name="password" placeholder="Password" maxlength="1024" />
            </div>
            <div class="field">
                <label class="label">{"Password (again)"}</label>
                <input class="input" ref={password2_noderef} type="password" name="password" placeholder="Password" maxlength="1024" />
            </div>
            <div class="field">
                <label class="label">{"Link"}</label>
                <input class="input" ref={link_noderef} type="text" name="link" placeholder="Link number" maxlength="64" />
            </div>
            <div class="field">
                <input type="submit" class="button is-link" name="register" value="Register"/>
            </div>
        </form>
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
    Link,
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

    let link_onclick = {
        let active_tab = active_tab.clone();
        Callback::from(move |_| active_tab.set(ActiveTab::Link))
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
        <div class="modal is-active is-text-shadowless">
            <div class="modal-background" onclick={toggle_modal.clone()} />
            <div class="modal-card">
                <div class="modal-card-body">
                    <div class="tabs is-fullwidth is-centered">
                        <ul>
                            <li class={get_class(*active_tab, ActiveTab::Login, false)} onclick={login_onclick}><a>{"Log In"}</a></li>
                            <li class={get_class(*active_tab, ActiveTab::Register, false)} onclick={register_onclick}><a>{"Register"}</a></li>
                            <li class={get_class(*active_tab, ActiveTab::Link, false)} onclick={link_onclick}><a>{"Link"}</a></li>
                        </ul>
                    </div>
                    <div class={get_class(*active_tab, ActiveTab::Login, true)}>
                        <LoginForm open_handle={props.open_handle.clone()}/>
                    </div>
                    <div class={get_class(*active_tab, ActiveTab::Register, true)}>
                        <RegisterForm open_handle={props.open_handle.clone()}/>
                    </div>
                    <div class={get_class(*active_tab, ActiveTab::Link, true)}>
                        <LinkForm open_handle={props.open_handle.clone()}/>
                    </div>
                </div>
            </div>
            <button class="modal-close" aria-label="close" onclick={toggle_modal}/>
        </div>
    }
}


#[derive(Clone, Debug, PartialEq)]
pub struct LoginStatus {
    pub current_user: Option<Requester>,
}

pub enum UserContextAction {
    UpdateInfo(Requester),
    RemoveInfo,
}

impl Reducible for LoginStatus {
    type Action = UserContextAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        match action {
            UserContextAction::UpdateInfo(req) => Self { current_user: Some(req) },
            UserContextAction::RemoveInfo => Self { current_user: None },
        }.into()
    }
}

pub type UserContext = UseReducerHandle<LoginStatus>;

#[function_component(Login)]
pub fn login() -> Html {
    let usercontext = use_context::<UserContext>().unwrap();
    let toastcontext = use_context::<ToastContext>().unwrap();

    let user = (*usercontext).current_user.clone();

    let open = use_bool_toggle(false);

    let toggle_modal = {
        let open = open.clone();
        Callback::from(move |_| {
            open.toggle();
        })
    };
    let logout_onclick = {
        let usercontext = usercontext.clone();
        let toastcontext = toastcontext.clone();

        let logout = use_async(async move {
            let resp = Request::post("/api/logout")
                .send().await.unwrap();

            if resp.ok() {
                Ok(())
            } else {
                Err(())
            }
        });

        Callback::from(move |_| {
            logout.run();

            usercontext.dispatch(UserContextAction::RemoveInfo);
            toastcontext.dispatch(toast_info!("Successfully logged out".into()));
        })
    };

    if use_is_first_mount() {
        use_async_with_options(async move {
            let resp = Request::post("/api/userinfo")
            .send().await.unwrap();

            if resp.ok() {
                login_update_usercontext(&resp, &usercontext, &toastcontext).await;

                Ok(())
            } else {
                Err(())
            }
        },
        UseAsyncOptions::enable_auto());
    }


    html! {
        <>
        <div class="logintray">
            {
                if let Some(user) = user {
                    html! {
                        <div class="is-flex is-flex-direction-row">
                            <RequesterTag requester={user.clone()} size={RequesterSize::Tiny} />
                            <div class="controlicon ml-2" onclick={logout_onclick}>
                                <log_out::LogOut />
                            </div>
                        </div>
                    }
                } else {
                    html! {
                        <div class="controlicon" onclick={toggle_modal.clone()}>
                            <log_in::LogIn />
                        </div>
                    }
                }
            }

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