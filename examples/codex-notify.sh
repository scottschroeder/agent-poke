#!/usr/bin/env bash

set -euo pipefail

raw_input="${1:-}"

event="agent-turn-complete"
if [[ -n "$raw_input" ]]; then
    if [[ "$raw_input" == \{* ]]; then
        event="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1]).get("type", "agent-turn-complete"))' "$raw_input" 2>/dev/null || true)"
        event="${event:-agent-turn-complete}"
    else
        event="$raw_input"
    fi
fi

case "$event" in
    start|session-start|turn-start|working)
        state="running"
        ;;
    approval-requested|permission*|approve*|needs-input|input-required|ask-user)
        state="needs-input"
        ;;
    agent-turn-complete|complete|done|stop|error|fail*)
        state="done"
        ;;
    *)
        state="done"
        ;;
esac

agent-poke state --agent codex --state "$state"
