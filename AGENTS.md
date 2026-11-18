# Repository Guidelines

## Project Structure & Module Organization

The `skilj-shop` workspace currently contains no application source, tests, assets, or dependency manifests. No implementation language or framework is established. The existing hidden directories contain no project documentation; do not treat them as application modules.

When introducing the initial implementation, document its actual layout in `README.md` and update this guide. Prefer clear boundaries between application code, tests, and static assets; adopt directories such as `src/`, `tests/`, and `public/` only when appropriate for the chosen framework.

## Build, Test, and Development Commands

No install, development, build, lint, or test commands are currently configured. Do not assume commands such as `npm test` or `make build` work.

When adding tooling, document exact commands and prerequisites in `README.md`. Include dependency installation, local startup, production builds, formatting, linting, and testing where applicable. Commit the relevant manifests and lockfiles so contributors can reproduce the environment.

## Coding Style & Naming Conventions

No formatter, linter, or indentation standard is established. Follow the chosen language's conventions and configure its formatter before introducing substantial code. Keep indentation consistent within each file. Use descriptive names and follow framework conventions for filenames and module organization. Keep changes focused and avoid unrelated formatting edits.

## Testing Guidelines

No test framework or coverage threshold exists yet. Add tests for new behavior and regression tests for bug fixes once executable code is introduced. Use descriptive test names that explain the expected behavior. Document test discovery patterns and the exact test command alongside the selected framework.

## Commit & Pull Request Guidelines

Git history is unavailable in this workspace, so historical commit conventions cannot be verified. Use concise, imperative commit subjects. For issue-related work, include `Fixes #120` when fully resolving the issue or `Refs #120` otherwise.

Pull requests should explain the change, link relevant issues, and report validation performed or why checks could not run. Include screenshots for visible interface changes.

## Agent Instructions

Leave all Git commits and pushes to the user. Do not commit, push, or request permission to do either. Never add credentials, tokens, or private configuration to tracked files.
