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



#[derive(Properties, PartialEq)]
pub struct SongTextProps {
    pub song: Song,
}

#[function_component(SongText)]
pub fn song_text(props: &SongTextProps) -> Html {
    let song = &props.song;
    html! {
        <div class="songdata">
            <span class="songtitle songoverflow">{song.title.clone()}</span>
            <span class="songartist songoverflow">{song.artist.clone()}</span>
            <span class="songduration">{duration_text(song.duration)}</span>
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
        <div class={"upcomingitem"}>
            <div class="songicon">
                <a href={song.url.clone()} target="_blank" rel="noopener noreferrer">
                    <img src={song.thumbnail.clone()} alt="temp" />
                    <div class="songicon-overlay">
                        <div class="songicon-overlay-content">
                            <external_link::ExternalLink color="white" size="28"/>
                        </div>
                    </div>
                </a>
            </div>
            <SongText song={song.clone()} />
            <div class="user">
                <span class="username">{ song.requested_by.displayname.clone() }</span>
                <img src={ song.requested_by.icon.clone() } alt="temp" />
            </div>
        </div>
    }
}
