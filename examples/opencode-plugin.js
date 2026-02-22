export const AgentPokePlugin = async ({ $ }) => {
  return {
    event: async ({ event }) => {
      const eventTsMs = Date.now()

      if (event.type === "session.status") {
        const statusType = event.properties?.status?.type
        if (statusType) {
          await $`agent-poke opencode-event --event session.status --status ${statusType} --event-ts-ms ${eventTsMs}`
        }
        return
      }

      if (event.type === "session.idle" || event.type === "session.error") {
        await $`agent-poke opencode-event --event ${event.type} --event-ts-ms ${eventTsMs}`
        return
      }

      if (event.type === "permission.asked") {
        await $`agent-poke opencode-event --event permission.asked --event-ts-ms ${eventTsMs}`
      }
    },
  }
}
