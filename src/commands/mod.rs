pub mod general;
pub mod musicctl;
pub mod queuectl;
pub mod debug;

use serenity::{
    model::{
        channel::Message,
    },
    Result as SerenityResult
};

use super::music;


/// Checks that a message successfully sent; if not, then logs why to stdout.
fn check_msg(result: SerenityResult<Message>) {
    if let Err(why) = result {
        println!("Error sending message: {:?}", why);
    }
}

mod helpers;
pub use helpers::VOICE_READY_CHECK as VOICE_READY_CHECK;

// TODO: These can definitely be cleaner, but might as well macro out now to make
//  life slightly easier if I do end up needing to replace them
#[macro_export]
macro_rules! get_mstate {
    ($mstate:ident, $ctx:ident) => {
        let $mstate = music::get(&$ctx).await.unwrap();
        let $mstate = $mstate.lock().await;
    };

    ($mut:ident, $mstate:ident, $ctx:ident) => {
        let $mstate = music::get(&$ctx).await.unwrap();
        let $mut $mstate = $mstate.lock().await;
    };
}