use std::sync::Arc;
use tokio::sync::Mutex;
use music::musiccontroller::MusicAdapter;
use std::convert::Infallible;


pub async fn show_state(
    mstate: Arc<Mutex<MusicAdapter>>
) -> Result<impl warp::Reply, Infallible> {
    let ret = {
        let mstate = mstate.lock().await;

        mstate.get_webdata().await
    };

    Ok(warp::reply::json(&ret))
}
