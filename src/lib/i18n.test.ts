import { describe, expect, it } from "vitest";
import {
  parseLanguagePreference,
  resolveLocale,
  resolveSystemLocale,
  translate,
} from "./i18n";

describe("i18n", () => {
  it("resolves Chinese language variants to Simplified Chinese UI", () => {
    expect(resolveSystemLocale(["zh-Hans-CN", "en-US"])).toBe("zh-CN");
    expect(resolveLocale("system", ["zh-TW"])).toBe("zh-CN");
    expect(resolveSystemLocale(["en-US", "zh-CN"])).toBe("en");
  });

  it("uses English as the system fallback and respects explicit overrides", () => {
    expect(resolveSystemLocale(["ja-JP"])).toBe("en");
    expect(resolveLocale("zh-CN", ["en-US"])).toBe("zh-CN");
  });

  it("rejects invalid persisted preferences", () => {
    expect(parseLanguagePreference("fr")).toBe("system");
    expect(parseLanguagePreference("en")).toBe("en");
  });

  it("interpolates translated interface messages", () => {
    expect(translate("zh-CN", "projection.connectionChangesReady", { count: 3 })).toBe(
      "确认 3 项接入修改",
    );
    expect(translate("en", "notice.restored", { id: "snapshot-1" })).toBe(
      "Restored snapshot snapshot-1.",
    );
    expect(translate("zh-CN", "rollback.tooltip")).toContain("不会修改规则源 AGENTS.md");
  });

  it("uses direct actions followed by confirmation language", () => {
    const primaryActions = [
      "profiles.rollback",
      "profiles.addFileSubmit",
      "profiles.switch",
      "profiles.refresh",
      "profiles.create.submit",
      "setup.startCreate",
    ] as const;
    for (const key of primaryActions) {
      expect(translate("zh-CN", key)).not.toContain("预览");
    }
    expect(translate("zh-CN", "profiles.removeFileConfirm")).toBe("确认删除");
  });
});
