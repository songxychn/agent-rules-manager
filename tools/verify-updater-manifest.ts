import { readFile } from "node:fs/promises";

const requiredPlatforms = [
  "darwin-aarch64",
  "darwin-x86_64",
  "linux-x86_64",
  "windows-x86_64",
];

export function verifyUpdaterManifest(value: unknown, version: string, expectedNotes?: string): void {
  if (!value || typeof value !== "object") throw new Error("Invalid updater manifest");
  const manifest = value as Record<string, unknown>;
  if (manifest.version !== version) {
    throw new Error(`Expected updater version ${version}, got ${manifest.version}`);
  }
  if (expectedNotes !== undefined &&
      (typeof manifest.notes !== "string" || manifest.notes.trim() !== expectedNotes.trim())) {
    throw new Error("Updater notes do not match the CHANGELOG release section");
  }
  const platforms = manifest.platforms as
    | Record<string, { url?: unknown; signature?: unknown }>
    | undefined;
  const invalid = requiredPlatforms.filter((target) => {
    const entry = platforms?.[target];
    if (!entry || typeof entry.signature !== "string" || !entry.signature.trim()) return true;
    if (typeof entry.url !== "string") return true;
    try {
      return new URL(entry.url).protocol !== "https:";
    } catch {
      return true;
    }
  });
  if (invalid.length) {
    throw new Error(`Missing or invalid updater platforms: ${invalid.join(", ")}`);
  }
}

if (import.meta.main) {
  const [path, version, notesPath] = process.argv.slice(2);
  if (!path || !version) {
    throw new Error("Usage: bun tools/verify-updater-manifest.ts <latest.json> <version> [notes.md]");
  }
  verifyUpdaterManifest(
    JSON.parse(await readFile(path, "utf8")), version,
    notesPath ? await readFile(notesPath, "utf8") : undefined,
  );
  console.log(`Updater manifest ${version} includes all supported platforms.`);
}
