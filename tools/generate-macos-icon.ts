import { copyFile, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
const temporary = await mkdtemp(join(tmpdir(), "arm-macos-icon-"));
try {
  // Share the reviewed 824px artwork / 1024px canvas with the preview generator.
  const output = join(temporary, "generated");
  const child = Bun.spawn(
    [process.execPath, join(root, "tools/generate-macos-icons.ts"), output],
    { cwd: root, stdout: "inherit", stderr: "inherit" },
  );
  if ((await child.exited) !== 0) {
    throw new Error("macOS icon generation failed");
  }
  await copyFile(join(output, "icons/icon.icns"), join(root, "src-tauri/icons/icon.icns"));
} finally {
  await rm(temporary, { recursive: true, force: true });
}
