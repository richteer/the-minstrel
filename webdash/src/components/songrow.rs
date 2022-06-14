use gloo_net::http::Request;
use model::web::{ApBumpRequest, ReplyStatus};
use yew::{
    prelude::*,
    function_component,
    html,
};
use model::{
    Song, SongRequest,
};

use yew_feather::{
    external_link,
    plus_circle,
};
use yew_hooks::{
    use_async,
};

use yew_feather::{
    trash,
};
use yew_toast::{ToastContext, toast_info, toast_error};

use crate::components::helpers::duration_text;
use crate::components::requester::*;


#[derive(Properties, PartialEq)]
pub struct SongTextProps {
    pub song: Song,
}

#[function_component(SongText)]
pub fn song_text(props: &SongTextProps) -> Html {
    let song = &props.song;
    html! {
        <div class="is-flex is-flex-direction-column is-gapless mx-2">
            <span class="songoverflow has-text-weight-bold">{song.title.clone()}</span>
            <span class="songoverflow is-italic" style="font-size: 90%">{song.artist.clone()}</span>
            <span class="" style="font-size: 80%">{duration_text(song.duration)}</span>
        </div>
    }
}



#[derive(Properties, PartialEq)]
pub struct SongRowProps {
    pub song: SongRequest,
    pub enqueued: Option<bool>,
    pub index: Option<usize>,
}

#[function_component(SongRow)]
pub fn song_row(props: &SongRowProps) -> Html {
    let requested_by = &props.song.requested_by;
    let song = &props.song.song;

    let qicon = match props.enqueued {
        Some(true) => html! {
            <div class="queuedicon" title="In Queue">
                <plus_circle::PlusCircle />
            </div>
        },
        _ => html! {
            <></>
        },
    };

    let bump_callback = if let Some(index) = props.index {
        let toastcontext = use_context::<ToastContext>().unwrap();

        let bump = use_async(async move {
            let resp = Request::post("/api/autoplay/bump")
                .json(&ApBumpRequest { index } ).unwrap()
                .send().await.unwrap();

            if resp.ok() {
                toastcontext.dispatch(toast_info!("Bumped song from upcoming".into()));
                Ok(())
            } else {
                let resp = resp.json::<ReplyStatus>().await;
                if let Ok(resp) = resp {
                    toastcontext.dispatch(toast_error!(format!("Error bumping song: {} song from upcoming", resp.error)));
                } else {
                    log::error!("Server returned garbage: {:?}", resp);
                    toastcontext.dispatch(toast_error!("Server returned some garbage, check console".into()));
                }

                Err(())
            }
        });

        Callback::from(move |_| {
            bump.run();
        })
    } else {
        Callback::from(|_| ())
    };

    html! {
        <div class="columns is-gapless is-mobile mb-0 is-vcentered songrow">
            <div class="column container is-narrow">
                <a href={song.url.clone()} target="_blank" rel="noopener noreferrer">
                    <figure class="image is-flex is-4by3 is-justify-content-center" style={"width: 96px"}>
                        <img src={song.thumbnail.clone()} alt="temp" style="object-fit: cover"/>
                    </figure>
                    <div class="is-overlay">
                        <div class="container songicon-overlay is-flex is-justify-content-center is-align-items-center">
                            <external_link::ExternalLink color="white" size="28"/>
                        </div>
                    </div>
                </a>
                { qicon }
            </div>
            <div class="column is-clipped">
                <SongText song={song.clone()} />
            </div>
            {
                if props.index.is_some() {
                    html! {
                        // TODO: consider a pop-up menu if more controls are to be added
                        <div onclick={bump_callback} class="is-flex bumpicon mr-2">
                            <trash::Trash />
                        </div>
                    }
                } else {
                    html! {}
                }
            }
            <div class="column is-narrow is-flex is-flex-direction-column is-justify-content-center mr-2">
                <RequesterTag requester={requested_by.clone()} />
            </div>

        </div>
    }
}
