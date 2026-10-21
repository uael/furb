import { readFile, writeFile } from "node:fs/promises";
import { slashes } from "../src/commands.ts";
import { keyTable } from "../src/keys.ts";

const path = new URL("../README.md", import.meta.url);
const cell = (text: string) => text.replaceAll("\\", "\\\\").replaceAll("|", "\\|");
const tables = {
  keys: [
    "| Key | Action |",
    "| --- | --- |",
    ...keyTable.map(([key, action]) => `| ${cell(key)} | ${cell(action)} |`),
  ],
  commands: [
    "| Command | Action |",
    "| --- | --- |",
    ...slashes.map(({ usage, detail }) => `| \`${usage}\` | ${detail} |`),
  ],
};
let text = await readFile(path, "utf8");
for (const [name, rows] of Object.entries(tables))
  text = text.replace(
    new RegExp(`<!-- ${name}:start -->[\\s\\S]*?<!-- ${name}:end -->`),
    () => `<!-- ${name}:start -->\n\n${rows.join("\n")}\n\n<!-- ${name}:end -->`,
  );
await writeFile(path, text);
