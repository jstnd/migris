pub mod query;

use gpui::{AnyElement, SharedString};

use crate::components::icon::IconName;

pub trait TabView {
    fn icon(&self) -> IconName;
    fn label(&self) -> SharedString;
    fn content(&self) -> AnyElement;
}
