import type { ProjectionPlan, WorkspaceSnapshot } from "./types";

export function buildProjectionPlan(
  snapshot: WorkspaceSnapshot,
  selectedAgents: string[],
): ProjectionPlan {
  const selected = new Set(selectedAgents);
  const steps = snapshot.agents
    .filter((agent) => selected.has(agent.id))
    .map((agent) => ({
      agentId: agent.id,
      agentLabel: agent.label,
      targetPath: agent.targetPath,
      state: agent.state,
      action:
        agent.state === "inSync"
          ? "none"
          : agent.state === "conflict"
            ? "blocked"
            : agent.mode === "include"
              ? "addInclude"
              : "createLink",
      summary:
        agent.state === "inSync"
          ? "Already projects the canonical rules."
          : agent.state === "conflict"
            ? "An unmanaged file is present."
            : "Create a safe projection without duplicating rule content.",
    }));

  return {
    sourcePath: snapshot.sourcePath,
    blocked: steps.some((step) => step.state === "conflict"),
    changeCount: steps.filter((step) => step.state === "ready" || step.state === "drifted").length,
    steps,
  };
}
