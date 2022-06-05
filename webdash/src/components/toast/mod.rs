mod toastbus;
mod toasttray;
mod use_toast;

pub use toasttray::ToastTray;
pub use toastbus::ToastBus;
pub use use_toast::use_toast;

use serde::{Serialize, Deserialize};


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ToastType {
    Success(String),
    Info(String),
    Warning(String),
    Error(String),
}

