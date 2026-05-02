use gpui::{App, Hsla, IntoElement, RenderOnce, Styled, Window};
use gpui_component::{ActiveTheme, IconNamed};

pub enum IconName {
    ChevronDown,
    ChevronRight,
    CircleX2,
    Code,
    Database,
    Eye,
    Folder,
    FolderOpen,
    FolderPlus,
    Grid3x3,
    MousePointer2,
    Play,
    Plus,
    Save,
    Search,
    Settings,
    TextCursorInput,
    Trash,
    X,
}

impl IconNamed for IconName {
    fn path(self) -> gpui::SharedString {
        match self {
            Self::ChevronDown => "icons/chevron-down.svg",
            Self::ChevronRight => "icons/chevron-right.svg",
            Self::CircleX2 => "icons/circle-x-2.svg",
            Self::Code => "icons/code.svg",
            Self::Database => "icons/database.svg",
            Self::Eye => "icons/eye.svg",
            Self::Folder => "icons/folder.svg",
            Self::FolderOpen => "icons/folder-open.svg",
            Self::FolderPlus => "icons/folder-plus.svg",
            Self::Grid3x3 => "icons/grid-3x3.svg",
            Self::MousePointer2 => "icons/mouse-pointer-2.svg",
            Self::Play => "icons/play.svg",
            Self::Plus => "icons/plus.svg",
            Self::Save => "icons/save.svg",
            Self::Search => "icons/search.svg",
            Self::Settings => "icons/settings.svg",
            Self::TextCursorInput => "icons/text-cursor-input.svg",
            Self::Trash => "icons/trash.svg",
            Self::X => "icons/x.svg",
        }
        .into()
    }
}

#[derive(IntoElement)]
pub struct Icon {
    icon: IconName,
    color: Hsla,
    disabled: bool,
}

impl Icon {
    /// Creates a new [`Icon`].
    pub fn new(cx: &App, icon: IconName) -> Self {
        Self {
            icon,
            color: cx.theme().foreground,
            disabled: false,
        }
    }

    /// Sets the disabled state for the icon.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets the icon to use the danger color.
    pub fn danger(mut self, cx: &App) -> Self {
        self.color = cx.theme().danger;
        self
    }

    /// Sets the icon to use the primary color.
    pub fn primary(mut self, cx: &App) -> Self {
        self.color = cx.theme().button_primary;
        self
    }

    /// Returns the element to render for the icon.
    fn render(self) -> gpui_component::Icon {
        gpui_component::Icon::from(self.icon).text_color({
            let opacity = if self.disabled { 0.25 } else { 1.0 };
            self.color.opacity(opacity)
        })
    }
}

impl From<Icon> for gpui_component::Icon {
    fn from(icon: Icon) -> Self {
        icon.render()
    }
}

impl RenderOnce for Icon {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.render()
    }
}
