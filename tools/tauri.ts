import { delimiter, join } from "node:path";

const cargoBin = join(process.env.HOME ?? process.env.USERPROFILE ?? "", ".cargo", "bin");
const pathKey = Object.keys(process.env).find((key) => key.toUpperCase() === "PATH") ?? "PATH";
const inheritedPath = process.env[pathKey] ?? "";
const env = {
  ...process.env,
  [pathKey]: [cargoBin, inheritedPath].filter(Boolean).join(delimiter),
};

const child = Bun.spawn(
  [process.execPath, "x", "--bun", "tauri", ...process.argv.slice(2)],
  {
    env,
    stdin: "inherit",
    stdout: "inherit",
    stderr: "inherit",
  },
);

process.exit(await child.exited);
