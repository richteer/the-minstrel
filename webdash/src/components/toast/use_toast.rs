use yew_agent::{
    UseBridgeHandle,
    use_bridge
};

use super::ToastBus;

// TODO: Consider making this a bigger helper where other locations can just call a simple function for a toast
pub fn use_toast() -> UseBridgeHandle<ToastBus> {
    use_bridge::<ToastBus, _>(|_| {log::debug!("Component really didn't need to receive this, fix this at some point")})
}