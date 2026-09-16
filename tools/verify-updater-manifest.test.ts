import { describe, expect, it } from "vitest";
import { verifyUpdaterManifest } from "./verify-updater-manifest";

function manifest() {
  return {
    version: "0.1.0",
    platforms: Object.fromEntries(
      ["darwin-aarch64", "darwin-x86_64", "linux-x86_64", "windows-x86_64"].map(
        (target) => [target, { url: `https://example.com/${target}`, signature: "fixture-signature" }],
      ),
    ),
  };
}

describe("release updater manifest", () => {
  it("rejects the published regression with Windows/Linux but no macOS entries", () => {
    const release = manifest();
    delete release.platforms["darwin-aarch64"];
    delete release.platforms["darwin-x86_64"];
    expect(() => verifyUpdaterManifest(release, "0.1.0")).toThrow(
      "Missing or invalid updater platforms: darwin-aarch64, darwin-x86_64",
    );
  });

  it("accepts a complete manifest, including additional installer-specific entries", () => {
    const release = manifest();
    release.platforms["darwin-aarch64-app"] = release.platforms["darwin-aarch64"];
    expect(() => verifyUpdaterManifest(release, "0.1.0")).not.toThrow();
  });

  it("rejects a stale version", () => {
    expect(() => verifyUpdaterManifest(manifest(), "0.2.0")).toThrow("Expected updater version");
  });

  it.each(["", "http://example.com/app.tar.gz", "not a URL"])("rejects invalid download URL %j", (url) => {
    const release = manifest();
    release.platforms["darwin-aarch64"].url = url;
    expect(() => verifyUpdaterManifest(release, "0.1.0")).toThrow("darwin-aarch64");
  });

  it("rejects an empty signature", () => {
    const release = manifest();
    release.platforms["darwin-x86_64"].signature = "  ";
    expect(() => verifyUpdaterManifest(release, "0.1.0")).toThrow("darwin-x86_64");
  });

  it.each([null, {}, { version: "0.1.0", platforms: null }])("rejects incomplete metadata %j", (release) => {
    expect(() => verifyUpdaterManifest(release, "0.1.0")).toThrow();
  });
});
