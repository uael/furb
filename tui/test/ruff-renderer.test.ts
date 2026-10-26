import { expect, test } from "bun:test";
import { appendFile, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Session as ScriptedSession } from "furb";
import { until } from "../../bind/typescript/test/until.ts";
import { openEngine } from "../src/bridge.ts";
import { Preferences } from "../src/preferences.ts";
import { Session } from "../src/session.ts";
import { composing } from "./composing.ts";
import { original, ruffFixture } from "./ruff-fixture.ts";

async function displayed(frame: () => Promise<string>, text: string): Promise<void> {
  let shown = "";
  for (let at = 0; at < 300; at++) {
    shown = await frame();
    if (
      shown.includes(text) &&
      !shown
        .split("\n")
        .find((row) => row.includes("result = await bash("))
        ?.includes("stdin=")
    )
      return;
    await Bun.sleep(10);
  }
  throw new Error(`No ${text} in the rendered source:\n${shown}`);
}

test.skipIf(!Bun.which("ruff"))(
  "Ruff display at 160/80 keeps original full-copy, raw preference, live tail, focus, folds and appended numbers",
  async () => {
    const fixture = await ruffFixture();
    try {
      await composing(
        async ({ app, session, screen, frame, click }) => {
          await displayed(frame, 'stdin="keep the whole payload",');
          let shown = await frame();
          expect(shown).toContain("result = await bash(");
          expect(shown).toContain("streamed = [1,");
          expect(shown).toContain("# Choosing the next value");
          const resultRow = shown.split("\n").find((row) => row.includes("result = await bash("));
          expect(resultRow).not.toContain("stdin=");
          expect(shown).toMatch(/\b7 next_value = 2/);
          const focused = app.composer.focused;
          let copied = "";
          screen.renderer.copyToClipboardOSC52 = (text) => {
            copied = text;
            return true;
          };
          let rows = shown.split("\n");
          let y = rows.findIndex((row) => row.includes("result = await bash("));
          let x = (rows[y] ?? "").indexOf("result");
          await screen.mockMouse.click(x + 2, y, 2);
          await frame();
          await click("Copy its source");
          expect(copied).toBe(original);
          await session.command("/python raw");
          shown = await frame();
          expect(shown.split("\n").find((row) => row.includes("result = await bash("))).toContain("stdin=");
          expect(session.preferences.rawPython).toBe(true);
          expect(new Preferences(session.preferences.path).rawPython).toBe(true);
          // The real key and the slash command share the same saved preference.
          screen.mockInput.pressKey("r", { ctrl: true, meta: true });
          await screen.waitFor(() => !session.preferences.rawPython);
          await displayed(frame, 'stdin="keep the whole payload",');
          expect(new Preferences(session.preferences.path).rawPython).toBe(false);
          screen.resize(80, 24);
          await frame();
          expect(app.composer.focused).toBe(focused);
          app.scroll.scrollTo(0);
          shown = await frame();
          rows = shown.split("\n");
          y = rows.findIndex((row) => row.includes("brief = "));
          x = (rows[y] ?? "").indexOf("...");
          if (x < 0) throw new Error(JSON.stringify(rows));
          await screen.mockMouse.click(x + 1, y);
          await frame();
          app.scroll.scrollTo(99999);
          expect(await frame()).toContain("Last instruction");
          // Restore the fold, then a stable append must not renumber the completed prefix.
          session.folds = { "thread:prompt1": false };
          screen.resize(160, 48);
          const prefix = (await frame()).split("\n").find((row) => row.includes("next_value ="));
          const appended = [
            ["rung", "rung4", "prompt1", "chain1", "appended = 4", "", ""],
            ["run", "run4", "chain1", "chain1", "rung4", "appended = 4", ""],
            ["done", "run4", "", null],
            ["done", "rung4", "", null],
          ];
          await appendFile(fixture.record, appended.map((fact) => JSON.stringify(fact)).join("\n") + "\n");
          await until(session, () => Boolean(session.actOf("rung4")));
          shown = await frame();
          expect(shown.split("\n").find((row) => row.includes("next_value ="))).toBe(prefix);
          expect(shown).toContain("appended = 4");
          expect(session.preferences.still).toBe(true);
        },
        { width: 160, height: 48, useMouse: true, kittyKeyboard: true },
        fixture.session,
      );
    } finally {
      await fixture.remove();
    }
  },
);

test.skipIf(!Bun.which("ruff"))(
  "native scripted source remains exactly the edit program after Ruff projection",
  async () => {
    const directory = await mkdtemp(join(tmpdir(), "furb-ruff-edit-"));
    const record = join(directory, "scripted.jsonl");
    const word =
      original.replace(
        /result = await bash.*$/,
        'result = dict(first="original-source", second="keep the whole payload", third=600, fourth="chain1", fifth="force the actual one-hundred-column projection")',
      ) + "\nclose(None)";
    const scripted = new ScriptedSession({
      cwd: directory,
      record,
      config: join(directory, "config"),
      model: "claude-cli:sonnet",
      answer: async () => ["assistant", word, null, null],
    });
    let composed = false;
    let session: Session | undefined;
    try {
      const engine = scripted.open();
      const prompt = engine.prompt("None", { message: "Keep exact source", on: engine.root });
      await prompt;
      const id = prompt.id;
      await scripted.dispose();
      const opened = await openEngine({ demo: true, record, cwd: directory });
      session = new Session(opened.engine, opened.host, true, new Preferences(join(directory, "ui.json")));
      await session.refresh();
      composed = true;
      await composing(
        async ({ app, session, frame }) => {
          // Wait for the actual local Ruff result, not merely the first (still raw) render.
          for (let at = 0; ; at++) {
            const shown = await frame();
            const result = shown.split("\n").find((row) => row.includes("result = dict("));
            if (result && !result.includes("first=") && shown.includes('second="keep the whole payload",'))
              break;
            if (at === 299) throw new Error(`No actual Ruff projection before editing:\n${shown}`);
            await Bun.sleep(10);
          }
          expect(session.preferences.rawPython).toBe(false);
          await session.command(`/edit ${id}`);
          await frame();
          expect(session.editing).toBe(id);
          expect(app.composer.plainText.trimEnd()).toBe(word);
          expect(app.composer.plainText).toContain('text = """one "quoted" line\nsecond line 雪😀"""');
          expect(app.composer.plainText).toContain('path = r"""C:\\new\n\\drafts"""');
        },
        { width: 160, height: 48 },
        session,
      );
    } finally {
      if (session && !composed) await session.dispose();
      await scripted.dispose();
      await rm(directory, { recursive: true, force: true });
    }
  },
);
