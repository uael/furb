import { expect, test } from "bun:test";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { until } from "../../bind/typescript/test/until.ts";
import { openEngine } from "../src/bridge.ts";
import { Preferences } from "../src/preferences.ts";
import { Session } from "../src/session.ts";
import { composing } from "./composing.ts";

test("the provider menu saves, restores and validates provider settings", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-provider-menu-"));
  const path = join(directory, ".furb", "config.json");
  await mkdir(join(directory, ".furb"));
  await writeFile(path, JSON.stringify({ max_thinking_tokens: 1234, extensions: { memory: false } }));
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
    await session.command("/provider add desk http://127.0.0.1:9/v1 qwen 65536");
    const saved = JSON.parse(await readFile(path, "utf8"));
    expect(saved).toEqual({
      max_thinking_tokens: 1234,
      extensions: { memory: false },
      providers: {
        desk: { url: "http://127.0.0.1:9/v1", protocol: "chat", models: { qwen: { window: 65536 } } },
      },
    });
    await composing(
      async ({ app, frame, click }) => {
        app.providers();
        await session.host.providers();
        expect(await frame()).toContain("Add a custom provider");
        expect(await frame()).toContain("desk");
        await click("desk");
        await until(session, () => session.notice.startsWith("Provider desk removed"));
        expect(JSON.parse(await readFile(path, "utf8")).providers.desk.enabled).toBe(false);
        app.providers();
        await session.host.providers();
        await click("desk");
        await until(session, () => session.notice.startsWith("Provider desk added"));
        expect(JSON.parse(await readFile(path, "utf8")).providers.desk).toEqual({
          ...saved.providers.desk,
          enabled: true,
        });
        const before = await readFile(path, "utf8");
        const refusal = await session
          .command("/provider add broken http://127.0.0.1:9/v1 qwen 0")
          .catch(String);
        expect(refusal).toContain("positive window");
        expect(await readFile(path, "utf8")).toBe(before);
        await session.command("/provider add response http://127.0.0.1:9/v1 qwen 65536 - openai");
        expect(
          session.catalog.filter(([name]) => name.startsWith("response:")).map(([name]) => name),
        ).toEqual(["response:qwen"]);
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
