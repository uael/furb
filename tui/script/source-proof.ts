import { mkdir, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { until } from "../../bind/typescript/test/until.ts";
import { demoSession, removeDemoDirectories } from "../src/demo.ts";
import { loadParsers } from "../src/parsers.ts";
import { paletteOf } from "../src/theme.ts";
import { composing } from "../test/composing.ts";
import { gif, pack, type Still } from "./gif.ts";
import { pixels, rasterize } from "./raster.ts";

const out = resolve(process.env.FURB_SOURCE_OUT ?? "docs/source");
await mkdir(out, { recursive: true });
await loadParsers();
try {
  await composing(
    async ({ session, app, screen, frame, click }) => {
      const stills: Still[] = [];
      let size = { width: 0, height: 0 };
      const capture = async (name: string, animate = false) => {
        await frame();
        await app.colorsDone();
        await frame();
        const spans = screen.captureSpans();
        await writeFile(`${out}/${name}.png`, rasterize(spans, paletteOf(session.theme), "furb"));
        await writeFile(`${out}/${name}.txt`, screen.captureCharFrame());
        if (animate) {
          const image = pixels(spans, paletteOf(session.theme), "furb", 1);
          size = image;
          stills.push(pack(image.pixels, 15));
        }
      };
      const long = "one line with a quote, a path and clear source. ".repeat(8);
      const source = [
        `<s:brief>\n${long}\nsecond line\n</s:brief>`,
        `text = """${long}\nnext line"""`,
        `path = r"""C:\\notes\n\\drafts"""`,
        "await wait(60)",
      ].join("\n");
      const rung = await session.engine.rung({ word: source, on: session.engine.root });
      await session.refresh();
      await capture("source-wide", true);
      const rows = (await frame()).split("\n");
      const y = rows.findIndex((row) => row.includes("brief = "));
      const x = (rows[y] ?? "").indexOf("...");
      if (x >= 0) {
        await screen.mockMouse.click(x + 1, y);
        await screen.mockMouse.moveTo(0, 0);
        await capture("source-expanded", true);
      }
      screen.resize(76, 38);
      await capture("source-expanded-narrow");
      session.folds = {};
      await capture("source-narrow");
      screen.resize(140, 44);
      await session.engine.cancel(rung);
      session.folds = {};
      const question = await session.engine.prompt("Literal['yes', 'no']", {
        message: "Keep the Python surface and continue with this change?",
        to: "operator",
        on: session.engine.root,
      });
      await until(session.host, () => session.host.prompts.has(question));
      await session.refresh();
      await capture("question");
      await click("yes");
      await screen.mockMouse.moveTo(0, 0);
      screen.mockInput.pressArrow("down");
      await capture("question-keyboard");
      screen.mockInput.pressEnter();
      await until(session.host, () => !session.host.prompts.has(question));
      await session.engine.prompt("None", {
        message: "The next pass keeps the original source for copy and edit.",
        to: "operator",
        on: session.engine.root,
      });
      await session.refresh();
      await capture("notice");
      // A scripted demo reply, never a network model, provides the real pending rung.
      process.env.FURB_DEMO_PACE = "200";
      await session.submit("show live progress");
      await until(session.host, () => session.host.streams.size > 0);
      const stream = [...session.host.streams.entries()][0];
      if (stream) {
        const [id, initial] = stream;
        const actual = session.host.streams;
        const held = new Map([[id, { ...initial, text: "", thinking: "" }]]);
        Object.defineProperty(session.host, "streams", {
          configurable: true,
          get: () => held,
          set: () => {},
        });
        await capture("thinking-empty", true);
        for (const [n, thinking] of [
          "Read the source",
          "Keep the source and display coordinates apart",
          "Check the active line",
        ].entries()) {
          held.set(id, { ...initial, text: "", thinking });
          await capture(`thinking-${n}`, true);
        }
        held.set(id, { ...initial, thinking: "", text: 'answer = "a partial' });
        await capture("stream-partial", true);
        held.set(id, { ...initial, thinking: "", text: 'answer = "a partial statement"\nclose(answer)' });
        await capture("stream-source", true);
        screen.resize(76, 38);
        await capture("stream-narrow");
        screen.resize(140, 44);
        Object.defineProperty(session.host, "streams", { configurable: true, writable: true, value: actual });
      }
      if (stills.length) await writeFile(`${out}/source.gif`, gif(stills, size.width, size.height));
    },
    { width: 140, height: 44, useMouse: true, kittyKeyboard: true },
  );
  // This inert fixture exercises dense code and appended steps without running a model or its words.
  const record = `${out}/dense.jsonl`;
  const facts: unknown[][] = [
    ["chain", "chain1", "operator", "", "Feed source", ""],
    ["stand", "stand1", "operator", "chain1"],
    [
      "done",
      "stand1",
      "",
      [
        [
          ["codex:gpt-6.1-sol", ["high"], 475000],
          ["operator", [], 200000],
        ],
        out,
        "codex:gpt-6.1-sol/high",
      ],
    ],
  ];
  let rung = 0;
  const addStep = (prompt: string, n: number) => {
    const id = `rung${++rung}`;
    const word = [
      `# Pass ${n}: retain the source as the next step arrives`,
      `def choose_${n}(items: list[str]) -> list[str]:`,
      "    chosen = []",
      "    for item in items:",
      "        if item.strip():",
      "            chosen.append(item.strip())",
      "    return chosen",
      `index = ${n}`,
      "while index > 0:",
      "    index -= 1",
      `result_${n} = choose_${n}(["source", "display", "selection"])`,
      `assert len(result_${n}) == 3`,
    ].join("\n");
    facts.push(
      ["rung", id, prompt, "chain1", word, "", ""],
      ["run", `run${rung}`, "chain1", "chain1", id, word, ""],
      ["done", `run${rung}`, "", null],
      ["done", id, "", null],
    );
  };
  for (const [n, message] of [
    "Inspect the source",
    "Keep the literal values",
    "Finish the continuous program",
  ].entries()) {
    const id = `prompt${n + 1}`;
    facts.push(["prompt", id, "operator", "chain1", "str", message, "codex:gpt-6.1-sol/high"]);
    for (let step = 1; step <= (n === 2 ? 7 : 2); step++) addStep(id, step);
    if (n < 2) facts.push(["done", id, "", "The source is intact."]);
  }
  const save = () =>
    writeFile(
      record,
      `${JSON.stringify({ recorded: true })}\n${facts.map((fact) => JSON.stringify(fact)).join("\n")}\n`,
    );
  await save();
  const dense = await demoSession({ record, cwd: out });
  await composing(
    async ({ session, app, screen, frame }) => {
      session.preferences.reducedMotion = true;
      for (const id of ["prompt1", "prompt2"]) session.folds[`thread:${id}`] = true;
      const capture = async (name: string) => {
        await frame();
        await app.colorsDone();
        await frame();
        await writeFile(
          `${out}/${name}.png`,
          rasterize(screen.captureSpans(), paletteOf(session.theme), "furb"),
        );
        await writeFile(`${out}/${name}.txt`, screen.captureCharFrame());
      };
      app.scroll.scrollTo(0);
      await capture("dense-folded-wide");
      addStep("prompt3", 8);
      await save();
      await until(session, () => session.acts.some((act) => act.id === `rung${rung}`));
      await session.refresh();
      app.scroll.scrollTo(99999);
      await capture("dense-appended-wide");
      screen.resize(76, 38);
      await capture("dense-appended-narrow");
      session.folds["steps:prompt3"] = false;
      await capture("dense-steps-expanded-narrow");
      screen.resize(140, 44);
      session.folds["thread:prompt2"] = false;
      await capture("dense-threads-expanded-wide");
    },
    { width: 140, height: 44, useMouse: true },
    dense,
  );
} finally {
  await removeDemoDirectories();
}
