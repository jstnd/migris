mod app;
mod message;
mod widgets;

use iced::{Pixels, Settings};

use crate::app::Application;

fn main() -> iced::Result {
    let settings = Settings {
        fonts: vec![include_bytes!("../assets/fonts/lucide.ttf").into()],
        default_text_size: Pixels(12.0),
        ..Default::default()
    };

    iced::application(Application::new, Application::update, Application::view)
        .title("migris")
        .settings(settings)
        .run()
}
