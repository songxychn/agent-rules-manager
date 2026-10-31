import { describe, expect, it } from "vitest";
import { splitSyncDiff } from "./syncDiff";

describe("sync diff text fidelity", () => {
  it("highlights changes while preserving Unicode, CRLF, blank lines and final newlines", () => {
    for (const [before, after] of [["相同\r\n旧内容\r\n\r\n尾部", "相同\r\n新内容\r\n\r\n尾部"], ["a\n", "a"], ["a", "a\n"], [null, "\n"], ["", null], ["same\n", "same\n"]]) {
      const d = splitSyncDiff(before, after);
      expect(d.prefix + d.removed + d.suffix).toBe(before ?? "");
      expect(d.prefix + d.added + d.suffix).toBe(after ?? "");
    }
  });
});
