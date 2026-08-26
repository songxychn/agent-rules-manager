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
      "已检查 3 项接入调整",
    );
    expect(translate("en", "notice.restored", { id: "snapshot-1" })).toBe(
      "Restored snapshot snapshot-1.",
    );
    expect(translate("zh-CN", "rollback.tooltip")).toContain("不会修改规则源 AGENTS.md");
  });
});
