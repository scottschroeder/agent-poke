# agent-poke

See at a glance which tmux windows have coding agents that are running, waiting for input, or finished — with emoji indicators right in your tab bar.

`🔄 1:project` agent working · `⏸️ 2:project` needs your attention · `✅ 3:project` done

## Quickstart

### 1. Install

```bash
cargo install --git https://github.com/scottschroeder/agent-poke
```

### 2. Add to `~/.tmux.conf`

Paste the contents of [`examples/tmux.conf.snippet`](examples/tmux.conf.snippet) into your `~/.tmux.conf`, then reload:

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

Copy [`examples/opencode-plugin.js`](examples/opencode-plugin.js) to:

- `~/.config/opencode/plugins/agent-poke.js` (global), or
- `.opencode/plugins/agent-poke.js` (project-local)
</details>

<details>
<summary><b>Codex</b></summary>

Add to `~/.codex/config.toml`:

```toml
notify = ["bash", "/absolute/path/to/examples/codex-notify.sh"]
```

See [`examples/codex-notify.sh`](examples/codex-notify.sh) for the adapter script.
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
| `@agent-indicator-notification-sound` | `off` | Play a sound on notification |

## Development

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```
