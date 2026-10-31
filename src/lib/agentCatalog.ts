import catalog from "./agentCatalog.json";
import type { AgentStatus } from "./types";

export function demoAgents(scope: "global" | "project", root: string, platform = "macos"): AgentStatus[] {
  const agents: AgentStatus[] = [];
  for (const definition of catalog.filter((agent) => agent.scope === scope && (!agent.platform || agent.platform === platform))) {
    const targetPath = `${root}/${definition.path}`;
    const shared = agents.find((agent) => agent.targetPath === targetPath);
    if (shared) {
      shared.label += ` / ${definition.label}`;
      shared.note = definition.note;
      shared.docsUrl = definition.docsUrl;
      shared.maxChars = definition.maxChars ?? undefined;
      shared.maxBytes = definition.maxBytes ?? undefined;
      continue;
    }
    agents.push({
      id: definition.id, label: definition.label, targetPath, scope,
      mode: "symlink", state: "ready", targetKind: "missing",
      installed: ["gemini", "copilot", "devin-desktop", "cursor", "copilot-ide"].includes(definition.id),
      connected: false, detectionDetail: "Browser demo installation marker.",
      detail: "No native rules file exists yet.", note: definition.note,
      docsUrl: definition.docsUrl, maxChars: definition.maxChars ?? undefined,
      maxBytes: definition.maxBytes ?? undefined,
    });
  }
  return agents;
}

export function agentNote(agent: AgentStatus, chinese: boolean): string | undefined {
  if (!chinese) return agent.note;
  const id = agent.id === "gemini" && agent.label.includes("Antigravity") ? "antigravity" : agent.id;
  return catalog.find((definition) => definition.id === id)?.noteZh || agent.note;
}

export function agentWarning(agent: AgentStatus, chinese: boolean): string | undefined {
  if (!chinese || !agent.warning) return agent.warning;
  if (agent.maxBytes) return `规则超过 ${agent.maxBytes.toLocaleString()} 字节（UTF-8）限制，请精简 Profile。`;
  if (agent.maxChars) return `规则超过 ${agent.maxChars.toLocaleString()} 字符限制，请精简 Profile。`;
  return agent.warning;
}
