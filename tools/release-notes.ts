import { randomUUID } from "node:crypto";
import { appendFile, readFile } from "node:fs/promises";

export function releaseNotes(changelog: string, version: string): string {
  const lines = changelog.replace(/\r\n/g, "\n").split("\n");
  const sections: { version?: string; start: number }[] = [];
  let fence: string | undefined;
  for (const [index, line] of lines.entries()) {
    const marker = line.match(/^ {0,3}(`{3,}|~{3,})/);
    if (fence) {
      if (marker && marker[1][0] === fence[0] && marker[1].length >= fence.length &&
          !line.slice(marker[0].length).trim()) fence = undefined;
      continue;
    }
    if (marker) {
      fence = marker[1];
      continue;
    }
    if (/^##\s+/.test(line)) {
      sections.push({ version: line.match(/^##\s+\[([^\]]+)\](?:\s|$)/)?.[1], start: index });
    }
  }
  const matches = sections.filter((section) => section.version === version);
  if (matches.length !== 1) {
    throw new Error(`Expected exactly one CHANGELOG section for ${version}, found ${matches.length}`);
  }
  const section = matches[0];
  const next = sections[sections.indexOf(section) + 1];
  const notes = lines.slice(section.start + 1, next?.start)
    // Keep a final version's reference definitions out of the displayed notes.
    .filter((line) => !/^\[[^\]]+\]:\s/.test(line)).join("\n").trim();
  if (!notes) throw new Error(`CHANGELOG section for ${version} is empty`);
  return notes;
}

if (import.meta.main) {
  const [path, version, githubOutput] = process.argv.slice(2);
  if (!path || !version) {
    throw new Error("Usage: bun tools/release-notes.ts <CHANGELOG.md> <version> [GITHUB_OUTPUT]");
  }
  const notes = releaseNotes(await readFile(path, "utf8"), version);
  if (githubOutput) {
    const delimiter = `RELEASE_NOTES_${randomUUID()}`;
    await appendFile(githubOutput, `notes<<${delimiter}\n${notes}\n${delimiter}\n`);
  } else {
    process.stdout.write(`${notes}\n`);
  }
}
