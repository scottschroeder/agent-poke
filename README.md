# agent-poke

See at a glance which tmux windows have coding agents that are running, waiting for input, or finished — with emoji indicators right in your tab bar.

`🔄 1:project` agent working · `🔔 2:project` needs your attention · `✅ 3:project` done

## Quickstart

### 1. Install

```bash
cargo install --git https://github.com/scottschroeder/agent-poke
```

### 2. Add to `~/.tmux.conf`

There are three pieces to wire up. See [`examples/tmux.conf.snippet`](examples/tmux.conf.snippet) for a copy-pasteable version.

**Set the binary path** so later lines can reference it:

```tmux
set -g @agent-poke-bin "$HOME/.cargo/bin/agent-poke"
```

**macOS (Homebrew) only:** tmux's `#()` subprocesses inherit a minimal system PATH that doesn't include `/opt/homebrew/bin`. agent-poke calls `tmux` internally, so it needs to know the full path. The `-r` flag prevents the variable from leaking into your interactive shells:

```tmux
set-environment -g AGENT_POKE_TMUX_BIN "/opt/homebrew/bin/tmux"
set-environment -r AGENT_POKE_TMUX_BIN
```

**Add the status call** to your `window-status-format` and `window-status-current-format`. Prepend the `#(...)` shell expansion to whatever format string you already have:

```tmux
set -g window-status-format '#(#{@agent-poke-bin} window-status --window "#{window_id}") #I:#W#{?window_flags,#{window_flags},}'
set -g window-status-current-format '#(#{@agent-poke-bin} window-status --window "#{window_id}") #I:#W#{?window_flags,#{window_flags},}'
```

**Add focus hooks** so the indicator resets when you switch to a window. Use `set-hook -g` to set the hook globally, or `set-hook -ag` to _append_ to an existing hook (useful if you already have hooks defined):

```tmux
set-hook -g pane-focus-in 'run-shell -b "#{@agent-poke-bin} focus-reset"'
set-hook -g after-select-window 'run-shell -b "#{@agent-poke-bin} focus-reset"'
set-hook -g after-select-pane 'run-shell -b "#{@agent-poke-bin} focus-reset"'
```

Then reload:

```bash
tmux source-file ~/.tmux.conf
```

> If you see `returned 127`, tmux can't find the binary. Make sure `@agent-poke-bin` points to the right path (default is `$HOME/.cargo/bin/agent-poke`).

### 3. Hook up your agent

<details>
<summary><b>Claude Code</b></summary>

Merge the contents of [`examples/claude-hooks.snippet.json`](examples/claude-hooks.snippet.json) into `~/.claude/settings.json`.

The hooks tell agent-poke when Claude starts working, asks for permission, or finishes.
</details>

<details>
<summary><b>OpenCode</b></summary>

Copy [`examples/opencode-plugin.js`](examples/opencode-plugin.js) to `~/.config/opencode/plugins/agent-poke.js`.
</details>

<details>
<summary><b>Pi agent harness</b></summary>

Copy [`examples/pi-extension.ts`](examples/pi-extension.ts) to `~/.pi/agent/extensions/agent-poke.ts`.

The extension reports pi sessions as idle, running, done, or closed and sends throttled heartbeats throughout active agent runs. Set `AGENT_POKE_BIN` in the environment where pi starts if `agent-poke` is not on pi's `PATH`:

```bash
cp examples/pi-extension.ts ~/.pi/agent/extensions/agent-poke.ts
export AGENT_POKE_BIN="$HOME/.cargo/bin/agent-poke"
```

Pi does not expose a generic permission-request lifecycle event, so this integration cannot distinguish a permission prompt from a completed run. A Pi UI extension that asks for input can call `agent-poke state --agent pi --state needs-input` and then `agent-poke state --agent pi --state running` after the user responds.
</details>

That's it. Open a tmux window, start your agent, and watch the glyphs appear.

## Customization

All options are set as tmux user variables. See [`examples/tmux.conf.snippet`](examples/tmux.conf.snippet) for the full reference.

| Variable | Default | Description |
|---|---|---|
| `@agent-indicator-window-status-running-glyph` | `🔄` | Glyph while agent is working |
| `@agent-indicator-window-status-needs-input-glyph` | `⏸️` | Glyph when agent needs attention |
| `@agent-indicator-window-status-done-glyph` | `✅` | Glyph when agent finishes |
| `@agent-indicator-running-heartbeat-timeout-ms` | `30000` | Heartbeat timeout before marking stale |
| `@agent-indicator-notification-persistent` | `off` | Keep desktop notifications until dismissed |
| `@agent-indicator-notification-urgency` | `critical` | Desktop notification urgency level |
| `@agent-indicator-notification-sound` | `on` | Play a sound on notification |

## Development

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```
