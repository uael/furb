import { afterAll, expect, test } from "bun:test";
import { existsSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { until } from "../../bind/typescript/test/until.ts";
import { removeDemoDirectories } from "../src/demo.ts";
import type { Session } from "../src/session.ts";
import { composing, withDemo } from "./composing.ts";
import { idle } from "./idle.ts";

afterAll(removeDemoDirectories);

/** The demo up to the point after its first answer, then a message whose answer writes the README, and that point. */
async function shortcut(session: Session): Promise<{ point: string; readme: string; path: string }> {
  const { engine } = session;
  const on = engine.root;
  const path = join(session.host.directory, "README.md");
  const readme = await readFile(path, "utf8");
  await engine.result(await engine.prompt("str", { message: "Read this project and say what it does.", on }));
  await session.refresh();
  // The rung that answered the first message.
  const asked = session.activity.find((act) => session.isUserPrompt(act))?.id;
  const point = session.acts.find((act) => act.kind === "rung" && act.by === asked)?.id ?? "";
  await engine.result(await engine.prompt("str", { message: "Add the search shortcut, Ctrl+K.", on }));
  await idle(session);
  await session.refresh();
  expect(await readFile(path, "utf8")).not.toBe(readme);
  return { point, readme, path };
}

test("a branch at rung1 of the demo with its files gives back the README as it was", () =>
  withDemo(async (session) => {
    const { point, readme, path } = await shortcut(session);
    expect(point).toBe("rung1");
    const found = await session.filesAt(point);
    expect(found.files.map((change) => change.path)).toEqual([path]);
    const source = session.selected;
    const branch = await session.rewind(point, "Before the shortcut", found);
    expect(branch).not.toBe(source);
    expect(await readFile(path, "utf8")).toBe(readme);
    // The restore is a rung of the operator on the branch, so the record holds its write.
    const restore = session.acts.findLast((act) => act.kind === "write" && act.on === branch);
    expect(restore?.done).toBe(true);
    expect(session.acts.find((act) => act.id === restore?.by)?.by).toBe("operator");
    expect(session.notice).toContain("1 file is back as it was");
  }));

test("a restore of the files alone keeps the conversation, and says that a command is not undone", () =>
  withDemo(async (session) => {
    const { point, readme, path } = await shortcut(session);
    // The first answer ran a command, so a rewind to the message before it counts that command.
    const first = session.activity.find((act) => session.isUserPrompt(act))?.id ?? "";
    const before = await session.filesAt(first);
    expect(before.commands).toBe(1);
    expect(session.restored(before)).toContain("a change that a command made is not restored");
    const source = session.selected;
    await session.restoreAt(point);
    expect(session.selected).toBe(source);
    expect(await readFile(path, "utf8")).toBe(readme);
    expect(session.notice).toContain("The conversation goes on as it stands.");
  }));

test("the rewind tree offers the conversation, the files, or both, when a write changed a file after the point", () =>
  composing(async ({ session, app, frame, screen }) => {
    const { readme, path } = await shortcut(session);
    const source = session.selected;
    app.rewind();
    await frame();
    // The pointer stands on the last message, whose answer wrote the README.
    screen.mockInput.pressEnter();
    let shown = await frame();
    for (let tries = 0; tries < 400 && !shown.includes("Restore the files"); tries++) {
      await Bun.sleep(10);
      shown = await frame();
    }
    expect(shown).toContain("Restore the conversation");
    expect(shown).toContain("Both");
    expect(shown).toContain("README.md");
    expect(shown).toContain("A change that a command made is not restored.");
    screen.mockInput.pressArrow("down");
    screen.mockInput.pressArrow("down");
    screen.mockInput.pressEnter();
    await until(session, () => session.notice.includes("back as it was") || Boolean(session.error));
    expect(session.error).toBe("");
    expect(session.selected).not.toBe(source);
    expect(await readFile(path, "utf8")).toBe(readme);
  }));

test("a restore removes a file that a write made after the point, with a command", () =>
  withDemo(async (session) => {
    const { point, readme, path } = await shortcut(session);
    const made = join(session.host.directory, "NOTES.md");
    const on = session.engine.root;
    await session.engine.result(await session.engine.rung({ word: 'write(Text("NOTES.md", "new\\n"))', on }));
    await session.refresh();
    expect(await readFile(made, "utf8")).toBe("new\n");
    const found = await session.filesAt(point);
    expect(found.files.find((change) => change.path === made)?.made).toBe(true);
    await session.restoreAt(point, found);
    expect(existsSync(made)).toBe(false);
    expect(await readFile(path, "utf8")).toBe(readme);
  }));
