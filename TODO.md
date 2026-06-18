# TODO

- Fix disabled extension tool results being persisted as successful: `AgentToolResult.isError` returned from tools like `delegate_subagents` should propagate through `packages/agent/src/agent-loop.ts`.
- Make model/provider turn failures visible in the transcript/editor flow, not only as transient notifications, so repeated relay 502/upstream failures cannot look like the model silently stopped.
