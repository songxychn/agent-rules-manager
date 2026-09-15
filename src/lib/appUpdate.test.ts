import { describe, expect, it } from "vitest";
import { checkForAppUpdate, desktopUpdatesAvailable, downloadPercent } from "./appUpdate";

describe("app updates", () => {
  it("treats the browser demo as a non-updating surface", async () => {
    expect(desktopUpdatesAvailable()).toBe(false);
    await expect(checkForAppUpdate()).resolves.toEqual({ status: "unavailable" });
  });

  it("computes download progress only when a total size is known", () => {
    expect(downloadPercent(50)).toBeUndefined();
    expect(downloadPercent(50, 0)).toBeUndefined();
    expect(downloadPercent(25, 100)).toBe(25);
    expect(downloadPercent(150, 100)).toBe(100);
  });
});
