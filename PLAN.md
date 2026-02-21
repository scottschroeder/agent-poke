# agent-poke Rust Implementation Plan

This plan assumes an event-driven design (hook invocations + tmux hooks), with no always-on daemon in the MVP.

## Goals

- Provide reliable attention signaling for coding agents in tmux.
- Support Claude and OpenCode/Codex first.
- Target macOS and Arch Linux.
- Keep integrations event-driven and lightweight.

## Non-goals (initially)

- Deep process inspection beyond basic fallback checks.
- Rich animation engine in v1.
- Full plugin marketplace packaging in v1.

## Proposed architecture (single crate, modular)

- Single crate layout:
  - `src/main.rs`: minimal entrypoint and top-level error reporting.
  - `src/cli/`: clap structs, parsing, and command dispatch.
  - `src/core/`: domain types and state transition logic.
  - `src/tmux/`: tmux interaction layer (env/options/styles/hooks helpers).
  - `src/notify/`: macOS/Linux notification backends.
  - `src/install/`: Claude/OpenCode-Codex install and uninstall integration logic.
  - `src/config/`: defaults and option resolution.
- Thin shell entrypoints can remain for tmux/hook compatibility, but should only exec the Rust binary.
- Runtime model: short-lived command execution per event.

## CLI command surface (target)

- `agent-poke state --agent <name> --state <running|needs-input|done|off> [--pane <id>]`
- `agent-poke indicator`
- `agent-poke focus-reset --pane <id> --window <id>`
- `agent-poke install [--target-dir ...] [--no-claude] [--no-codex]`
- `agent-poke uninstall [--claude] [--codex]`

## Phase 0 - Discovery and design baseline

Deliverables:

- Capture required behavior from `examples/tmux-agent-indicator/` as acceptance criteria.
- Define canonical state transitions and precedence rules.
- Document compatibility assumptions for tmux, Claude hooks, and OpenCode/Codex notify.

Acceptance criteria:

- Written behavior spec exists in repo (`docs/behavior-spec.md`).
- Every current example feature is marked as `v1`, `v1.1`, or `later`.

## Phase 1 - Crate and domain foundation

Deliverables:

- Create a single Rust crate with module skeletons.
- Add core domain types in `src/core/`:
  - `AgentName`, `AgentState`, `PaneRef`, `WindowRef`
  - transition validation logic
  - notification message model
- Add config option model for tmux options and defaults in `src/config/`.

Implementation notes (per `RUST_CLI.md` guidance):

- Keep CLI modules thin; no core business logic in argument parsing modules.
- Use `clap` derive and per-subcommand `run` methods.
- Use `anyhow::Result` with `Context` on IO boundaries.

Acceptance criteria:

- `cargo test` passes with unit tests for transition rules.
- `cargo fmt --all` and `cargo clippy --all-targets --all-features -D warnings` pass.

## Phase 2 - tmux module and state command MVP

Deliverables:

- Implement `agent-poke state` with tmux integration:
  - resolve target pane/window
  - apply/clear pane background and active border style
  - apply/restore window title styles
  - store state in tmux environment keys
- Implement notification trigger contract from state transitions.

Acceptance criteria:

- Running equivalent of `running -> needs-input -> done -> off` updates tmux styles and env markers correctly.
- Focus-reset deferred markers are set for `done` when configured.
- Existing shell tests can be adapted or mirrored to validate behavior.

## Phase 3 - focus-reset and indicator parity

Deliverables:

- Implement `agent-poke focus-reset` command.
- Implement `agent-poke indicator` output:
  - icon mapping by agent
  - active pane and same-window fallback state lookup
  - optional process detection fallback
- Wire tmux hooks in plugin bootstrap to call `focus-reset`.

Acceptance criteria:

- `needs-input` and `done` styles clear when source pane/window regains focus.
- Indicator renders expected icon for active state and window fallback.
- No persistent process required for normal operation.

## Phase 4 - cross-platform notifications and install flows

Deliverables:

- Implement notification backend in `src/notify/`:
  - macOS via `osascript`
  - Linux via `notify-send` (best-effort)
- Implement `install` / `uninstall` commands in `src/install/`:
  - patch `~/.claude/settings.json` hooks
  - patch `~/.codex/config.toml` notify command
  - install bootstrap script/config snippet
- Keep changes idempotent and reversible.

Acceptance criteria:

- Re-running install does not duplicate entries.
- Uninstall removes only managed integration entries.
- Notification message includes session and window name.

## Phase 5 - migration from example scripts

Deliverables:

- Replace shell script internals with Rust command calls.
- Preserve user-facing options and defaults where possible.
- Add migration notes for existing users.

Acceptance criteria:

- Existing tmux config patterns continue to work with minimal changes.
- Script wrappers are thin and contain no non-trivial business logic.

## Phase 6 - hardening, docs, and release

Deliverables:

- Add integration tests under `tests/` using isolated tmux socket strategy.
- Add docs:
  - quickstart
  - config reference
  - hook integration examples
  - troubleshooting for macOS/Linux notifications
- Add CI checks:
  - `fmt`, `clippy -D warnings`, `test`
  - optional shell compatibility checks for wrappers

Acceptance criteria:

- Clean CI on Linux and macOS runners.
- Manual test checklist passes for tmux + Claude + OpenCode/Codex.
- First tagged release and changelog entry.

## Suggested milestone breakdown

- Milestone A (v0.1): Phases 1-2 complete (`state` command works).
- Milestone B (v0.2): Phase 3 complete (`indicator` + `focus-reset`).
- Milestone C (v0.3): Phase 4 complete (notifications + installer).
- Milestone D (v1.0): Phases 5-6 complete (migration, docs, CI, release).

## Risks and mitigations

- tmux behavior differences by version:
  - Mitigate with explicit minimum version docs and integration tests.
- Hook schema drift in Claude/OpenCode/Codex:
  - Keep adapter layer isolated and versioned in docs.
- Notification command availability on Linux:
  - Use best-effort backend and clear fallback logging.

## Definition of done (project)

- Event-driven Rust CLI handles all core state transitions and focus reset logic.
- Tmux highlighting and system notifications work on macOS + Arch Linux.
- Claude and OpenCode/Codex integrations are installable, idempotent, and tested.
- Repository includes clear docs, tests, and release process.
