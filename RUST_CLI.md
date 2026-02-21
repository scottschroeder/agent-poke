# Rust CLI Guidance

This document captures general guidance for building Rust CLI applications. It is intended to be reusable across projects.

## Workspace or module layout

- For workspace projects, use a dedicated CLI crate and one or more library crates for domain logic.
- For single-crate projects, use modules to separate CLI orchestration from reusable domain logic.
- Keep reusable, domain-centric types and logic in library crates or modules; keep CLI crates or modules thin.
- Place example inputs or fixtures in top-level folders that are clearly labeled (for docs and tests).

## Crate responsibilities

- CLI crate/module
  - Owns argument parsing, user-facing output, and command dispatch.
  - Keeps the entrypoint small; delegates real work to modules or library crates.
  - Does validation and IO coordination, not core business logic.
- Domain Modules/Crates
  - Provide domain types, parsing, and behavior used by the CLI or other crates.
  - Keep pure logic here to make testing simple and reuse easy.

## CLI structure

- Use `clap` derive for parsing; keep parsing separate from execution.
- Provide a `run` method on subcommands or command structs that performs the work.
- Organize subcommands into dedicated modules; avoid a single monolithic CLI file.
- Keep CLI help text user-focused with clear flag names.
- Treat the CLI as orchestration: translate arguments/config into domain objects and delegate behavior to the domain modules that own that behavior. Keep CLI-specific helpers local, but move non-trivial command logic to the domain modules where it belongs.

## Error handling

- Use `anyhow::Result` for fallible CLI operations.
- Add context with `anyhow::Context` or `with_context`.
- Prefer `?` over `unwrap`/`expect` (allow `unwrap` only in tests).
- Log errors before returning when users need extra feedback.

## Logging and output

- Use the `log` crate macros and qualify them (for example, `log::debug!`).
- Avoid printing to stdout/stderr except for intended CLI output.
- Respect `verbose` flags when choosing log levels.

## Testing

- Unit tests live next to code (`mod tests`).
- Integration tests go in `tests/` directories.
- Name tests for behavior, not implementation details.
- Avoid network access in tests unless required.
- Don't write overly specific, brittle tests.
- When evaluating what tests to write, focus on core logic. 


## Linting and quality

- Treat warnings as errors in CI (`-D warnings`).
- Prefer explicit lint allowances over broad `allow` attributes.

## Suggested development flow

- `cargo fmt --all`
- `cargo clippy --all-targets --all-features -D warnings`
- `cargo test`
- `cargo run -p <cli-crate> -- <args>`
