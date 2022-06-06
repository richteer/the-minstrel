pub mod toasttray;
pub mod toastlist;

use std::collections::BTreeMap;
use std::rc::Rc;

pub use toasttray::*;
pub use toastlist::*;

use serde::{Serialize, Deserialize};
use yew::{Reducible, UseReducerHandle};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ToastType {
    Success(String),
    Info(String),
    Warning(String),
    Error(String),
}

pub enum ToastAction {
    Toast(ToastType),
    Fade(usize),
    Delete(usize),
}

pub type ToastContext = UseReducerHandle<ToastList>;


#[macro_export]
macro_rules! toast_info {
    ($string:expr) => {
        $crate::ToastAction::Toast($crate::ToastType::Info($string))
    };
}
#[macro_export]
macro_rules! toast_success {
    ($string:expr) => {
        $crate::ToastAction::Toast($crate::ToastType::Success($string))
    };
}
#[macro_export]
macro_rules! toast_warning {
    ($string:expr) => {
        $crate::ToastAction::Toast($crate::ToastType::Warning($string))
    };
}
#[macro_export]
macro_rules! toast_error {
    ($string:expr) => {
        $crate::ToastAction::Toast($crate::ToastType::Error($string))
    };
}