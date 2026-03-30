export const AgentPokePlugin = async ({ $ }) => {
  let lastHeartbeatMs = 0
  const lastSessionStatusBySession = new Map()
  const subagentSessionIds = new Set()
  const transitionEvents = new Set(["permission.asked", "session.error", "session.idle"])

  const recordSession = (event) => {
    const sessionInfo = event.properties?.info
    if (!sessionInfo?.id) {
      return
    }

    if (event.type === "session.created" || event.type === "session.updated") {
      if (sessionInfo.parentID) {
        subagentSessionIds.add(sessionInfo.id)
      } else {
        subagentSessionIds.delete(sessionInfo.id)
      }
      return
    }

    if (event.type === "session.deleted") {
      subagentSessionIds.delete(sessionInfo.id)
      lastSessionStatusBySession.delete(sessionInfo.id)
    }
  }

  const eventSessionId = (event) => {
    if (typeof event.properties?.sessionID === "string") {
      return event.properties.sessionID
    }

    if (typeof event.properties?.info?.id === "string") {
      return event.properties.info.id
    }

    return null
  }

  const heartbeat = async (eventType, eventTsMs) => {
    if (eventTsMs - lastHeartbeatMs < 1000) {
      return
    }

    await $`agent-poke heartbeat --agent opencode --event ${eventType}`
    lastHeartbeatMs = eventTsMs
  }

  return {
    event: async ({ event }) => {
      const eventTsMs = Date.now()
      recordSession(event)

      const sessionID = eventSessionId(event)
      if (sessionID && subagentSessionIds.has(sessionID)) {
        await heartbeat(event.type, eventTsMs)
        return
      }

      const statusType = event.properties?.status?.type

      if (event.type === "session.status") {
        const statusKey = sessionID ?? "__unknown__"
        const lastStatus = lastSessionStatusBySession.get(statusKey)

        if (
          statusType &&
          statusType === lastStatus?.type &&
          eventTsMs - lastStatus.tsMs < 750
        ) {
          return
        }

        if (statusType) {
          await $`agent-poke opencode-event --event session.status --status ${statusType} --event-ts-ms ${eventTsMs}`
        } else {
          await $`agent-poke opencode-event --event session.status --event-ts-ms ${eventTsMs}`
        }
        lastSessionStatusBySession.set(statusKey, {
          type: statusType ?? null,
          tsMs: eventTsMs,
        })
        return
      }

      if (transitionEvents.has(event.type)) {
        await $`agent-poke opencode-event --event ${event.type} --event-ts-ms ${eventTsMs}`
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
