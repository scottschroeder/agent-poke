
# What this is.

Tooling to help notify the user when a coding agent needs attention.

This should work for both opencode and claude.

While things should ideally work under any circumstances, we are specifically targetting:
- opencode/claude running in tmux
- runing on either archlinux or macos

When agents are done, or promt the user for permissions or questions, we want to inform the user
- tmux highlighting
- system notification
  - this notification should include the tmux session & window name
  - ideally this notification should clear if the tmux pane is focused, but thats a nice-to-have


# Examples

See the `examples/` directory for inspirations.

- See opencode docs: https://opencode.ai/docs/plugins/
- See claude docs: https://code.claude.com/docs/en/hooks-guide
