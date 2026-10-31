import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

// Generate into a new directory for review; never overwrite the installed icons.
const outputArgument = process.argv[2];
if (!outputArgument) {
  throw new Error("Usage: bun tools/generate-macos-icons.ts <new-output-directory>");
}

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const output = resolve(outputArgument);
const brand = await readFile(join(root, "assets/icon-source.svg"), "utf8");
// Keep the UI brand mark unchanged. On a 1024px canvas the 824px artwork
// occupies about 80.5%, leaving 100px transparent margins around the background.
const padded = `<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024" viewBox="0 0 1024 1024">
${brand.replace(/<svg\b[^>]*>/, '<svg x="100" y="100" width="824" height="824" viewBox="0 0 40 40">')}
</svg>\n`;

await mkdir(output);
const source = join(output, "icon-macos.svg");
await writeFile(source, padded, { flag: "wx" });
const child = Bun.spawn(
  [process.execPath, "run", "tauri", "icon", source, "--output", join(output, "icons")],
  { cwd: root, stdin: "inherit", stdout: "inherit", stderr: "inherit" },
);
process.exit(await child.exited);
