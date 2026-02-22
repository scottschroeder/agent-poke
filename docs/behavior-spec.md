# Behavior Specification

This document captures the expected runtime behavior for `agent-poke` based on `examples/tmux-agent-indicator`.

## Scope

- Target environments: tmux on macOS and Arch Linux.
- Integrations: Claude hooks and OpenCode/Codex notify command.
- Runtime model: event-driven command invocations plus tmux hook callbacks.

## Core state model

Per pane, an agent can be in one of five tracked states:

- `idle`
- `running`
- `needs-input`
- `done`
- `closed`

There is also an implicit `none` state for panes with no record.

State drives glyph/status output and notifications:

- pane and window glyph outputs
- user notifications (tmux display-message plus optional system command)

## Event mapping

### Claude

- `UserPromptSubmit` -> `running`
- `PermissionRequest` -> `needs-input`
- `Stop` -> `done`

### OpenCode/Codex

- start-like events (`start`, `session-start`, `turn-start`, `working`) -> `running`
- permission/input events (`permission*`, `approve*`, `needs-input`, `input-required`, `ask-user`) -> `needs-input`
- completion/error events (`agent-turn-complete`, `complete`, `done`, `stop`, `error`, `fail*`) -> `done`
- unknown event -> `done`

## Liveness and heartbeat behavior

- transition events change state
- heartbeat-only events do not change state and only refresh heartbeat timestamp
- `running` transitions back to `idle` when heartbeat timeout expires
- tracked panes transition to `closed` when process fingerprint no longer matches
- `closed` records are retained for a configurable TTL then garbage-collected to `none`

## Focus reset behavior

With `@agent-indicator-reset-on-focus` enabled:

- `done` persists until source pane/window regains focus.
- on focus of source pane/window, clear attention state and return to `idle`.
- ignore stale focus hook calls for panes/windows that are not currently focused.

`needs-input` behavior:

- keep state while user is away.
- clear to `idle` when user returns focus to source pane/window.

## Notification behavior

Notification triggers:

- enabled by default.
- default notify states: `needs-input,done`.

Message format supports placeholders:

- `#{agent_name}`
- `#{agent_state}`
- `#{session_name}`
- `#{window_name}`
- `#{window_index}`

Notification outputs:

- tmux `display-message` with optional duration.
- optional external command with environment variables:
  - `AGENT_NAME`
  - `AGENT_STATE`
  - `AGENT_SESSION`
  - `AGENT_WINDOW`

## Indicator behavior

Priority order:

1. state for active pane
2. state for non-active panes in current window
3. process detection fallback in active pane
4. process detection fallback in other panes in current window
5. empty output

Indicator output uses per-agent icon mapping with a default icon.

## Feature targeting

### v1

- State transitions (`idle`, `running`, `needs-input`, `done`, `closed`)
- tmux env state markers
- glyph/status state output
- focus-driven reset for `done` and `needs-input` back to `idle`
- notification formatting including session/window
- indicator icon for state in active pane and same-window fallback
- Claude hook integration
- OpenCode/Codex notify integration

### v1.1

- Process detection fallback in indicator
- Optional running animation support
- Additional integration adapters for other agents

### later

- Rich theming UX (interactive color setup wizard parity)
- More advanced multi-window/session aggregation strategies
