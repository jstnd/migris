use iced::{
    Font,
    widget::{Text, text},
};

const FONT_LUCIDE: Font = Font::with_name("lucide");

#[derive(Debug, Clone, Copy)]
pub enum Icon {
    Plus,
}

impl Icon {
    pub fn unicode(self) -> &'static str {
        match self {
            Self::Plus => "\u{E13D}",
        }
    }
}

pub fn icon<'a>(icon: Icon) -> Text<'a> {
    text(icon.unicode()).font(FONT_LUCIDE)
}
