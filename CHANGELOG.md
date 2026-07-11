# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog.

## [Unreleased]

### Added
- Core: Added allocation-free borrowed entry views and stable-ID ordered entry iterators.
- App: Added typed `AppAction` and `AppEffect` APIs for frontend-independent state transitions.
- Added `dist-gui` and `dist-tui` Cargo profiles for optimized distribution builds of the GUI and TUI frontends.
- GUI: Added shortcuts for new entry, save, save as, load, quit, edit selected entry, and recalculation actions.
- GUI: Added double-click editing for entry rows.

### Changed
- GUI/TUI: Show `Error` instead of full diagnostics in expression completion suggestions; keep details in the selected entry details view.
- App/GUI/TUI: Encapsulated shared session state behind actions and borrowed selectors, removed the legacy `SessionChange` alias and GUI `Deref` access, and migrated both frontends to the reducer boundary.
- App: Added explicit actions for frontend status messages, non-materializing editor updates, input resets, selection clearing, and empty-draft cleanup.
- GUI: Separated Iced task/effect handling from shared state mutation and moved GUI tests to the same public app contract used by production code.
- Core: Update dependency edges incrementally when entries change instead of rebuilding the complete graph.
- Core: Removed the redundant `DependencyIndex` forwarding layer; recomputation now depends directly on the internal `ReferenceGraph`.
- Core: Isolated undo/redo history from evaluation session state and use ordered entry storage to avoid repeated query sorting.
- App/GUI/TUI: Route shared application operations through the app reducer and provide allocation-free filtered-entry selectors.
- GUI: Removed forwarding-only action methods and translate app effects at the Iced adapter boundary.
- GUI/TUI: Omit the currently edited or drafted entry from completion dropdown suggestions.
- GUI/TUI: Show current results on named entry and `$n` completion suggestions.
- GUI release packaging now builds `dagcal-gui` with the dedicated `dist-gui` profile instead of the default `release` profile.
- GUI: Anchored the menu bar to the top edge and the status bar to the bottom edge of the main window while keeping workspace padding inside the content area.
- GUI: Keep the selected entry visible when moving through entries with Arrow Up or Arrow Down.
- GUI: Removed the inline Details button and compacted the selected-entry preview area.
- GUI: Keep the entries table header visible while scrolling entry rows.
- GUI: Make the expression completion dropdown scrollable when candidates exceed the visible area.
- GUI: Keep the selected completion candidate visible while moving through the dropdown with Arrow Up or Arrow Down.

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
