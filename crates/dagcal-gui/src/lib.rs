mod app;
mod formatting;
mod style;
mod views;

use app::GuiApp;
use iced::Theme;

pub fn run() -> iced::Result {
    iced::application(GuiApp::new, GuiApp::update, GuiApp::view)
        .title("dagcal")
        .subscription(GuiApp::subscription)
        .theme(app_theme)
        .run()
}

fn app_theme(_: &GuiApp) -> Theme {
    Theme::KanagawaDragon
}
