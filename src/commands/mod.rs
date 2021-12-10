pub mod general;
pub mod musicctl;
pub mod queuectl;
pub mod debug;

use super::music;

mod helpers;
pub use helpers::VOICE_READY_CHECK as VOICE_READY_CHECK;
pub use helpers::check_msg as check_msg;
