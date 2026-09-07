use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Size {
    XSmall,
    #[default]
    Small,
    Medium,
    Large,
    XLarge,
    XXLarge,
    XXXLarge,
}

impl Size {
    /// Returns the size one level smaller than the current.
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

    /// Returns the size one level larger than the current.
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
}
