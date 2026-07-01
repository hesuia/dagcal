# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog.

## [Unreleased]

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
