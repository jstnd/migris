mod app;
mod message;
mod widgets;

use crate::app::Application;

#[tokio::main]
async fn main() -> iced::Result {
    iced::application(Application::new, Application::update, Application::view)
        .title("migris")
        .run()
}
