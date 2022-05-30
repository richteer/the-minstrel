use std::sync::Arc;
use tokio::sync::Mutex;
use music::MusicState;
use std::convert::Infallible;


pub async fn show_state(
    mstate: Arc<Mutex<MusicState>>
) -> Result<impl warp::Reply, Infallible> {
    let ret = {
        let mstate = mstate.lock().await;

        mstate.get_webdata()
    };

    Ok(warp::reply::json(&ret))
}
