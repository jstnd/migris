mod app;
mod message;
mod widgets;

use crate::app::Application;

fn main() -> iced::Result {
    iced::application(Application::new, Application::update, Application::view)
        .title("migris")
        .font(include_bytes!("../assets/fonts/lucide.ttf"))
        .run()
}
