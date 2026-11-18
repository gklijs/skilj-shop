# Repository Guidelines

## Project Structure & Module Organization

- `src/main.rs` starts the loopback HTTP server and chooses a database.
  `src/lib.rs` registers the `shop` bounded context with skilj.
- `src/shop.rs` defines `PlaceOrder`, `OrderPlaced`, both order projections,
  and their access policies. `src/demo.rs` configures demo identities and
  grants, signs JWTs, and seeds purchases through `PlaceOrder`.
- `src/storefront.rs` serves the public routes and assets from `storefront/`.
  `queries/` contains seller, finance, and introspection examples.

## Domain Boundaries

Keep the public catalog's list prices and product copy independent of seeded
purchases. Sellers read fulfillment details only for their own orders.
Finance reads payment details across sellers. Both roles are read-only.
Preserve the projection capability gates, seller owner scope, and matching
private-field rules on the command and event when changing the order model.
See [How access is enforced](README.md#how-access-is-enforced).

## Project Skills

Codex discovers these skills from the checkout. Read the linked skill for
the relevant task.

- [skilj-shop-storefront](.agents/skills/skilj-shop-storefront/SKILL.md) for
  product cards, public list prices, and product assets.
- [skilj-shop-projections](.agents/skills/skilj-shop-projections/SKILL.md) for
  role-specific projection fields, access gates, and seller isolation.

See [Review the project skills](README.md#review-the-project-skills) when
upgrading skilj.

## Build, Test, and Development Commands

The demo pins `skilj` and `skilj-core` to `0.4.0` and requires Rust 1.95 or
newer. Run `cargo build --locked` to build and `cargo run --locked` to launch
from the repository root. See the [README](README.md#quickstart) for system
prerequisites, the storefront, and the GraphQL walkthrough, and
[database setup](README.md#database-setup-and-troubleshooting) for the
dedicated PostgreSQL fallback. Use `cargo fmt --check` and
`cargo clippy --locked --all-targets -- -D warnings` for formatting and linting.

## Coding Style & Naming Conventions

Use standard Rust naming and `rustfmt` formatting. Keep changes focused
and avoid unrelated formatting edits.

## Testing Guidelines

Run `cargo test --locked` for the full suite. `cargo test --locked --test orders`
checks command validation and duplicate order IDs without PostgreSQL.
`cargo test --locked --test access_control` checks role-specific GraphQL
reads, seller isolation, and public storefront responses. That test starts
its own embedded PostgreSQL instance. See [Develop and verify](README.md#develop-and-verify)
for runtime requirements. Add tests for new behavior and regression tests
for bug fixes. Use descriptive test names that explain the expected behavior.

## Commit & Pull Request Guidelines

Use concise, imperative commit subjects. For issue-related work, include `Fixes #120` when fully resolving the issue or `Refs #120` otherwise.

Pull requests should explain the change, link relevant issues, and report validation performed or why checks could not run. Include screenshots for visible interface changes.

## Agent Instructions

Leave all Git commits and pushes to the user. Do not commit, push, or request permission to do either. Never add credentials, tokens, or private configuration to tracked files.
