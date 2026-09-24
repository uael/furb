import { readdir, readFile, writeFile } from "node:fs/promises";
import type { TuiPart } from "@furb/engine";
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
/** The tables of a README written again between their marks. */
async function written(path: URL, given: Record<string, string[]>): Promise<void> {
  let text = await readFile(path, "utf8");
  for (const [name, rows] of Object.entries(given))
    text = text.replace(
      new RegExp(`<!-- ${name}:start -->[\\s\\S]*?<!-- ${name}:end -->`),
      () => `<!-- ${name}:start -->\n\n${rows.join("\n")}\n\n<!-- ${name}:end -->`,
    );
  await writeFile(path, text);
}
await written(path, tables);
// Each extension of the repository whose part for the TUI gives commands has its table in its own README.
for (const entry of await readdir(new URL("../../extensions/", import.meta.url), { withFileTypes: true })) {
  if (!entry.isDirectory()) continue;
  const root = new URL(`../../extensions/${entry.name}/`, import.meta.url);
  const manifest = JSON.parse(await readFile(new URL("package.json", root), "utf8")).furb as { tui?: string };
  if (!manifest.tui) continue;
  const part = (await import(new URL(manifest.tui, root).href)) as {
    default: () => TuiPart | Promise<TuiPart>;
  };
  const commands = (await part.default()).commands ?? {};
  await written(new URL("README.md", root), {
    commands: [
      "| Command | Action |",
      "| --- | --- |",
      ...Object.entries(commands).map(
        ([name, command]) =>
          `| \`/${name}${command.argument ? ` ${command.argument}` : ""}\` | ${command.detail} |`,
      ),
    ],
  });
}
