use iced::{
    Font,
    widget::{Text, text},
};

pub const FONT_LUCIDE: Font = Font::with_name("lucide");

pub fn icon<'a>(icon: Icon) -> Text<'a> {
    text(icon.unicode()).font(FONT_LUCIDE)
}

#[derive(Debug, Clone, Copy)]
pub enum Icon {
    ChevronDown,
    ChevronRight,
    Database,
    Eye,
    Plus,
    Search,
    Table,
}

impl Icon {
    pub fn unicode(self) -> char {
        match self {
            Self::ChevronDown => '\u{E06D}',
            Self::ChevronRight => '\u{E06F}',
            Self::Database => '\u{E0AD}',
            Self::Eye => '\u{E0BA}',
            Self::Plus => '\u{E13D}',
            Self::Search => '\u{E151}',
            Self::Table => '\u{E17D}',
        }
    }
}
