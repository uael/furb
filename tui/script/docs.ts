import { existsSync, readdirSync } from "node:fs";
import { readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { slashes } from "../src/commands.ts";
import { chords, keys } from "../src/keys.ts";
import { loadPart } from "../src/parts.ts";

/** The commands that the part of each official extension adds, which a life that runs the extension has. */
const official = fileURLToPath(new URL("../../extensions/", import.meta.url));
const parted: string[] = [];
for (const name of readdirSync(official).sort()) {
  const file = join(official, name, "tui.ts");
  if (!existsSync(file)) continue;
  const part = await loadPart(name, file);
  if (part.error) throw new Error(part.error);
  for (const one of part.commands)
    parted.push(`| \`/${one.name}\` | ${one.detail} (the extension ${name}) |`);
}

const path = new URL("../README.md", import.meta.url);
const cell = (text: string) => text.replaceAll("\\", "\\\\").replaceAll("|", "\\|");
const tables = {
  keys: [
    "| Key | Action |",
    "| --- | --- |",
    ...keys.map((key) => `| ${cell(chords(key, true))} | ${cell(key.action)} |`),
  ],
  commands: [
    "| Command | Action |",
    "| --- | --- |",
    ...slashes.map(({ usage, detail }) => `| \`${usage}\` | ${detail} |`),
    ...parted,
  ],
};
let text = await readFile(path, "utf8");
for (const [name, rows] of Object.entries(tables))
  text = text.replace(
    new RegExp(`<!-- ${name}:start -->[\\s\\S]*?<!-- ${name}:end -->`),
    () => `<!-- ${name}:start -->\n\n${rows.join("\n")}\n\n<!-- ${name}:end -->`,
  );
await writeFile(path, text);
