use super::document::path_label;
use super::{Confirmation, GuiApp, Message};
use dagcal_app::{AppSession, EngineSnapshot};
use iced::Task;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum SaveResult {
    Cancelled,
    Saved(PathBuf, EngineSnapshot),
    Failed(String),
}

#[derive(Debug, Clone)]
pub enum LoadResult {
    Cancelled,
    Loaded(PathBuf, EngineSnapshot),
    Failed(String),
}

impl GuiApp {
    pub(super) fn save(&mut self) -> Task<Message> {
        if let Some(path) = self.current_path.clone() {
            self.save_to_path(path)
        } else {
            self.save_as()
        }
    }

    pub(super) fn save_as(&mut self) -> Task<Message> {
        let snapshot = self.session.snapshot();
        self.set_status("Saving...");

        Task::perform(
            save_snapshot_with_dialog(snapshot, self.current_path.clone()),
            Message::SaveFinished,
        )
    }

    fn save_to_path(&mut self, path: PathBuf) -> Task<Message> {
        let snapshot = self.session.snapshot();
        self.set_status(format!("Saving {}...", path_label(&path)));

        Task::perform(save_snapshot_to_path(path, snapshot), Message::SaveFinished)
    }

    pub(super) fn load(&mut self) -> Task<Message> {
        if self.is_dirty() {
            return self.request_confirmation(Confirmation::Load);
        }

        self.start_load()
    }

    pub(super) fn start_load(&mut self) -> Task<Message> {
        self.set_status("Loading...");

        Task::perform(load_snapshot(), Message::LoadFinished)
    }

    pub(super) fn finish_save(&mut self, result: SaveResult) -> super::effects::UiEffect {
        let status = match result {
            SaveResult::Cancelled => "Save cancelled".to_string(),
            SaveResult::Saved(path, snapshot) => {
                let status = format!("Saved {}", path_label(&path));
                self.current_path = Some(path);
                self.saved_snapshot = snapshot;
                status
            }
            SaveResult::Failed(error) => format!("Save failed: {error}"),
        };
        self.set_status(status);

        super::effects::UiEffect::None
    }

    pub(super) fn finish_load(&mut self, result: LoadResult) -> super::effects::UiEffect {
        match result {
            LoadResult::Cancelled => {
                self.set_status("Load cancelled");
            }
            LoadResult::Loaded(path, snapshot) => {
                let status = format!("Loaded {}", path_label(&path));
                match AppSession::from_snapshot(snapshot) {
                    Ok(mut session) => {
                        session.dispatch(dagcal_app::AppAction::SetStatus(status));
                        self.current_path = Some(path);
                        self.saved_snapshot = session.snapshot();
                        self.session = session;
                    }
                    Err(error) => {
                        self.set_status(format!(
                            "Load failed: could not restore snapshot ({error})"
                        ));
                    }
                }
            }
            LoadResult::Failed(error) => {
                self.set_status(format!("Load failed: {error}"));
            }
        }

        super::effects::UiEffect::None
    }
}

async fn save_snapshot_with_dialog(
    snapshot: EngineSnapshot,
    current_path: Option<PathBuf>,
) -> SaveResult {
    let file_name = current_path
        .as_deref()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("dagcal-session.json");

    let Some(path) = rfd::FileDialog::new()
        .add_filter("Dagcal session", &["json"])
        .set_file_name(file_name)
        .save_file()
    else {
        return SaveResult::Cancelled;
    };

    save_snapshot_to_path(path, snapshot).await
}

async fn save_snapshot_to_path(path: PathBuf, snapshot: EngineSnapshot) -> SaveResult {
    let json = match serde_json::to_string_pretty(&snapshot) {
        Ok(json) => json,
        Err(error) => return SaveResult::Failed(format!("could not encode JSON ({error})")),
    };

    match std::fs::write(&path, json) {
        Ok(()) => SaveResult::Saved(path, snapshot),
        Err(error) => SaveResult::Failed(format!("could not write file ({error})")),
    }
}

async fn load_snapshot() -> LoadResult {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("Dagcal session", &["json"])
        .pick_file()
    else {
        return LoadResult::Cancelled;
    };

    load_snapshot_from_path(path)
}

fn load_snapshot_from_path(path: PathBuf) -> LoadResult {
    let json = match std::fs::read_to_string(&path) {
        Ok(json) => json,
        Err(error) => return LoadResult::Failed(format!("could not read file ({error})")),
    };

    let snapshot = match serde_json::from_str(&json) {
        Ok(snapshot) => snapshot,
        Err(error) => return LoadResult::Failed(format!("could not parse JSON ({error})")),
    };

    LoadResult::Loaded(path, snapshot)
}
