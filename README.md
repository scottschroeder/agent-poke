# agent-poke

Event-driven tmux attention signaling for coding agents.

This project is being implemented as a Rust CLI that reacts to hook events from Claude and OpenCode/Codex.

## Status

Initial Rust foundation is in place:

- Single-crate modular CLI structure (`src/cli`, `src/core`, `src/tmux`, `src/notify`, `src/install`, `src/config`)
- State command skeleton (`state`) with tmux environment marker updates
- Indicator command skeleton (`indicator`) based on active pane state markers
- Focus reset command skeleton (`focus-reset`) for focus-driven marker cleanup
- Core domain model with state parsing/validation and unit tests
- Phase 0 behavior spec: `docs/behavior-spec.md`

The implementation is not feature-complete yet. Styling parity, full install/uninstall flows, and test parity with the shell prototype are still in progress.

## Commands (current)

```bash
cargo run -- state --agent claude --state running
cargo run -- state --agent claude --state needs-input
cargo run -- state --agent claude --state done
cargo run -- state --agent claude --state off

cargo run -- indicator
cargo run -- focus-reset --pane %1 --window @1
```

## Development

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

## Plan

See `PLAN.md` for the phased implementation roadmap.
