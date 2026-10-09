use gpui_kit::{App, Pixels, component::ActiveTheme};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Size {
    XSmall,
    Small,
    #[default]
    Medium,
    Large,
    XLarge,
    XXLarge,
    XXXLarge,
}

impl Size {
    /// Returns the size one level smaller than the current size.
    pub fn decrease(&self) -> Self {
        match self {
            Self::XSmall => Self::XSmall,
            Self::Small => Self::XSmall,
            Self::Medium => Self::Small,
            Self::Large => Self::Medium,
            Self::XLarge => Self::Large,
            Self::XXLarge => Self::XLarge,
            Self::XXXLarge => Self::XXLarge,
        }
    }

    /// Returns the corresponding font size for the current size.
    pub fn font_size(&self, cx: &App) -> Pixels {
        let font_size = cx.theme().font_size;
        match self {
            Self::XSmall => font_size * 0.75,
            Self::Small => font_size * 0.875,
            Self::Medium => font_size,
            Self::Large => font_size * 1.125,
            Self::XLarge => font_size * 1.25,
            Self::XXLarge => font_size * 1.5,
            Self::XXXLarge => font_size * 1.875,
        }
    }

    /// Returns the size one level larger than the current size.
    pub fn increase(&self) -> Self {
        match self {
            Self::XSmall => Self::Small,
            Self::Small => Self::Medium,
            Self::Medium => Self::Large,
            Self::Large => Self::XLarge,
            Self::XLarge => Self::XXLarge,
            Self::XXLarge => Self::XXXLarge,
            Self::XXXLarge => Self::XXXLarge,
        }
    }

    /// Returns the corresponding GPUI size to use for table components.
    pub fn table_size(&self) -> gpui_kit::component::Size {
        match self {
            Size::XSmall | Size::Small | Size::Medium => gpui_kit::component::Size::XSmall,
            Size::Large | Size::XLarge => gpui_kit::component::Size::Small,
            Size::XXLarge => gpui_kit::component::Size::Medium,
            Size::XXXLarge => gpui_kit::component::Size::Large,
        }
    }
}
