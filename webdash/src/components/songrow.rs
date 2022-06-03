use yew::{
    prelude::*,
    function_component,
    html,
};
use model::{
    Song,
};

use yew_feather::{
    external_link,
};

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
    pub song: Song,
}

#[function_component(SongRow)]
pub fn song_row(props: &SongRowProps) -> Html {
    let song = &props.song;

    html! {
        <div class="columns is-gapless is-mobile mb-0">
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
            </div>
            <div class="column is-clipped">
                <SongText song={song.clone()} />
            </div>
            <div class="column is-narrow is-flex is-flex-direction-column is-justify-content-center mr-2">
                <RequesterTag requester={song.requested_by.clone()} />
            </div>
        </div>
    }
}
