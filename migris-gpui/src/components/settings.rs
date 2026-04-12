use gpui::{App, ParentElement, SharedString, Styled, Window, px};
use gpui_component::{
    ActiveTheme, Theme, WindowExt,
    button::Button,
    dialog::{Dialog, DialogFooter},
    h_flex,
    setting::{SettingField, SettingGroup, SettingItem, SettingPage, Settings},
};

use crate::{
    assets::Themes,
    config::{AppSettings, AppThemeMode},
    shared,
};

pub fn settings_dialog(dialog: Dialog, _: &mut Window, cx: &mut App) -> Dialog {
    dialog
        .w(px(800.0))
        .h(px(600.0))
        .title("Settings")
        .child(Settings::new("app-settings").pages(Vec::from([
            SettingPage::new("General").group(appearance_group(cx)),
        ])))
        .footer(
            DialogFooter::new().child(
                h_flex()
                    .gap_2()
                    .child(Button::new("settings-cancel").label("Cancel").on_click(
                        |_, window, cx| {
                            window.close_dialog(cx);
                        },
                    ))
                    .child(
                        Button::new("settings-save")
                            .label("Save")
                            .on_click(|_, window, cx| {
                                window.close_dialog(cx);
                            }),
                    ),
            ),
        )
}

fn appearance_group(cx: &mut App) -> SettingGroup {
    SettingGroup::new()
        .title("Appearance")
        .item(SettingItem::new(
            "Theme Mode",
            SettingField::dropdown(
                AppThemeMode::options(),
                |cx| SharedString::from(AppSettings::global(cx).theme_mode.to_string()),
                |value, cx| {
                    AppSettings::global_mut(cx).theme_mode = AppThemeMode::from(value);
                    Theme::change(AppSettings::global(cx).theme_mode(cx), None, cx);
                },
            ),
        ))
        .item(SettingItem::new("Theme", {
            let options = Themes::options(cx, cx.theme().mode);
            let theme = AppSettings::global(cx).theme(cx);

            // If the theme saved in the settings does not match any themes in the current theme mode,
            // we want to change the saved theme to the default for the current theme mode.
            // TODO: move this to happen on settings load when implemented
            if !options.iter().any(|option| option.0 == theme) {
                shared::apply_theme(cx, Themes::default(cx.theme().mode));
            }

            SettingField::dropdown(
                options,
                |cx| AppSettings::global(cx).theme(cx),
                |value, cx| {
                    shared::apply_theme(cx, value);
                },
            )
        }))
}
