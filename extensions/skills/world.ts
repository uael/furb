import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, resolve } from "node:path";
import type { Fact, WorldContext, WorldPart } from "@furb/engine";

/** A skill as the World answers it: plain data, of which the word makes its Skill. */
export interface Found {
  name: string;
  description: string;
  path: string;
}

/** The folders that hold skills, in the order in which a name wins: the project first, then the user. */
export function roots(directory: string, config: string): string[] {
  return [join(directory, ".furb", "skills"), join(directory, ".claude", "skills"), join(config, "skills")];
}

/** The name and the description that the frontmatter of a SKILL.md file gives: the lines between a first line `---`
 * and the next line `---`, each key at the start of a line, and a value as it stands, in quotes, or as a block that
 * `|` keeps line by line and `>` folds into one line. Nothing for a file that opens with no frontmatter. */
export function frontmatter(text: string): { name?: string; description?: string } {
  const lines = text.split(/\r?\n/);
  if (lines[0]?.trim() !== "---") return {};
  const end = lines.indexOf("---", 1);
  const held = lines.slice(1, end < 0 ? undefined : end);
  const got: { name?: string; description?: string } = {};
  for (let at = 0; at < held.length; at++) {
    const key = /^(name|description):\s*(.*)$/.exec(held[at] ?? "");
    if (!key) continue;
    const [, field, rest = ""] = key;
    let value = rest.trim();
    if (/^[|>][+-]?$/.test(value)) {
      const block: string[] = [];
      while (at + 1 < held.length && /^(\s|$)/.test(held[at + 1] ?? ""))
        block.push((held[++at] ?? "").trim());
      while (block.at(-1) === "") block.pop();
      value = block.join(value.startsWith(">") ? " " : "\n");
    } else if (/^(".*"|'.*')$/.test(value)) value = value.slice(1, -1);
    got[field as "name" | "description"] = value;
  }
  return got;
}

/** The skills in folders: each folder in a root that holds a SKILL.md file is a skill, named by its frontmatter or
 * by its folder, and described by its frontmatter. A name found in an earlier root wins, a root that is missing
 * holds nothing, and the skills stand in the order of their names. The disk is read at each call, so a new skill
 * shows at the next question. */
export function found(folders: readonly string[]): Found[] {
  const named = new Map<string, Found>();
  for (const root of folders) {
    let entries: string[];
    try {
      entries = readdirSync(root);
    } catch {
      continue;
    }
    for (const entry of entries.sort()) {
      const path = resolve(root, entry, "SKILL.md");
      let text: string;
      try {
        if (!statSync(path).isFile()) continue;
        text = readFileSync(path, "utf8");
      } catch {
        continue;
      }
      const { name = entry, description = "" } = frontmatter(text);
      if (!named.has(name)) named.set(name, { name, description, path });
    }
  }
  return [...named.values()].sort((one, other) =>
    one.name < other.name ? -1 : one.name > other.name ? 1 : 0,
  );
}

/** The part of the skills extension for a World: it answers a skills question with the skills of the project and of
 * the user, as plain data, and hears nothing else. */
export default function skills(context: WorldContext): WorldPart {
  return {
    *hears([kind, qid]: Fact) {
      if (kind === "skills" && qid.startsWith("skills@"))
        yield ["done", qid, found(roots(context.directory, context.config))];
    },
  };
}
