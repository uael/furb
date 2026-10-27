import { expect, test } from "bun:test";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { until } from "../../bind/typescript/test/until.ts";
import { clip, scrub, shortened } from "../script/fixture.ts";
import { openEngine } from "../src/bridge.ts";
import { Session } from "../src/session.ts";
import { composing } from "./composing.ts";

/** The recorded life of a conductor, which the demo and the gallery show. */
const conductor = join(import.meta.dir, "..", "fixtures", "conductor.jsonl");
/** A session on the recorded life, which shows its log and runs nothing. */
async function recorded(): Promise<Session> {
  const { engine, host } = await openEngine({ record: conductor });
  return new Session(engine, host, true);
}

test("the feed folds the earlier steps of a long thread into one line that a click opens", async () =>
  // One test holds both, since each test runs in a process of its own and two demo libraries at once save one list.
  composing(
    async ({ session, app, frame, click }) => {
      await session.refresh();
      const shown = await frame();
      const lines = shown.split("\n");
      // The sidebar lists the workspace and the threads of the life, and no tree of its chains.
      expect(lines.some((line) => line.includes("Threads"))).toBe(true);
      expect(shown).not.toContain("Chains");
      // The conductor runs one long word, under which its messages pile up: the earlier ones fold into one line.
      const folds = () => app.scroll.getChildren().filter((node) => node.id.startsWith("more-")).length;
      expect(folds()).toBeGreaterThan(0);
      const before = app.scroll.scrollHeight;
      const line = app.scroll.getChildren().find((node) => node.id.startsWith("more-"));
      const key = line?.id.slice("more-".length) ?? "";
      app.scroll.stickyScroll = false;
      app.scroll.scrollTo((line?.y ?? 0) - app.scroll.y + app.scroll.scrollTop);
      await frame();
      await click("earlier");
      await until(session, () => session.folds[key] === false);
      await frame();
      expect(app.scroll.scrollHeight).toBeGreaterThan(before);
    },
    { width: 152, height: 46, useMouse: true },
    await recorded(),
  ));

test("a recorded life runs nothing: what the operator sends to it fails with the reason", async () => {
  const session = await recorded();
  try {
    await session.refresh();
    expect(session.chains.length).toBe(23);
    expect(
      String(await session.engine.rung({ word: "x = 1", on: session.selected }).catch((error) => error)),
    ).toContain("This record is open to read");
  } finally {
    await session.dispose();
  }
});

test("the fixture of a recorded life holds no secret: no key, no address of mail, and no home directory", async () => {
  expect(
    scrub("gh auth ghp_abcdefghijklmnopqrstuvwxyz0123 for a@b.co in /Users/alice/src and API_KEY=s3cret"),
  ).toBe("gh auth [key] for someone@example.com in ~/src and API_KEY=[key]");
  expect(clip("one\ntwo\nthree", 2)).toBe("one\ntwo\n… 1 more lines");
  const text = await readFile(conductor, "utf8");
  expect(text).not.toMatch(/\/Users\/|\/home\/|gh[pousr]_[A-Za-z0-9]{20}|github_pat_\w{20}|sk-[\w-]{20}/);
});

test("the fixture keeps a word python: a string that makes its line too long is cut inside its quotes, and a line of a quote as it is", () => {
  const long = "x".repeat(50);
  const word = [
    `a = f("${long}", '${long}')  # ${long}`,
    `b = """${long}`,
    `${long} """ + "${long}"`,
    "<s:q>",
    `it's ${long}`,
    "</s:q>",
  ].join("\n");
  expect(shortened(word, 40, 4).split("\n")).toEqual([
    `a = f("xxxx…", 'xxxx…')  # ${long}`,
    'b = """xxxx…',
    'xxxx…""" + "xxxx…"',
    "<s:q>",
    "it's…",
    "</s:q>",
  ]);
});
