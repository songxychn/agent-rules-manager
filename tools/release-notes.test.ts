import { describe, expect, it } from "vitest";
import { releaseNotes } from "./release-notes";

const changelog = `# 更新日志

## [未发布]

- 下一版本内容

## [0.1.1] - 2026-09-16

### 修复

- 修复 macOS 更新。

## [0.1.0] - 2026-09-15

- 首次发布。

[0.1.1]: https://example.com/v0.1.1
[0.1.0]: https://example.com/v0.1.0
`;

describe("release notes from CHANGELOG", () => {
  it("selects only the exact version, preserving Markdown and Chinese", () => {
    expect(releaseNotes(changelog, "0.1.1")).toBe("### 修复\n\n- 修复 macOS 更新。");
  });
  it("handles Windows line endings", () => {
    expect(releaseNotes(changelog.replace(/\n/g, "\r\n"), "0.1.1"))
      .toBe(releaseNotes(changelog, "0.1.1"));
  });
  it("excludes reference definitions from the last version", () => {
    expect(releaseNotes(changelog, "0.1.0")).toBe("- 首次发布。");
  });
  it("does not treat a heading in a fenced example as another version", () => {
    const content = "## [0.1.1]\n\n```md\n## [0.1.1]\n```\n\n- 修复。\n\n## [0.1.0]\n- 旧版。";
    expect(releaseNotes(content, "0.1.1")).toBe("```md\n## [0.1.1]\n```\n\n- 修复。");
  });
  it.each(["0.1", "0.1.10", "0.1.1-beta.1"])("rejects a missing version %s", (version) => {
    expect(() => releaseNotes(changelog, version)).toThrow("found 0");
  });
  it("rejects duplicate version sections", () => {
    expect(() => releaseNotes(changelog + "\n## [0.1.1]\n- duplicate", "0.1.1"))
      .toThrow("found 2");
  });
  it("rejects an empty version section", () => {
    expect(() => releaseNotes("## [0.1.1]\n\n## [0.1.0]\n- old", "0.1.1"))
      .toThrow("is empty");
  });
});
