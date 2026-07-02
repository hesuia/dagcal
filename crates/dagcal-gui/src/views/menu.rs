use crate::app::{GuiApp, Message};
use crate::style::{menu_bar_style, menu_button_style};
use dagcal_core::{CompletionItem, CompletionKind};
use iced::widget::{button, container, row, text};
use iced::{Element, Fill, Length};
use iced_aw::menu::{Item, Menu, MenuBar};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MenuEntry {
    pub(super) label: String,
    pub(super) detail: Option<String>,
}

impl GuiApp {
    pub(super) fn menu_bar_view(&self) -> Element<'_, Message> {
        let menu_bar = MenuBar::new(vec![
            Item::with_menu(menu_button("File"), file_menu()),
            Item::with_menu(menu_button("Edit"), edit_menu()),
            Item::with_menu(menu_button("Insert"), insert_menu(self)),
            Item::with_menu(menu_button("Help"), help_menu()),
        ])
        .width(Length::Fill)
        .height(Length::Fixed(34.0))
        .padding([2, 4])
        .spacing(4)
        .close_on_item_click_global(true)
        .close_on_background_click_global(true)
        .style(menu_bar_style);

        container(menu_bar).width(Fill).into()
    }

    pub(super) fn constant_menu_entries(&self) -> Vec<MenuEntry> {
        menu_entries_for_kind(self.engine.completion_items(), CompletionKind::Constant)
    }

    pub(super) fn function_menu_entries(&self) -> Vec<MenuEntry> {
        menu_entries_for_kind(self.engine.completion_items(), CompletionKind::Function)
    }
}

fn file_menu() -> Menu<'static, Message, iced::Theme, iced::Renderer> {
    Menu::new(vec![
        Item::new(menu_item("Save", Message::Save)),
        Item::new(menu_item("Save As...", Message::SaveAs)),
        Item::new(menu_item("Load...", Message::Load)),
        Item::new(menu_item("Quit", Message::Quit)),
    ])
    .width(Length::Fixed(150.0))
    .max_width(180.0)
    .offset(3.0)
}

fn edit_menu() -> Menu<'static, Message, iced::Theme, iced::Renderer> {
    Menu::new(vec![
        Item::new(menu_item("Undo", Message::Undo)),
        Item::new(menu_item("Redo", Message::Redo)),
        Item::new(menu_item("Recalculate All", Message::RecalculateAll)),
        Item::new(menu_item("Clear", Message::Clear)),
    ])
    .width(Length::Fixed(150.0))
    .max_width(180.0)
    .offset(3.0)
}

fn insert_menu(app: &GuiApp) -> Menu<'static, Message, iced::Theme, iced::Renderer> {
    Menu::new(vec![
        Item::with_menu(
            submenu_item("Constants"),
            constants_menu(app.constant_menu_entries()),
        ),
        Item::with_menu(
            submenu_item("Functions"),
            functions_menu(app.function_menu_entries()),
        ),
    ])
    .width(Length::Fixed(170.0))
    .max_width(220.0)
    .offset(3.0)
}

fn help_menu() -> Menu<'static, Message, iced::Theme, iced::Renderer> {
    Menu::new(vec![
        Item::new(menu_item(
            "Keyboard shortcuts",
            Message::ShowKeyboardShortcuts,
        )),
        Item::new(menu_item("About", Message::ShowAbout)),
    ])
    .width(Length::Fixed(220.0))
    .max_width(260.0)
    .offset(3.0)
}

fn constants_menu(entries: Vec<MenuEntry>) -> Menu<'static, Message, iced::Theme, iced::Renderer> {
    let items = if entries.is_empty() {
        vec![Item::new(inert_menu_item("No constants"))]
    } else {
        entries
            .into_iter()
            .map(|entry| {
                let label = entry.label.clone();
                Item::new(menu_item(&entry.label, Message::InsertConstant(label)))
            })
            .collect()
    };

    Menu::new(items)
        .width(Length::Fixed(160.0))
        .max_width(220.0)
        .offset(3.0)
}

fn functions_menu(entries: Vec<MenuEntry>) -> Menu<'static, Message, iced::Theme, iced::Renderer> {
    let items = if entries.is_empty() {
        vec![Item::new(inert_menu_item("No functions"))]
    } else {
        entries
            .into_iter()
            .map(|entry| {
                let name = entry.label.clone();
                let label = match entry.detail {
                    Some(detail) => format!("{name} - {detail}"),
                    None => name.clone(),
                };
                Item::new(menu_item(&label, Message::InsertFunction(name)))
            })
            .collect()
    };

    Menu::new(items)
        .width(Length::Fixed(260.0))
        .max_width(340.0)
        .offset(3.0)
}

fn menu_entries_for_kind(items: Vec<CompletionItem>, kind: CompletionKind) -> Vec<MenuEntry> {
    items
        .into_iter()
        .filter(|item| item.kind == kind)
        .map(|item| MenuEntry {
            label: item.label,
            detail: item.detail,
        })
        .collect()
}

fn menu_button(label: &'static str) -> Element<'static, Message> {
    button(text(label).size(14))
        .padding([6, 10])
        .style(|_, status| menu_button_style(status))
        .into()
}

fn menu_item(label: &str, message: Message) -> Element<'static, Message> {
    button(text(label.to_string()).size(14).width(Fill))
        .width(Fill)
        .padding([7, 10])
        .style(|_, status| menu_button_style(status))
        .on_press(message)
        .into()
}

fn submenu_item(label: &'static str) -> Element<'static, Message> {
    button(text(label).size(14).width(Fill))
        .width(Fill)
        .padding([7, 10])
        .style(|_, status| menu_button_style(status))
        .into()
}

fn inert_menu_item(label: &'static str) -> Element<'static, Message> {
    row![text(label).size(14)]
        .padding([7, 10])
        .width(Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use dagcal_core::Engine;

    #[test]
    fn menu_entries_collect_runtime_constants() {
        let (mut app, _) = GuiApp::new();
        app.engine.set_constant("tau", 6);

        let entries = app.constant_menu_entries();

        assert!(entries.iter().any(|entry| entry.label == "pi"));
        assert!(entries.iter().any(|entry| entry.label == "tau"));
    }

    #[test]
    fn menu_entries_collect_function_signatures() {
        let (mut app, _) = GuiApp::new();
        app.engine
            .register_fixed_function("triple", 1, |args| Ok(args[0].clone() * 3.into()));

        let entries = app.function_menu_entries();

        assert!(entries.iter().any(|entry| {
            entry.label == "triple" && entry.detail.as_deref() == Some("1 argument(s)")
        }));
    }

    #[test]
    fn menu_entries_filter_by_kind() {
        let mut engine = Engine::new();
        engine.execute("x = 10");

        let constants = menu_entries_for_kind(engine.completion_items(), CompletionKind::Constant);

        assert!(constants.iter().all(|entry| entry.label != "x"));
    }
}
