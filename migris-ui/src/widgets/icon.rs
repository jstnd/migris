use iced::{
    Font,
    widget::{Text, text},
};

pub const FONT_LUCIDE: Font = Font::with_name("lucide");

#[derive(Debug, Clone, Copy)]
pub enum Icon {
    Plus,
    Search,
}

impl Icon {
    pub fn unicode(self) -> char {
        match self {
            Self::Plus => '\u{E13D}',
            Self::Search => '\u{E151}',
        }
    }
}

pub fn icon<'a>(icon: Icon) -> Text<'a> {
    text(icon.unicode()).font(FONT_LUCIDE)
}
