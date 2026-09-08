use gpui_kit::{
    App, ParentElement, SharedString, Styled, Window,
    base::h_flex,
    component::{
        ActiveTheme, Theme, WindowExt,
        button::Button,
        dialog::{Dialog, DialogFooter},
        setting::{SettingField, SettingGroup, SettingItem, SettingPage, Settings},
    },
};

use crate::{
    assets::Themes,
    settings::{AppThemeMode, SettingsManager},
    shared,
};

pub fn settings_dialog(dialog: Dialog, _: &mut Window, cx: &mut App) -> Dialog {
    dialog
        .w(shared::DIALOG_WIDTH)
        .h(shared::DIALOG_HEIGHT)
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
                            SettingsManager::reload(cx);
                            window.close_dialog(cx);
                        },
                    ))
                    .child(
                        Button::new("settings-save")
                            .label("Save")
                            .on_click(|_, window, cx| {
                                SettingsManager::save(cx);
                                window.close_dialog(cx);
                            }),
                    ),
            ),
        )
        .on_close(|_, _, cx| {
            SettingsManager::reload(cx);
        })
        .on_ok(|_, _, cx| {
            SettingsManager::save(cx);
            true
        })
}

fn appearance_group(cx: &mut App) -> SettingGroup {
    SettingGroup::new()
        .title("Appearance")
        .item(SettingItem::new(
            "Theme Mode",
            SettingField::dropdown(
                AppThemeMode::options(),
                |cx| SharedString::from(SettingsManager::app_theme_mode(cx).to_string()),
                |value, cx| {
                    SettingsManager::set_app_theme_mode(cx, AppThemeMode::from(value));
                    Theme::change(SettingsManager::theme_mode(cx), None, cx);
                },
            ),
        ))
        .item(SettingItem::new("Theme", {
            SettingField::scrollable_dropdown(
                Themes::options(cx, cx.theme().mode),
                SettingsManager::theme,
                |value, cx| {
                    SettingsManager::set_theme(cx, value.clone());
                    Themes::apply(cx, value);
                },
            )
        }))
}
