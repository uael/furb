import { expect, test } from "bun:test";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { until } from "../../bind/typescript/test/until.ts";
import { openEngine } from "../src/bridge.ts";
import { typed } from "../src/configs.ts";
import { Preferences } from "../src/preferences.ts";
import { Session } from "../src/session.ts";
import { composing } from "./composing.ts";

test("a value that is no JSON is the text that the operator typed", () => {
  expect([typed("false"), typed("12"), typed('"a b"'), typed("a b")]).toEqual([false, 12, "a b", "a b"]);
});

test("/config shows each key with its schema, flips a flag, takes a key out and refuses a value of another kind", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-config-menu-"));
  const path = join(directory, ".furb", "config.json");
  await mkdir(join(directory, ".furb"));
  await writeFile(path, JSON.stringify({ other: 1 }));
  const opened = await openEngine({
    cwd: directory,
    record: join(directory, "life.jsonl"),
    config: join(directory, "user"),
    roster: [],
    extensions: false,
  });
  const session = new Session(opened.engine, opened.host, false, new Preferences(join(directory, "ui.json")));
  try {
    await session.refresh();
    const gate = (await session.host.settings()).find((one) => one.key.join(".") === "gate");
    expect([gate?.value, gate?.layer, gate?.property?.type]).toEqual([true, undefined, "boolean"]);
    await composing(
      async ({ app, frame, screen }) => {
        app.configs();
        await session.host.settings();
        expect(await frame()).toContain("Not in a schema");
        await screen.mockInput.typeText("gate");
        expect(await frame()).toContain("Read each word of a model");
        // A flag offers its two values, with the one that holds first, then the config to write.
        screen.mockInput.pressEnter();
        expect(await frame()).toContain("the default");
        screen.mockInput.pressArrow("down");
        screen.mockInput.pressEnter();
        expect(await frame()).toContain("Write gate = false to");
        screen.mockInput.pressEnter();
        await until(session, () => session.notice.startsWith("gate is false"));
        expect(JSON.parse(await readFile(path, "utf8"))).toEqual({ other: 1, gate: false });
        const refusal = await session.command("/config gate maybe").catch(String);
        expect(refusal).toContain("takes a boolean");
        await session.command("/config gate");
        expect(JSON.parse(await readFile(path, "utf8"))).toEqual({ other: 1 });
      },
      { width: 120, height: 44, useMouse: true },
      session,
    );
  } catch (error) {
    await session.dispose().catch(() => {});
    throw error;
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
