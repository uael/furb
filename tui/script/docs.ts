import { readFile, writeFile } from "node:fs/promises";
import { commands } from "../src/commands.ts";

const path = new URL("../README.md", import.meta.url);
const text = await readFile(path, "utf8");
const table = [
  "| Command | Action |",
  "| --- | --- |",
  ...Object.entries(commands).map(
    ([name, [, args, detail]]) => `| \`/${name}${args ? ` ${args}` : ""}\` | ${detail} |`,
  ),
].join("\n");
await writeFile(
  path,
  text.replace(
    /<!-- commands:start -->[\s\S]*?<!-- commands:end -->/,
    `<!-- commands:start -->\n\n${table}\n\n<!-- commands:end -->`,
  ),
);
