export const AgentPokePlugin = async ({ $ }) => {
  let lastHeartbeatMs = 0
  let lastSessionStatusType = null
  let lastSessionStatusMs = 0

  return {
    event: async ({ event }) => {
      const eventTsMs = Date.now()
      const statusType = event.properties?.status?.type

      if (event.type === "session.status") {
        if (
          statusType &&
          statusType === lastSessionStatusType &&
          eventTsMs - lastSessionStatusMs < 750
        ) {
          return
        }

        if (statusType) {
          await $`agent-poke opencode-event --event session.status --status ${statusType} --event-ts-ms ${eventTsMs}`
        } else {
          await $`agent-poke opencode-event --event session.status --event-ts-ms ${eventTsMs}`
        }
        lastSessionStatusType = statusType ?? null
        lastSessionStatusMs = eventTsMs
        return
      }

      if (eventTsMs - lastHeartbeatMs < 1000) {
        return
      }

      await $`agent-poke opencode-event --event ${event.type} --event-ts-ms ${eventTsMs}`
      lastHeartbeatMs = eventTsMs
    },
  }
}
