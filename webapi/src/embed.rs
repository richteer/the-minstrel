use warp::{Filter, path::FullPath};
use rust_embed::RustEmbed;
use log::*;

#[derive(RustEmbed)]
#[folder = "../webdash/dist/"]
struct EmbeddedWebdash;


pub fn get_embedded_file_filter() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::get()
    .and(warp::path::full())
    .map(|filename: FullPath| {
        let filename = &filename.as_str()[1..];
        let file = EmbeddedWebdash::get(filename);
        debug!("GET /{}", filename);

        if let Some(data) = file {
            let data = data.data;
            let mime = mime_guess::from_path(filename).first();

            if let Some(mime) = mime {
                debug!("mime = {}", mime);
                warp::http::Response::builder()
                    .header("Content-Type", mime.to_string())
                    .body(Vec::from(data))
            } else {
                warp::http::Response::builder().status(500).body(Vec::new())
            }
        } else {
            warn!("file not embedded: {}", filename);
            warp::http::Response::builder().status(404).body(Vec::new())
        }
    })
}
