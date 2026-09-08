mod app;
mod assets;
mod components;
mod connections;
mod events;
mod secrets;
mod settings;
mod shared;
mod size;
mod state;
mod tabs;
mod types;

use gpui_kit::{
    AppContext, WindowOptions,
    base::ScrollbarMode,
    component::{Root, Theme},
};

#[cfg(target_os = "windows")]
use windows_native_keyring_store::Store;

#[cfg(target_os = "macos")]
use apple_native_keyring_store::keychain::Store;

use crate::app::Application;

fn main() -> anyhow::Result<()> {
    // Use tokio runtime (needed for sqlx operations)
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let handle = runtime.handle();
    let _guard = handle.enter();

    // Set keyring store for storing secrets
    keyring_core::set_default_store(Store::new()?);

    let app = gpui_kit::platform::application().with_assets(assets::Assets);
    app.run(|cx| {
        gpui_kit::init(cx);
        Theme::global_mut(cx).scrollbar_mode = ScrollbarMode::Always;

        cx.on_app_quit(|_| {
            keyring_core::unset_default_store();
            async {}
        })
        .detach();

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                app::init(window, cx);
                window.activate_window();

                let view = cx.new(|cx| Application::new(window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            })
        })
        .detach();
    });

    Ok(())
}
