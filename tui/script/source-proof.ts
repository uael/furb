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
    async ({ session, app, screen, frame }) => {
      const stills: Still[] = [];
      let size = { width: 0, height: 0 };
      const capture = async (name: string, animate = false, expected?: string) => {
        await frame();
        await app.colorsDone();
        await frame();
        const spans = screen.captureSpans();
        await writeFile(`${out}/${name}.png`, rasterize(spans, paletteOf(session.theme), "furb"));
        const text = screen.captureCharFrame();
        if (expected && !text.includes(expected))
          throw new Error(`${name} does not show its expected state: ${expected}`);
        await writeFile(`${out}/${name}.txt`, text);
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
      await screen.mockMouse.moveTo(0, 0);
      screen.mockInput.pressArrow("down");
      if (!session.host.prompts.has(question)) throw new Error("The keyboard choice was already answered.");
      await capture("question-keyboard");
      if (!session.host.prompts.has(question)) throw new Error("The keyboard capture answered the question.");
      screen.mockInput.pressEnter();
      await until(session.host, () => !session.host.prompts.has(question));
      await session.engine.prompt("None", {
        message: "The next pass keeps the original source for copy and edit.",
        to: "operator",
        on: session.engine.root,
      });
      await session.refresh();
      await capture("notice");
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
      const handoff: Still[] = [];
      let dimensions = { width: 0, height: 0 };
      const capture = async (name: string, animate = false, expected?: string) => {
        await frame();
        await app.colorsDone();
        await frame();
        await writeFile(
          `${out}/${name}.png`,
          rasterize(screen.captureSpans(), paletteOf(session.theme), "furb"),
        );
        const text = screen.captureCharFrame();
        if (expected && !text.includes(expected))
          throw new Error(`${name} does not show its expected state: ${expected}`);
        await writeFile(`${out}/${name}.txt`, text);
        if (animate) {
          const image = pixels(screen.captureSpans(), paletteOf(session.theme), "furb", 1);
          dimensions = image;
          handoff.push(pack(image.pixels, 60));
        }
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
      addStep("prompt3", 9);
      await save();
      await until(session, () => session.acts.some((act) => act.id === `rung${rung}`));
      app.scroll.scrollTo(99999);
      await capture("dense-three-digit-wide");
      const pending = `rung${++rung}`;
      facts.push(["rung", pending, "prompt3", "chain1", "", "", ""]);
      await save();
      await until(session, () => session.acts.some((act) => act.id === pending));
      const actual = session.host.streams;
      const held = new Map([[pending, { text: "", thinking: "" }]]);
      Object.defineProperty(session.host, "streams", { configurable: true, get: () => held, set: () => {} });
      try {
        app.scroll.scrollTo(99999);
        await capture("thinking-empty", true, "# thinking");
        for (const [n, thinking] of [
          "Read the source",
          "Keep the source and display coordinates apart",
          "Check the active line",
        ].entries()) {
          held.set(pending, { text: "", thinking });
          app.scroll.scrollTo(99999);
          await capture(`thinking-${n}`, true, `# ${thinking}`);
        }
        held.set(pending, { text: 'answer = "a partial', thinking: "" });
        await capture("stream-partial", true, 'answer = "a partial');
        held.set(pending, { text: 'answer = "a partial statement"\nclose(answer)', thinking: "" });
        await capture("stream-source", true, 'answer = "a partial statement"');
        screen.resize(76, 38);
        await capture("stream-narrow", false, 'answer = "a partial statement"');
        screen.resize(140, 44);
        held.set(pending, { text: 'value = "steady source"\nif Tr', thinking: "" });
        await capture("handoff-partial", true, 'value = "steady source"');
        const rows = (await frame()).split("\n");
        const y = rows.findIndex((row) => row.includes('value = "steady source"'));
        const x = (rows[y] ?? "").indexOf("steady source");
        await screen.mockMouse.drag(x, y, x + 5, y);
        await capture("handoff-selection", true);
        const word = 'value = "steady source"\nif True:\n    answer = value';
        held.set(pending, { text: word, thinking: "Keep the current statement" });
        await capture("handoff-streamed", true);
        held.clear();
        await capture("handoff-gap", true);
        facts.push(
          ["run", `run${rung}`, "chain1", "chain1", pending, word, ""],
          ["done", `run${rung}`, "", null],
          ["done", pending, "", null],
        );
        await save();
        await until(session, () => session.acts.some((act) => act.id === pending && act.done));
        await capture("handoff-word", true);
        if (screen.renderer.getSelection()?.getSelectedText() !== "steady")
          throw new Error("The complete word lost the source selection.");
        await writeFile(`${out}/handoff.gif`, gif(handoff, dimensions.width, dimensions.height));
      } finally {
        Object.defineProperty(session.host, "streams", { configurable: true, writable: true, value: actual });
      }
    },
    { width: 140, height: 44, useMouse: true },
    dense,
  );
} finally {
  await removeDemoDirectories();
}
