import type { ConnectionChange, ProjectionPlan, WorkspaceSnapshot } from "./types";

export function buildProjectionPlan(
  snapshot: WorkspaceSnapshot,
  changes: ConnectionChange[],
): ProjectionPlan {
  const requested = new Map(changes.map((change) => [change.agentId, change.connected]));
  const steps = snapshot.agents
    .filter((agent) => requested.has(agent.id))
    .map((agent) => {
      const desiredConnected = requested.get(agent.id) ?? agent.connected;
      let action = "none";
      let summary = "Already uses the selected connection mode.";

      if (desiredConnected !== agent.connected) {
        if (desiredConnected && agent.targetKind === "missing") {
          action = "createLink";
          summary = "Create a symbolic link to the canonical rules file.";
        } else if (desiredConnected) {
          action = "blocked";
          summary = "The existing independent file or unexpected link will not be overwritten.";
        } else if (agent.targetKind === "connectedLink") {
          action = "createIndependentFile";
          summary = "Replace the managed link with an independent copy.";
        } else if (agent.targetKind === "legacyInclude") {
          action = "detachManagedInclude";
          summary = "Replace the managed include with an independent copy.";
        }
      }

      return {
      agentId: agent.id,
      agentLabel: agent.label,
      targetPath: agent.targetPath,
      state: agent.state,
        desiredConnected,
        action,
        summary,
      };
    });

  return {
    sourcePath: snapshot.sourcePath,
    blocked: steps.some((step) => step.action === "blocked"),
    changeCount: steps.filter((step) => step.action !== "none" && step.action !== "blocked").length,
    steps,
  };
}
