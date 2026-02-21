# Behavior Specification

This document captures the expected runtime behavior for `agent-poke` based on `examples/tmux-agent-indicator`.

## Scope

- Target environments: tmux on macOS and Arch Linux.
- Integrations: Claude hooks and OpenCode/Codex notify command.
- Runtime model: event-driven command invocations plus tmux hook callbacks.

## Core state model

Per pane, an agent can be in one of four states:

- `running`
- `needs-input`
- `done`
- `off`

State drives three visual channels and one notification channel:

- pane background style
- active pane border style
- window title styles (`window-status-style`, `window-status-current-style`)
- user notifications (tmux display-message plus optional system command)

## Event mapping

### Claude

- `UserPromptSubmit` -> `off` then `running`
- `PermissionRequest` -> `needs-input`
- `Stop` -> `done`

### OpenCode/Codex

- start-like events (`start`, `session-start`, `turn-start`, `working`) -> `running`
- permission/input events (`permission*`, `approve*`, `needs-input`, `input-required`, `ask-user`) -> `needs-input`
- completion/error events (`agent-turn-complete`, `complete`, `done`, `stop`, `error`, `fail*`) -> `done`
- unknown event -> `done`

## Style behavior

Defaults by state:

- `running`: border/background default, no title styling
- `needs-input`: yellow border, yellow title bg, black title fg
- `done`: green border, red title bg, black title fg
- `off`: restore baseline styles and clear markers

Option resolution semantics:

- If an option is unset, use default value.
- If an option is explicitly set to empty string, skip applying that property.
- If background/border channel is globally disabled, reset that channel to baseline style.

## Focus reset behavior

With `@agent-indicator-reset-on-focus` enabled:

- `done` styling is deferred and persists until source pane/window regains focus.
- on focus of source pane/window, clear deferred style and restore original window title/border style.
- ignore stale focus hook calls for panes/windows that are not currently focused.

`needs-input` behavior:

- keep styling while user is away.
- clear styles when user returns focus to source pane/window.

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

- State transitions (`running`, `needs-input`, `done`, `off`)
- tmux env state markers
- pane border/background + title style apply/restore
- focus-driven reset for `done` and `needs-input`
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
