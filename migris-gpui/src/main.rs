mod app;
mod assets;
mod components;
mod models;

use gpui::{AppContext, WindowOptions};
use gpui_component::{Root, ThemeMode};

use crate::app::Application;

fn main() -> anyhow::Result<()> {
    let app = gpui_platform::application().with_assets(assets::Assets);

    // Use tokio runtime (needed for sqlx operations)
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let handle = runtime.handle();
    let _guard = handle.enter();

    app.run(|cx| {
        gpui_component::init(cx);
        gpui_component::Theme::change(ThemeMode::Dark, None, cx);

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                window.activate_window();

                let view = cx.new(|cx| Application::new(window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            })
        })
        .detach();
    });

    Ok(())
}
