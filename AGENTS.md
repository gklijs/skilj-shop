# Repository Guidelines

## Project Structure & Module Organization

The Rust demo entry point is `src/main.rs`. Domain logic and policies live in
`src/shop.rs`, demo identities and seeded orders in `src/demo.rs`, GraphQL
examples in `queries/`, and integration tests in `tests/`.

## Build, Test, and Development Commands

Use `cargo build --locked` to build and `cargo run --locked` to start the demo.
See `README.md` for system prerequisites, the dedicated PostgreSQL fallback,
and the GraphQL walkthrough. Use `cargo fmt --check` and
`cargo clippy --locked --all-targets -- -D warnings` for formatting and linting.

## Coding Style & Naming Conventions

Use standard Rust naming and `rustfmt` formatting. Keep changes focused
and avoid unrelated formatting edits.

## Testing Guidelines

Run `cargo test --locked` to discover and run tests under `tests/`.
`cargo test --locked --test access_control` runs the authorization integration
test, which requires embedded PostgreSQL at runtime. Add tests for new
behavior and regression tests for bug fixes. Use descriptive test names
that explain the expected behavior.

## Commit & Pull Request Guidelines

Use concise, imperative commit subjects. For issue-related work, include `Fixes #120` when fully resolving the issue or `Refs #120` otherwise.

Pull requests should explain the change, link relevant issues, and report validation performed or why checks could not run. Include screenshots for visible interface changes.

## Agent Instructions

Leave all Git commits and pushes to the user. Do not commit, push, or request permission to do either. Never add credentials, tokens, or private configuration to tracked files.
