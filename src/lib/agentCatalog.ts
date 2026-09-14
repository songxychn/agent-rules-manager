import catalog from "./agentCatalog.json";
import type { AgentStatus } from "./types";

export function demoAgents(scope: "global" | "project", root: string): AgentStatus[] {
  const agents: AgentStatus[] = [];
  for (const definition of catalog.filter((agent) => agent.scope === scope)) {
    const targetPath = `${root}/${definition.path}`;
    const shared = agents.find((agent) => agent.targetPath === targetPath);
    if (shared) {
      shared.label += ` / ${definition.label}`;
      shared.note = definition.note;
      shared.maxChars = definition.maxChars ?? undefined;
      continue;
    }
    agents.push({
      id: definition.id, label: definition.label, targetPath, scope,
      mode: "symlink", state: "ready", targetKind: "missing",
      installed: ["gemini", "copilot", "cursor", "copilot-ide"].includes(definition.id),
      connected: false, detectionDetail: "Browser demo installation marker.",
      detail: "No native rules file exists yet.", note: definition.note,
      docsUrl: definition.docsUrl, maxChars: definition.maxChars ?? undefined,
    });
  }
  return agents;
}

export function agentNote(agent: AgentStatus, chinese: boolean): string | undefined {
  if (!chinese) return agent.note;
  const id = agent.id === "gemini" && agent.label.includes("Antigravity") ? "antigravity" : agent.id;
  return catalog.find((definition) => definition.id === id)?.noteZh || agent.note;
}
