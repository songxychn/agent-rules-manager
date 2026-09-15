import { copyFile, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
const source = await readFile(join(root, "assets/icon-source.svg"), "utf8");
const viewBox = /viewBox="([^"]+)"/;
const bounds = source.match(viewBox)?.[1].trim().split(/[\s,]+/).map(Number);
if (!bounds || bounds.length !== 4 || !bounds.every(Number.isFinite)) {
  throw new Error("Icon source must have a numeric SVG viewBox");
}
const [x, y, width, height] = bounds;
if (width <= 0 || width !== height) {
  throw new Error("Icon source must have a positive, square SVG viewBox");
}

// Keep the artwork at 80% of the macOS canvas, with 10% transparent edges.
// Expanding the viewBox preserves the canonical artwork and other platforms.
const inset = width * 0.125;
const padded = source.replace(
  viewBox,
  `viewBox="${x - inset} ${y - inset} ${width + inset * 2} ${height + inset * 2}"`,
);
const temporary = await mkdtemp(join(tmpdir(), "arm-macos-icon-"));
try {
  const input = join(temporary, "icon-macos.svg");
  const output = join(temporary, "icons");
  await writeFile(input, padded);
  const child = Bun.spawn(
    [process.execPath, join(root, "tools/tauri.ts"), "icon", input, "--output", output],
    { cwd: root, stdout: "inherit", stderr: "inherit" },
  );
  if ((await child.exited) !== 0) {
    throw new Error("macOS icon generation failed");
  }
  await copyFile(join(output, "icon.icns"), join(root, "src-tauri/icons/icon.icns"));
} finally {
  await rm(temporary, { recursive: true, force: true });
}
