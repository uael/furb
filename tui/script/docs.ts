import { readFile, writeFile } from "node:fs/promises";
import { builtinTuiParts } from "../src/builtin/index.ts";
import { commands } from "../src/commands.ts";
import { chords, keys } from "../src/keys.ts";

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
    ...Object.entries(commands).map(
      ([name, [, args, detail]]) => `| \`/${name}${args ? ` ${args}` : ""}\` | ${detail} |`,
    ),
  ],
  // The commands of the builtin extensions, each under the name of its extension, which a config may turn off.
  extensions: [
    "| Command | Extension | Action |",
    "| --- | --- | --- |",
    ...(
      await Promise.all(
        Object.entries(builtinTuiParts).map(async ([extension, made]) =>
          Object.entries((await made()).commands ?? {}).map(
            ([name, command]) =>
              `| \`/${name}${command.argument ? ` ${command.argument}` : ""}\` | ${extension} | ${command.detail} |`,
          ),
        ),
      )
    ).flat(),
  ],
};
let text = await readFile(path, "utf8");
for (const [name, rows] of Object.entries(tables))
  text = text.replace(
    new RegExp(`<!-- ${name}:start -->[\\s\\S]*?<!-- ${name}:end -->`),
    () => `<!-- ${name}:start -->\n\n${rows.join("\n")}\n\n<!-- ${name}:end -->`,
  );
await writeFile(path, text);
