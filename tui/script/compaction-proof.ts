import { appendFile, mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { until } from "../../bind/typescript/test/until.ts";
import { demoSession } from "../src/demo.ts";
import { loadParsers } from "../src/parsers.ts";
import { paletteOf } from "../src/theme.ts";
import { compactionFacts, selfCompactionFacts } from "../test/compaction-fixture.ts";
import { composing } from "../test/composing.ts";
import { rasterize } from "./raster.ts";
import * as stage from "./stage.ts";

const out = resolve(process.env.FURB_COMPACTION_OUT ?? "docs/compaction");
await mkdir(out, { recursive: true });
await loadParsers();
const record = `${out}/inert.jsonl`;
await writeFile(
  record,
  [
    JSON.stringify({ recorded: true }),
    ...compactionFacts("/project").map((fact) => JSON.stringify(fact)),
    "",
  ].join("\n"),
);
const session = await demoSession({ record, cwd: out });
await composing(
  async ({ session, app, screen, frame }) => {
    session.preferences.reducedMotion = true;
    await session.select("chain1");
    const capture = async (name: string) => {
      await frame();
      await app.colorsDone();
      await frame();
      await writeFile(
        `${out}/${name}.png`,
        rasterize(screen.captureSpans(), paletteOf(session.theme), "furb"),
      );
      await writeFile(
        `${out}/${name}.txt`,
        screen
          .captureCharFrame()
          .split("\n")
          .map((line) => line.trimEnd())
          .join("\n")
          .trimEnd() + "\n",
      );
    };
    const focused = app.composer.focused;
    await capture("wide");
    screen.resize(80, 24);
    await capture("narrow");
    if (app.composer.focused !== focused) throw new Error("A passive resize changed the input focus.");
    const rows = (await frame()).split("\n");
    const y = rows.findIndex((row) => row.includes('summary = "Keep the exact'));
    const x = (rows[y] ?? "").indexOf("...");
    if (x < 0 || y < 0) throw new Error("The narrow summary has no fold to expand.");
    await screen.mockMouse.click(x + 1, y);
    await screen.mockMouse.moveTo(0, 0);
    await capture("expanded-narrow");
    let copied = "";
    screen.renderer.copyToClipboardOSC52 = (text) => {
      copied = text;
      return true;
    };
    const expanded = (await frame()).split("\n");
    const summaryRow = expanded.findIndex((row) => row.includes('summary = "Keep the exact'));
    await screen.mockMouse.click(20, summaryRow, 2);
    await frame();
    await stage.click(screen, "Copy its source");
    await frame();
    const original = String(session.actOf("rung5")?.words[0]);
    if (copied !== original || !copied.includes("The next step verifies the answer."))
      throw new Error("Copy lost part of the original summary.");
    await writeFile(`${out}/copied.txt`, `${copied}\n`);
    app.scroll.scrollTo(99999);
    await capture("expanded-narrow-tail");
    if (!/Do not drop\s+this final sentence\./.test(await frame()))
      throw new Error("The expanded summary tail is not visible after scrolling.");

    screen.resize(160, 48);
    await capture("expanded-wide");
    session.folds = { "thread:prompt1": true };
    await capture("thread-collapsed");
    session.folds = {};
    const append = async (facts: unknown[][]) => {
      await appendFile(record, facts.map((fact) => JSON.stringify(fact)).join("\n") + "\n");
    };
    const prior = Array.from({ length: 99 }, (_, i) => `value_${i + 1} = ${i + 1}`).join("\n");
    await append([
      ["prompt", "prompt10", "operator", "chain1", "str", "Continue after the cut", "codex:gpt-6.1-sol/high"],
      ["rung", "rung10", "prompt10", "chain1", prior, "", ""],
      ["run", "run10", "chain1", "chain1", "rung10", prior, ""],
      ["done", "run10", "", null],
      ["done", "rung10", "", null],
      ["rung", "rung11", "prompt10", "chain3", "", "", ""],
    ]);
    await until(session, () => Boolean(session.actOf("rung11")));
    const streams = session.host.streams;
    const live = new Map([
      ["rung11", { text: 'continuation = "steady"', thinking: "Check the final value" }],
    ]);
    Object.defineProperty(session.host, "streams", { configurable: true, get: () => live, set: () => {} });
    try {
      await frame();
      app.scroll.scrollTo(99999);
      await capture("live-wide");
      const running = await frame();
      if (!/100 continuation =/.test(running) || !running.includes("# Check the final value"))
        throw new Error("The compacted thread lost its live continuation or gutter.");
      screen.resize(80, 24);
      app.scroll.scrollTo(99999);
      await capture("live-narrow");
      const lines = (await frame()).split("\n");
      const row = lines.findIndex((line) => line.includes('continuation = "steady"'));
      const column = (lines[row] ?? "").indexOf("steady");
      if (row < 0 || column < 0) throw new Error("The live source is outside the viewport.");
      await screen.mockMouse.drag(column, row, column + 5, row);
      if (screen.renderer.getSelection()?.getSelectedText() !== "steady")
        throw new Error("The live source could not be selected.");
      const inputFocus = app.composer.focused;
      live.set("rung11", { text: 'continuation = "steady"\nanswer = continuation', thinking: "" });
      await frame();
      live.clear();
      await frame();
      await append([
        ["run", "run11", "chain3", "chain3", "rung11", 'continuation = "steady"\nanswer = continuation', ""],
        ["done", "run11", "", null],
        ["done", "rung11", "", null],
      ]);
      await until(session, () => Boolean(session.actOf("rung11")?.done));
      await capture("finished-narrow");
      if (
        screen.renderer.getSelection()?.getSelectedText() !== "steady" ||
        app.composer.focused !== inputFocus
      )
        throw new Error("Completion lost source selection or input focus.");
      if (/# thinking|# Check the final value/.test(await frame()))
        throw new Error("The finished source kept its live tail.");
      screen.renderer.clearSelection();
      screen.resize(160, 48);
      app.scroll.scrollTo(99999);
      await capture("finished-wide");
    } finally {
      Object.defineProperty(session.host, "streams", { configurable: true, writable: true, value: streams });
    }
    await writeFile(
      `${out}/evidence.json`,
      JSON.stringify(
        {
          selected: session.selected,
          chains: session.chains.map((chain) => chain.id),
          cuts: session.compactions,
          passiveFocusPreserved: true,
          reader: session.host.recorded,
          scriptedStreamSelectionAndFocus: true,
          continuationGutter: 100,
          motion: session.preferences.reducedMotion,
        },
        null,
        2,
      ) + "\n",
    );
  },
  { width: 160, height: 48, useMouse: true },
  session,
);

const generatedDirectory = await mkdtemp(join(tmpdir(), "furb-self-compaction-proof-"));
try {
  const generated = await selfCompactionFacts(generatedDirectory);
  const record = join(generatedDirectory, "inert.jsonl");
  await writeFile(
    record,
    [JSON.stringify({ recorded: true }), ...generated.facts.map((fact) => JSON.stringify(fact)), ""].join(
      "\n",
    ),
  );
  const actual = await demoSession({ record, cwd: generatedDirectory });
  await composing(
    async ({ session, app, screen, frame }) => {
      session.preferences.reducedMotion = true;
      for (const [name, width, height] of [
        ["same-thread-wide", 160, 48],
        ["same-thread-narrow", 80, 24],
      ] as const) {
        screen.resize(width, height);
        await frame();
        await app.colorsDone();
        await frame();
        const shown = screen.captureCharFrame();
        if (!/3 summary =/.test(shown) || !/5 after = before \+ 1/.test(shown))
          throw new Error("The actual compact helper lost its prompt's display coordinates.");
        await writeFile(
          `${out}/${name}.png`,
          rasterize(screen.captureSpans(), paletteOf(session.theme), "furb"),
        );
        await writeFile(
          `${out}/${name}.txt`,
          shown
            .split("\n")
            .map((line) => line.trimEnd())
            .join("\n")
            .trimEnd() + "\n",
        );
      }
      const helper = session.acts.find(
        (act) => act.kind === "rung" && String(act.words[0]).startsWith("<s:summary>"),
      );
      await writeFile(
        `${out}/same-thread.json`,
        JSON.stringify(
          {
            origin: "Official compact helper in a native session with scripted answers and no provider call",
            prompt: generated.prompt,
            helper: helper?.id,
            originalBy: helper?.by,
            originalParentPrompt: session.actOf(helper?.by ?? "")?.by,
            displayLines: { before: 1, compact: 2, summary: 3, reground: 4, after: 5, close: 6 },
            localSummaryLine: 1,
            localRegroundLine: 3,
          },
          null,
          2,
        ) + "\n",
      );
    },
    { width: 160, height: 48 },
    actual,
  );
} finally {
  await rm(generatedDirectory, { recursive: true, force: true });
}
