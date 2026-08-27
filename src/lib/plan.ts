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
      let requiresConfirmation = false;

      if (desiredConnected && agent.targetKind === "legacyLink") {
        action = "replaceLegacyLink";
        summary = "Move the managed legacy link to current/AGENTS.md.";
      } else if (desiredConnected && agent.targetKind === "legacyInclude") {
        action = "migrateLegacyInclude";
        summary = "Replace the legacy managed include with a symbolic link.";
      } else if (desiredConnected !== agent.connected) {
        if (desiredConnected && agent.targetKind === "missing") {
          action = "createLink";
          summary = "Create a symbolic link to the canonical rules file.";
        } else if (
          desiredConnected &&
          (agent.targetKind === "independentFile" || agent.targetKind === "invalidManagedFile")
        ) {
          action = "backupAndCreateLink";
          summary = "Back up the existing regular file, then replace it with a symbolic link.";
          requiresConfirmation = true;
        } else if (desiredConnected) {
          action = "blocked";
          summary = "The unexpected symbolic link will not be replaced.";
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
        requiresConfirmation,
      };
    });

  return {
    sourcePath: snapshot.sourcePath,
    blocked: steps.some((step) => step.action === "blocked"),
    changeCount: steps.filter((step) => step.action !== "none" && step.action !== "blocked").length,
    confirmationCount: steps.filter((step) => step.requiresConfirmation).length,
    steps,
  };
}
