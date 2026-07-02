mod app;
mod formatting;
mod style;
mod views;

use app::GuiApp;
use iced::{Theme, window};

pub fn run() -> iced::Result {
    iced::daemon(GuiApp::new, GuiApp::update, GuiApp::view)
        .title(GuiApp::title)
        .subscription(GuiApp::subscription)
        .theme(app_theme)
        .run()
}

fn app_theme(_: &GuiApp, _: window::Id) -> Theme {
    Theme::KanagawaDragon
}
