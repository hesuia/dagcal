# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog.

## [Unreleased]

### Added
- Added `dist-gui` and `dist-tui` Cargo profiles for optimized distribution builds of the GUI and TUI frontends.

### Changed
- GUI release packaging now builds `dagcal-gui` with the dedicated `dist-gui` profile instead of the default `release` profile.

## [0.4.0] - 2026-07-05

### Added
- Added a shared `dagcal-app` crate for UI-agnostic app session state, draft input, completion, filtering, and display formatting used by frontends.
- GUI: Added entry search and All/Values/Errors filtering for the entries table.
- GUI: Show entry search controls only while searching, with View menu, Ctrl+F, Clear, and Esc controls.
- TUI: Added entry search, All/Values/Errors filtering, completions, input preview, selected-entry details, reference insertion, and manual recalculation actions backed by `dagcal-app`.

### Changed
- GUI/TUI: Route shared entry operations and frontend-facing state through `dagcal-app`.
- GUI: Show discard-confirmation prompts in a separate window instead of inline in the main window.
- GUI: Remove the confirmation prompt when deleting entries.
- GUI: Refactored app actions, window handling, file I/O, keyboard handling, and dialog views into focused modules.
- GUI: Reused shared `dagcal-app` completion menu entry helpers when building Insert menus.
- TUI: Split terminal setup, keyboard handling, rendering, app actions, selectors, formatting, and tests into focused modules.

## [0.3.0] - 2026-07-02

### Added
- GUI: Added dirty-state tracking with current file display in the window title and status bar.
- GUI: Added Save and Save As behavior that reuses the current file path after saving or loading a session.
- GUI: Added in-app confirmation prompts for deleting entries and discarding unsaved changes before loading, clearing, or quitting.
- GUI: Added a status bar that shows the latest status message, entry count, and undo/redo availability.
- GUI: Added inline input completions for named entries, result references, constants, and functions.
- GUI: Added an entry details window for full dependency and error details.
- GUI: Added per-entry recalculation to the right-click menu and a menu-bar action to recalculate all entries.
- Core: Added public APIs for manually recomputing one entry or all stored entries.
- Core: Manual recomputation now reparses and resolves stored source so expressions can recover after referenced names are defined later.

### Fixed
- GUI: Suppressed the extra console window when running Windows release builds.

## [0.2.1] - 2026-07-01

### Fixed
- Fixed the GitHub release workflow so the release job checks out the repository before creating a release.
- Passed `GH_REPO` to `gh release create` so release creation does not depend on the current working directory being a Git checkout.

### Changed
- Aligned `dagcal-core`, `dagcal-repl`, `dagcal-tui`, and `dagcal-gui` on version `0.2.1`.

## [0.2.0] - 2026-07-01

### Added
- Added a GitHub Actions workflow to run formatting checks, workspace tests, and dependency license checks on pull requests and pushes to `main`.
- GUI: Added automated packaging for Linux and Windows `dagcal-gui` binaries on tagged releases.
- Added a `cargo-deny` license policy for workspace dependencies.

### Changed
- Workspace: Started versioning the current application crates together for release `0.2.0`.
