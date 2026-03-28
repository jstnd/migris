pub mod query;

use std::any::Any;

use gpui::{AnyElement, SharedString};

use crate::components::icon::IconName;

pub trait TabView: Any {
    fn icon(&self) -> IconName;
    fn label(&self) -> SharedString;
    fn content(&self) -> AnyElement;
    fn as_any(&self) -> &dyn Any;
}
