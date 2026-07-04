use super::{GuiApp, Message};
use iced::{Size, Task, window};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HelpTopic {
    KeyboardShortcuts,
    About,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Confirmation {
    Clear,
    Load,
    Quit,
    CloseMain(window::Id),
}

impl GuiApp {
    pub(super) fn open_help_window(&mut self, topic: HelpTopic) -> Task<Message> {
        self.help_topic = topic;

        if self.help_window.is_some() {
            self.session.status = "Help is already open".to_string();
            return Task::none();
        }

        let (id, open_window) = window::open(window::Settings {
            size: Size::new(520.0, 420.0),
            min_size: Some(Size::new(420.0, 320.0)),
            exit_on_close_request: false,
            ..window::Settings::default()
        });

        self.help_window = Some(id);
        self.session.status = "Opened help".to_string();

        open_window.discard()
    }

    pub(super) fn open_details_window(&mut self, id: dagcal_app::ExpressionId) -> Task<Message> {
        self.details_target = Some(id);

        if self.details_window.is_some() {
            self.session.status = format!("Showing details for {id}");
            return Task::none();
        }

        let (window, open_window) = window::open(window::Settings {
            size: Size::new(640.0, 460.0),
            min_size: Some(Size::new(480.0, 320.0)),
            exit_on_close_request: false,
            ..window::Settings::default()
        });

        self.details_window = Some(window);
        self.session.status = format!("Opened details for {id}");

        open_window.discard()
    }

    pub(super) fn window_closed(&mut self, id: window::Id) -> Task<Message> {
        if self.help_window == Some(id) {
            self.help_window = None;
            return window::close(id);
        }

        if self.details_window == Some(id) {
            self.details_window = None;
            self.details_target = None;
            return window::close(id);
        }

        if self.confirmation_window == Some(id) {
            self.confirmation_window = None;
            self.pending_confirmation = None;
            self.session.status = "Action cancelled".to_string();
            return window::close(id);
        }

        if self.main_window == Some(id) {
            if self.is_dirty() {
                return self.request_confirmation(Confirmation::CloseMain(id));
            }
            self.main_window = None;
            return iced::exit();
        }

        Task::none()
    }

    pub(super) fn confirm_pending(&mut self) -> Task<Message> {
        let Some(confirmation) = self.pending_confirmation.take() else {
            return Task::none();
        };
        let close_confirmation = self.close_confirmation_window();

        let action = match confirmation {
            Confirmation::Clear => {
                self.perform_clear();
                Task::none()
            }
            Confirmation::Load => self.start_load(),
            Confirmation::Quit => iced::exit(),
            Confirmation::CloseMain(_) => {
                self.main_window = None;
                iced::exit()
            }
        };

        Task::batch([close_confirmation, action])
    }

    pub(super) fn cancel_confirmation(&mut self) -> Task<Message> {
        self.pending_confirmation = None;
        self.session.status = "Action cancelled".to_string();
        self.close_confirmation_window()
    }

    pub(super) fn quit(&mut self) -> Task<Message> {
        if self.is_dirty() {
            self.request_confirmation(Confirmation::Quit)
        } else {
            iced::exit()
        }
    }

    pub(super) fn request_confirmation(&mut self, confirmation: Confirmation) -> Task<Message> {
        self.pending_confirmation = Some(confirmation);
        self.session.status = match confirmation {
            Confirmation::Clear => "Confirm clear".to_string(),
            Confirmation::Load => "Confirm load".to_string(),
            Confirmation::Quit | Confirmation::CloseMain(_) => "Confirm quit".to_string(),
        };

        if self.confirmation_window.is_some() {
            return Task::none();
        }

        let (id, open_window) = window::open(window::Settings {
            size: Size::new(380.0, 180.0),
            min_size: Some(Size::new(340.0, 160.0)),
            exit_on_close_request: false,
            ..window::Settings::default()
        });
        self.confirmation_window = Some(id);

        open_window.discard()
    }

    fn close_confirmation_window(&mut self) -> Task<Message> {
        let Some(id) = self.confirmation_window.take() else {
            return Task::none();
        };

        window::close(id)
    }
}
