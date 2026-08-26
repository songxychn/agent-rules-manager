import { describe, expect, it } from "vitest";
import { groupOpenTargets, resolveOpenTarget } from "./openTargets";
import type { OpenTarget } from "./types";

const targets: OpenTarget[] = [
  { id: "default", label: "Default app", kind: "default" },
  { id: "vscode", label: "Visual Studio Code", kind: "editor" },
  { id: "finder", label: "Finder", kind: "reveal" },
  { id: "terminal", label: "Terminal", kind: "terminal" },
];

describe("open target selection", () => {
  it("keeps an available preference and falls back to the system default", () => {
    expect(resolveOpenTarget(targets, "vscode")?.id).toBe("vscode");
    expect(resolveOpenTarget(targets, "missing")?.id).toBe("default");
  });

  it("separates editors from system location actions", () => {
    const groups = groupOpenTargets(targets);

    expect(groups.editors.map((target) => target.id)).toEqual(["default", "vscode"]);
    expect(groups.system.map((target) => target.id)).toEqual(["finder", "terminal"]);
  });
});
