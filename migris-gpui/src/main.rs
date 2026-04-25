mod app;
mod assets;
mod components;
mod connections;
mod event;
mod settings;
mod shared;
mod state;
mod tabs;
mod types;

use gpui::{AppContext, WindowOptions};
use gpui_component::Root;

use crate::app::Application;

fn main() -> anyhow::Result<()> {
    // Use tokio runtime (needed for sqlx operations)
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let handle = runtime.handle();
    let _guard = handle.enter();

    let app = gpui_platform::application().with_assets(assets::Assets);
    app.run(|cx| {
        gpui_component::init(cx);
        gpui_component::Theme::global_mut(cx).scrollbar_show =
            gpui_component::scroll::ScrollbarShow::Always;

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
