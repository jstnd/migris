use anyhow::anyhow;
use gpui::{App, AssetSource, SharedString};
use gpui_component::{Theme, ThemeMode, ThemeRegistry};

#[derive(rust_embed::RustEmbed)]
#[folder = "assets"]
#[include = "icons/*.svg"]
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> gpui::Result<Option<std::borrow::Cow<'static, [u8]>>> {
        if path.is_empty() {
            return Ok(None);
        }

        Self::get(path)
            .map(|f| Some(f.data))
            .ok_or_else(|| anyhow!("could not find asset at path: {path}"))
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<gpui::SharedString>> {
        Ok(Self::iter()
            .filter_map(|p| p.starts_with(path).then(|| p.into()))
            .collect())
    }
}

#[derive(rust_embed::RustEmbed)]
#[folder = "assets"]
#[include = "themes/*.json"]
pub struct Themes;

impl Themes {
    const DEFAULT_DARK: &str = "Default Dark";
    const DEFAULT_LIGHT: &str = "Default Light";

    /// Initializes the application themes.
    ///
    /// This should only be called at the application's entry point.
    pub fn init(cx: &mut App) {
        let registry = ThemeRegistry::global_mut(cx);

        for path in Self::iter() {
            if let Some(file) = Self::get(&path)
                && let Ok(theme) = str::from_utf8(&file.data)
            {
                // TODO: log issues with loading themes here
                _ = registry.load_themes_from_str(theme);
            }
        }
    }

    /// Applies the given theme to the application.
    pub fn apply(cx: &mut App, theme: SharedString) {
        if let Some(theme) = ThemeRegistry::global(cx).themes().get(&theme).cloned() {
            Theme::global_mut(cx).apply_config(&theme);
        }
    }

    /// Returns the default theme for the given [`ThemeMode`].
    pub fn default(mode: ThemeMode) -> SharedString {
        match mode {
            ThemeMode::Dark => SharedString::from(Self::DEFAULT_DARK),
            ThemeMode::Light => SharedString::from(Self::DEFAULT_LIGHT),
        }
    }

    /// Returns a list of themes matching the given [`ThemeMode`].
    /// 
    /// Intended for use with dropdown components.
    pub fn options(cx: &App, mode: ThemeMode) -> Vec<(SharedString, SharedString)> {
        ThemeRegistry::global(cx)
            .sorted_themes()
            .iter()
            .filter_map(|theme| {
                (theme.mode == mode).then_some((theme.name.clone(), theme.name.clone()))
            })
            .collect()
    }
}
