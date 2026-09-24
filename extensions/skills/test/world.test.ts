import { expect, test } from "bun:test";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import type { WorldContext } from "@furb/engine";
import skills, { found, frontmatter, roots } from "../world.ts";

/** A folder of the test, removed after it. */
async function folder(run: (at: string) => Promise<void>): Promise<void> {
  const at = await mkdtemp(join(tmpdir(), "furb-skills-"));
  try {
    await run(at);
  } finally {
    await rm(at, { recursive: true, force: true });
  }
}

/** A skill in a root: its folder and its SKILL.md file. */
async function skill(root: string, folder: string, text: string): Promise<string> {
  await mkdir(join(root, folder), { recursive: true });
  await writeFile(join(root, folder, "SKILL.md"), text);
  return join(root, folder, "SKILL.md");
}

test("the roots are the project first, then the project for claude, then the config of the user", () => {
  expect(roots("/p", "/c")).toEqual(["/p/.furb/skills", "/p/.claude/skills", "/c/skills"]);
});

test("the frontmatter gives a name and a description as they stand, quoted, folded or kept line by line", () => {
  expect(frontmatter("---\nname: brew\ndescription: Make tea.\nlicense: MIT\n---\nbody\n")).toEqual({
    name: "brew",
    description: "Make tea.",
  });
  expect(frontmatter("---\r\nname: \"brew\"\r\ndescription: 'Make: tea'\r\n---\r\n")).toEqual({
    name: "brew",
    description: "Make: tea",
  });
  expect(frontmatter("---\nname: brew\ndescription: >\n  Make tea\n  for two.\n---\n")).toEqual({
    name: "brew",
    description: "Make tea for two.",
  });
  expect(frontmatter("---\ndescription: |\n  One.\n  Two.\n\nname: brew\n---\n")).toEqual({
    name: "brew",
    description: "One.\nTwo.",
  });
  expect(frontmatter("# No frontmatter\n")).toEqual({});
});

test("a skill is a folder of a root with a SKILL.md, named by its folder when its frontmatter names none", async () => {
  await folder(async (at) => {
    const root = join(at, "skills");
    const path = await skill(root, "steep", "Wait for the leaves.\n");
    await mkdir(join(root, "empty"));
    await writeFile(join(root, "loose.md"), "not a skill\n");
    expect(found([root, join(at, "missing")])).toEqual([{ name: "steep", description: "", path }]);
  });
});

test("a name of an earlier root wins, and the skills stand in the order of their names", async () => {
  await folder(async (at) => {
    const [project, claude, config] = roots(join(at, "p"), join(at, "c"));
    if (!project || !claude || !config) throw new Error("three roots");
    const brew = await skill(project, "brew", "---\nname: brew\ndescription: Mine.\n---\n");
    await skill(config, "brew", "---\nname: brew\ndescription: Theirs.\n---\n");
    const steep = await skill(claude, "steep", "---\ndescription: Wait.\n---\n");
    const boil = await skill(config, "boil", "---\nname: boil\n---\n");
    expect(found([project, claude, config])).toEqual([
      { name: "boil", description: "", path: boil },
      { name: "brew", description: "Mine.", path: brew },
      { name: "steep", description: "Wait.", path: steep },
    ]);
  });
});

test("the part answers a skills question with the skills of the project and of the user, and nothing else", async () => {
  await folder(async (at) => {
    const path = await skill(join(at, "p", ".furb", "skills"), "brew", "---\ndescription: Make tea.\n---\n");
    const part = skills({ directory: join(at, "p"), config: join(at, "c") } as WorldContext);
    expect(part.hears?.(["skills", "skills@rung4.1", "rung4", "chain1"])?.next().value).toEqual([
      "done",
      "skills@rung4.1",
      [{ name: "brew", description: "Make tea.", path }],
    ]);
    expect(part.hears?.(["read", "read@rung4.2", "rung4", "chain1", "a.txt"])?.next().done).toBe(true);
  });
});
