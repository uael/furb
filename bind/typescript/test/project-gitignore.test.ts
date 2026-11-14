import { expect, test } from "bun:test";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { furbDirectory, IGNORED } from "../src/project.ts";

async function inProject(body: (project: string) => void): Promise<void> {
  const project = await mkdtemp(join(tmpdir(), "furb-project-gitignore-"));
  try {
    body(project);
  } finally {
    await rm(project, { recursive: true, force: true });
  }
}

test("a project whose .furb stands with no .gitignore gets the .gitignore of the state paths when it is opened", async () => {
  await inProject((project) => {
    mkdirSync(join(project, ".furb"));
    furbDirectory(project, "sessions");
    expect(readFileSync(join(project, ".furb", ".gitignore"), "utf8")).toBe(IGNORED);
    expect(existsSync(join(project, ".furb", "sessions"))).toBe(true);
  });
});

test("a project with no .furb gets the same .gitignore as one whose .furb stood before", async () => {
  await inProject((project) => {
    furbDirectory(project, "sessions");
    expect(readFileSync(join(project, ".furb", ".gitignore"), "utf8")).toBe(IGNORED);
  });
});

test("a .gitignore that the user edited stays as it was when the project is opened", async () => {
  await inProject((project) => {
    mkdirSync(join(project, ".furb"));
    writeFileSync(join(project, ".furb", ".gitignore"), "mine\n");
    furbDirectory(project, "sessions");
    furbDirectory(project, "shares");
    expect(readFileSync(join(project, ".furb", ".gitignore"), "utf8")).toBe("mine\n");
  });
});
