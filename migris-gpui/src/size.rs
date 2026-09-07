use gpui::Styled;
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

    /// Applies the corresponding text size to the given element.
    pub fn text_size<T: Styled>(&self, element: T) -> T {
        match self {
            Self::XSmall => element.text_xs(),
            Self::Small => element.text_sm(),
            Self::Medium => element.text_base(),
            Self::Large => element.text_lg(),
            Self::XLarge => element.text_xl(),
            Self::XXLarge => element.text_2xl(),
            Self::XXXLarge => element.text_3xl(),
        }
    }
}
