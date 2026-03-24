use gpui_component::IconNamed;

pub enum IconName {
    ChevronDown,
    ChevronRight,
    Database,
    Eye,
    Grid3x3,
    Plus,
    Search,
}

impl IconNamed for IconName {
    fn path(self) -> gpui::SharedString {
        match self {
            Self::ChevronDown => "icons/chevron-down.svg",
            Self::ChevronRight => "icons/chevron-right.svg",
            Self::Database => "icons/database.svg",
            Self::Eye => "icons/eye.svg",
            Self::Grid3x3 => "icons/grid-3x3.svg",
            Self::Plus => "icons/plus.svg",
            Self::Search => "icons/search.svg",
        }
        .into()
    }
}
