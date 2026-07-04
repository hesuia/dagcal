# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog.

## [Unreleased]

### Added
- GUI: Added entry search and All/Values/Errors filtering for the entries table.
- GUI: Show entry search controls only while searching, with View menu, Ctrl+F, Clear, and Esc controls.

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
