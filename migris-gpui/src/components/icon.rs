use gpui_component::IconNamed;

pub enum IconName {
    ChevronDown,
    ChevronRight,
    Code,
    Database,
    Eye,
    Grid3x3,
    Play,
    Plus,
    Search,
    X,
}

impl IconNamed for IconName {
    fn path(self) -> gpui::SharedString {
        match self {
            Self::ChevronDown => "icons/chevron-down.svg",
            Self::ChevronRight => "icons/chevron-right.svg",
            Self::Code => "icons/code.svg",
            Self::Database => "icons/database.svg",
            Self::Eye => "icons/eye.svg",
            Self::Grid3x3 => "icons/grid-3x3.svg",
            Self::Play => "icons/play.svg",
            Self::Plus => "icons/plus.svg",
            Self::Search => "icons/search.svg",
            Self::X => "icons/x.svg",
        }
        .into()
    }
}
