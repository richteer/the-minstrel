pub mod general;
pub mod musicctl;
pub mod queuectl;
pub mod autoplay;
pub mod config;
pub mod debug;

use crate::music;

pub mod helpers;
pub use helpers::IN_SAME_VOICE_CHECK as IN_SAME_VOICE_CHECK;
pub use helpers::check_msg as check_msg;
pub use helpers::mstate_get as mstate_get;