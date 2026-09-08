import { execFile } from "node:child_process";
import { promisify } from "node:util";
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";

const execFileAsync = promisify(execFile);
const agentPokeBin = process.env.AGENT_POKE_BIN ?? "agent-poke";
const HEARTBEAT_INTERVAL_MS = 1_000;
const HEARTBEAT_TIMER_MS = 10_000;

async function runAgentPoke(args: string[]): Promise<void> {
  try {
    await execFileAsync(agentPokeBin, args);
  } catch (error) {
    // An unavailable notifier must not interrupt the pi agent.
    console.error(`agent-poke failed: ${error instanceof Error ? error.message : error}`);
  }
}

export default function (pi: ExtensionAPI) {
  let lastHeartbeatMs = 0;
  let heartbeatTimer: ReturnType<typeof setInterval> | undefined;

  const state = (value: "idle" | "running" | "done" | "closed", event: string) =>
    runAgentPoke(["state", "--agent", "pi", "--state", value, "--event", event]);

  const heartbeat = (event: string) => {
    const now = Date.now();
    if (now - lastHeartbeatMs < HEARTBEAT_INTERVAL_MS) {
      return Promise.resolve();
    }
    lastHeartbeatMs = now;
    return runAgentPoke(["heartbeat", "--agent", "pi", "--event", event]);
  };

  const startHeartbeatTimer = () => {
    if (heartbeatTimer) {
      return;
    }
    heartbeatTimer = setInterval(() => {
      void heartbeat("agent_active");
    }, HEARTBEAT_TIMER_MS);
  };

  const stopHeartbeatTimer = () => {
    if (!heartbeatTimer) {
      return;
    }
    clearInterval(heartbeatTimer);
    heartbeatTimer = undefined;
  };

  pi.on("session_start", (event) => state("idle", `session_start:${event.reason}`));
  pi.on("before_agent_start", async () => {
    startHeartbeatTimer();
    await state("running", "before_agent_start");
  });
  pi.on("agent_start", async () => {
    startHeartbeatTimer();
    await state("running", "agent_start");
  });
  pi.on("tool_execution_start", (event) => heartbeat(`tool_start:${event.toolName}`));
  pi.on("tool_execution_update", (event) => heartbeat(`tool_update:${event.toolName}`));
  pi.on("tool_execution_end", (event) => heartbeat(`tool_end:${event.toolName}`));
  pi.on("agent_settled", async () => {
    stopHeartbeatTimer();
    await state("done", "agent_settled");
  });
  pi.on("session_shutdown", async (event) => {
    stopHeartbeatTimer();
    await state("closed", `session_shutdown:${event.reason}`);
  });
}
