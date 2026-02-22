# agent-poke

Event-driven tmux attention signaling for coding agents.

`agent-poke` updates tmux visuals and notifications when an agent is running, needs input, or is done.

## Install

No installer script is required. Install the binary and then add config snippets manually.

### Option A: build/install with Cargo

From a local checkout:

```bash
cargo install --path .
```

From GitHub directly:

```bash
cargo install --git https://github.com/scottschroeder/agent-poke
```

### Option B: install from GitHub release tarball

1. Download the matching `agent-poke-<version>-<target>.tar.gz` from Releases.
2. Extract and move the binary into your `PATH`.

Example:

```bash
tar -xzf agent-poke-0.1.0-x86_64-unknown-linux-gnu.tar.gz
install -m 0755 agent-poke-0.1.0-x86_64-unknown-linux-gnu/agent-poke ~/.local/bin/agent-poke
```

## Configure tmux

Add this to `~/.tmux.conf`:

```tmux
# status segment
set -g @agent-poke-bin "$HOME/.cargo/bin/agent-poke"

# minimal: prefix each window label with state glyph
set -g window-status-format '#(#{@agent-poke-bin} window-status --window "#{window_id}" 2>/dev/null) #I:#W#{?window_flags,#{window_flags},}'
set -g window-status-current-format '#(#{@agent-poke-bin} window-status --window "#{window_id}" 2>/dev/null) #I:#W#{?window_flags,#{window_flags},}'

# default window-status glyphs:
# running 🔄, needs-input ⏸️, done ✅

# focus hooks to clear deferred done/needs-input styling
set-hook -ag pane-focus-in 'run-shell -b "#{@agent-poke-bin} focus-reset >/dev/null 2>&1"'
set-hook -ag after-select-window 'run-shell -b "#{@agent-poke-bin} focus-reset >/dev/null 2>&1"'
set-hook -ag after-select-pane 'run-shell -b "#{@agent-poke-bin} focus-reset >/dev/null 2>&1"'
```

Reload tmux:

```bash
tmux source-file ~/.tmux.conf
```

Reference template: `examples/tmux.conf.snippet`.

If you see `returned 127`, tmux cannot find `agent-poke`. Set `@agent-poke-bin` to an absolute path (as above), then reload tmux.

## Configure Claude hooks

Add these commands to your Claude hook config (`~/.claude/settings.json`) so Claude events drive state transitions:

- `UserPromptSubmit` -> `agent-poke state --agent claude --state running`
- `PermissionRequest` -> `agent-poke state --agent claude --state needs-input`
- `Stop` -> `agent-poke state --agent claude --state done`

Reference template: `examples/claude-hooks.snippet.json`.

## Configure OpenCode (recommended)

OpenCode integration is plugin-based. Copy `examples/opencode-plugin.js` to one of:

- `~/.config/opencode/plugins/agent-poke.js` (global)
- `.opencode/plugins/agent-poke.js` (project)

The plugin maps OpenCode events to `agent-poke` states:

- `session.status busy|retry` -> `running`
- `permission.asked` -> `needs-input`
- `session.status idle`, `session.idle`, `session.error` -> `done`


## Configure Codex (optional compatibility)

If you also use Codex, point notify to the adapter script in your config (for example `~/.codex/config.toml`):

```toml
notify = ["bash", "/absolute/path/to/agent-poke/examples/codex-notify.sh"]
```

Current Codex notifies by invoking the command with a JSON payload argument (for example `{"type":"agent-turn-complete",...}`). The adapter handles this format and also accepts older plain-string event names.

Reference templates: `examples/codex-notify.sh` and `examples/codex-config.snippet.toml`.

## Useful tmux options

```tmux
# Window-status glyphs (defaults: running='🔄', needs-input='⏸️', done='✅')
set -g @agent-indicator-window-status-running-glyph '🔄'
set -g @agent-indicator-window-status-needs-input-glyph '⏸️'
set -g @agent-indicator-window-status-done-glyph '✅'

# Notification behavior
# Keep notifications sticky until dismissed (default: off)
set -g @agent-indicator-notification-persistent 'off'

# Urgency level (default: critical)
set -g @agent-indicator-notification-urgency 'critical'

# Sound hint (default: off)
set -g @agent-indicator-notification-sound 'off'

# Optional freedesktop sound name when sound is on
set -g @agent-indicator-notification-sound-name 'dialog-information'
```

System notification format is fixed:

- title: `opencode` / `claude` (agent name)
- body: `[Done|Needs Attention] <session> > <window>`

## Current command surface

```bash
agent-poke state --agent claude --state running
agent-poke state --agent claude --state needs-input
agent-poke state --agent claude --state done
agent-poke state --agent claude --state off

agent-poke opencode-event --event session.status --status busy --event-ts-ms 1730000000000
agent-poke opencode-event --event permission.asked --event-ts-ms 1730000000001
agent-poke opencode-event --event session.idle --event-ts-ms 1730000000002

agent-poke window-status --window @1
agent-poke focus-reset --pane %1 --window @1
agent-poke debug stress --iterations 50 --sleep-ms 10
agent-poke debug state
agent-poke debug poll --interval-ms 500
agent-poke debug clear-state
```

## Development

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

## Plan

See `PLAN.md` for the phased implementation roadmap.
