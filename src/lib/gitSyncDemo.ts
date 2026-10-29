import type { GitSyncConfig, GitSyncPlan, GitSyncRequest, GitSyncVersion, WorkspaceSnapshot } from "./types";

type Files = Record<string, string>;
export interface DemoSyncState { config?: GitSyncConfig; baseline?: Files }
const bodies = new Map<string, string>();
let remote: Files | undefined;
let initialBase: Files | undefined;

export function demoSyncSources(snapshot: WorkspaceSnapshot): Files {
  if (snapshot.libraryState === "empty") return {};
  const files: Files = { "schema.json": JSON.stringify({ schemaVersion: 3, format: "agent-rules-library", sync: { include: ["schema.json", "profiles/**"], exclude: [".runtime/**", "current", "machine state"], machineSelection: "local" } }, null, 2) + "\n" };
  for (const p of snapshot.profiles) {
    const digest = p.files[0]?.digest ?? p.id;
    if (!bodies.has(digest)) bodies.set(digest, `# ${p.name}\n\n${p.description}\n\n- Review changes before applying them.\n`);
    files[`profiles/${p.id}/profile.json`] = JSON.stringify({ schemaVersion: 3, id: p.id, name: p.name, description: p.description }, null, 2) + "\n";
    files[`profiles/${p.id}/AGENTS.md`] = bodies.get(digest)!;
  }
  return files;
}
function versions(files: Files, prefix: string): GitSyncVersion[] {
  return Object.entries(files).filter(([p]) => p.startsWith(prefix)).map(([path, content]) => ({ path, content }));
}
function equal(a: GitSyncVersion[], b: GitSyncVersion[]) { return JSON.stringify(a) === JSON.stringify(b); }

export function previewDemoSync(snapshot: WorkspaceSnapshot, state: DemoSyncState, request: GitSyncRequest): { plan: GitSyncPlan; merged: Files } {
  if (!request.remote.trim() || !request.branch.trim()) throw new Error("Enter a remote and branch");
  const local = demoSyncSources(snapshot);
  if (!remote) {
    remote = structuredClone(local);
    initialBase = structuredClone(local);
    if (remote["profiles/work/AGENTS.md"]) {
      initialBase!["profiles/work/AGENTS.md"] = "# Work rules\n\n- Review changes.\n";
      remote["profiles/work/AGENTS.md"] += "- Run checks before sharing changes.\n";
    }
    if (remote["profiles/personal/AGENTS.md"]) remote["profiles/personal/AGENTS.md"] += "- Keep experiments in a separate branch.\n";
  }
  const base = state.baseline ?? initialBase ?? {};
  const merged: Files = {};
  const conflicts: GitSyncPlan["conflicts"] = [];
  const prefixes = new Set([...Object.keys(local), ...Object.keys(remote), ...Object.keys(base)].map((p) => p === "schema.json" ? p : p.split("/").slice(0, 2).join("/") + "/"));
  const decisions = new Map(request.resolutions.map((d) => [d.id, d]));
  for (const prefix of prefixes) {
    const b = versions(base, prefix), l = versions(local, prefix), r = versions(remote, prefix);
    let chosen = equal(l, r) || equal(b, r) ? l : equal(b, l) ? r : undefined;
    if (!chosen) {
      const id = prefix === "schema.json" ? "schema" : `profile:${prefix.split("/")[1]}`;
      const decision = decisions.get(id);
      const canKeepBoth = prefix !== "schema.json" && Boolean(l.length && r.length);
      conflicts.push({ id, base: b, local: l, remote: r, canKeepBoth, resolved: Boolean(decision) });
      chosen = decision?.choice === "useRemote" ? r : l;
      if (decision?.choice === "keepBoth") {
        const newId = decision.newProfileId ?? "";
        if (!canKeepBoth || !/^[a-z0-9][a-z0-9_-]{0,63}$/.test(newId) || Object.keys(local).some((p) => p.startsWith(`profiles/${newId}/`)) || Object.keys(remote).some((p) => p.startsWith(`profiles/${newId}/`)) || Object.keys(merged).some((p) => p.startsWith(`profiles/${newId}/`))) throw new Error("Choose an unused Profile id for the remote copy");
        for (const file of r) {
          merged[file.path.replace(prefix, `profiles/${newId}/`)] = file.path.endsWith("profile.json") ? JSON.stringify({ ...JSON.parse(file.content), id: newId }, null, 2) + "\n" : file.content;
        }
      }
    }
    for (const file of chosen) merged[file.path] = file.content;
  }
  const blockers: string[] = [];
  if (snapshot.activeProfileId && !merged[`profiles/${snapshot.activeProfileId}/profile.json`]) blockers.push("Activate another Profile before deleting the active Profile through sync.");
  const changes = [...new Set([...Object.keys(local), ...Object.keys(remote), ...Object.keys(merged)])].sort().filter((p) => local[p] !== merged[p] || remote![p] !== merged[p]).map((path) => ({ path, localBefore: local[path] ?? null, localAfter: merged[path] ?? null, remoteBefore: remote![path] ?? null, remoteAfter: merged[path] ?? null }));
  const plan: GitSyncPlan = { token: JSON.stringify([snapshot, state, request, remote]), config: { remote: request.remote, branch: request.branch }, remoteCommit: "demo-remote", changes, directories: [], conflicts, blockers, blocked: Boolean(blockers.length || conflicts.some((c) => !c.resolved)), connectionChanged: JSON.stringify(state.config) !== JSON.stringify({ remote: request.remote, branch: request.branch }) };
  return { plan, merged };
}

export function applyDemoSync(snapshot: WorkspaceSnapshot, state: DemoSyncState, request: GitSyncRequest, token: string): GitSyncPlan {
  const { plan, merged } = previewDemoSync(snapshot, state, request);
  if (plan.blocked || plan.token !== token) throw new Error("Sync preview is stale or unresolved; preview again");
  const activeChanged = plan.changes.some((c) => c.path === `profiles/${snapshot.activeProfileId}/AGENTS.md` && c.localBefore !== c.localAfter);
  snapshot.profiles = Object.entries(merged).filter(([p]) => p.endsWith("/profile.json")).map(([path, text]) => {
    const p = JSON.parse(text) as { id: string; name: string; description: string };
    const content = merged[path.replace("profile.json", "AGENTS.md")];
    let hash = 2166136261;
    for (let i = 0; i < content.length; i++) hash = Math.imul(hash ^ content.charCodeAt(i), 16777619);
    const digest = `demo${(hash >>> 0).toString(16).padStart(8, "0")}`;
    bodies.set(digest, content);
    return { ...p, isActive: snapshot.activeProfileId === p.id, files: [{ path: "AGENTS.md", digest }] };
  });
  snapshot.libraryState = "ready";
  if (activeChanged) snapshot.runtimeState = "stale";
  state.config = plan.config;
  state.baseline = structuredClone(merged);
  remote = structuredClone(merged);
  return plan;
}
