mod app;
mod components;

use gpui::{AppContext, WindowOptions};
use gpui_component::{Root, ThemeMode};

use crate::app::Application;

fn main() {
    let app = gpui_platform::application().with_assets(gpui_component_assets::Assets);

    app.run(|cx| {
        gpui_component::init(cx);
        gpui_component::Theme::change(ThemeMode::Dark, None, cx);

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|cx| Application::new(window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            })
        })
        .detach();
    });
}
