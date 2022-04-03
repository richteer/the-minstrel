use yew::{
    prelude::*,
    function_component,
    html
};
use webdata::{
    Song,
};


fn duration_text(dur: i64) -> String {
    let min = dur / 60;
    let secs = dur % 60;

    format!("{}:{:02}", min, secs)
}

#[derive(Properties, PartialEq)]
pub struct SongTextProps {
    pub song: Song,
}

#[function_component(SongText)]
pub fn song_text(props: &SongTextProps) -> Html {
    let song = &props.song;
    html! {
        <div class="songdata">
            <div>
                <div><span class="songtitle">{song.title.clone()}</span></div>
                <div><span class="songartist">{song.artist.clone()}</span></div>
                <div><span class="songduration">{duration_text(song.duration)}</span></div>
            </div>
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
        <>
            <div class="songicon">
                <img src={song.thumbnail.clone()} alt="temp" />
            </div>
            <SongText song={song.clone()} />
            <div class="user">
                <img src={ song.requested_by.icon.clone() } alt="temp" />
                <span class="username">{ song.requested_by.displayname.clone() }</span>
            </div>
        </>
    }
}