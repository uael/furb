/** bun run bench: how fast the TUI draws a large life. It makes a recorded life of 30000 facts from the facts of the
 * life in tui/fixtures and opens it in the App on a test terminal. It measures the first frame, from the open of the
 * life to its first frame, once cold and twice warm. Then the renderer draws at its target rate while the feed
 * scrolls and a command streams a line of output into the life each frame, and it measures how many frames each
 * second it draws and how long each frame takes, the work of the view in it. It takes the best of the warm first
 * frames and the best of three such rounds for each measure, since other work of the machine makes a measure worse and
 * never better. It measures the machine too, with no code of furb: the frames each second that the renderer draws at
 * its target rate with nothing to show, and the pace, the time that the machine takes to parse each fact of the
 * record. Each budget is set from these, so that it holds on a fast machine and on a slow one. It fails when the warm
 * first frame takes 32 paces or more, when the renderer draws fewer than 95 in each hundred of the frames that it
 * draws with nothing to show, or when the 95th percentile of the frames takes a pace or more, or longer than a frame
 * at the target rate. FURB_BENCH_JSON names a file that takes one line of the measures of each run. */
import { appendFileSync, readFileSync, writeFileSync } from "node:fs";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { createTestRenderer } from "@opentui/core/testing";
import { App } from "../src/app.ts";
import { demoLibrary, demoSession } from "../src/demo.ts";
import { Preferences } from "../src/preferences.ts";
import { fps } from "../src/theme.ts";

/** The facts of the life that the bench draws, and the budgets that it holds the TUI to. */
const FACTS = Number(process.env.FURB_BENCH_FACTS) || 30000;
/** The budgets of the warm first frame and of the 95th percentile of the frames, in units of the pace of the
 * machine. On 120 runs of the bench on ubuntu-latest, the warm first frame took 17 to 26 paces, and the 95th
 * percentile 0.43 to 0.81 of a pace, against 0.89 to 1.38 before the cards of the feed were kept. */
const FIRST = 32;
const SMOOTH = 1;
const FRAME = 1000 / fps;
/** The steps of a round, one for each frame at the target rate: three seconds. */
const STEPS = fps * 3;
const ROUNDS = 3;

/** The facts of a recorded life of a count of facts: the facts of the fixture again and again, each copy with names
 * of its own, on the one root chain, so that each copy adds its chains, its prompts and its commands to the life. */
function life(count: number): unknown[][] {
  const lines = readFileSync(resolve(import.meta.dir, "..", "fixtures", "conductor.jsonl"), "utf8")
    .split("\n")
    .filter((line) => line.startsWith("["));
  const names = new Map<string, number>();
  for (const line of lines) {
    const found = /^([a-z_]+)(\d+)$/.exec(String((JSON.parse(line) as unknown[])[1]));
    if (found?.[1] && found[2]) names.set(found[1], Math.max(names.get(found[1]) ?? 0, Number(found[2])));
  }
  const facts: unknown[][] = [];
  for (let copy = 0; facts.length < count; copy++)
    for (const line of lines) {
      if (facts.length >= count) break;
      const renamed = copy
        ? line.replace(/\b([a-z_]+)(\d+)\b/g, (whole, kind: string, at: string) =>
            whole === "chain1" || !names.has(kind) || Number(at) > (names.get(kind) ?? 0)
              ? whole
              : `${kind}${Number(at) + copy * (names.get(kind) ?? 0)}`,
          )
        : line;
      const fact = JSON.parse(renamed) as unknown[];
      // The root chain stands once, and each copy adds its acts to it.
      if (copy && fact[1] === "chain1") continue;
      facts.push(fact);
    }
  return facts;
}

/** The value at a share of the sorted values, as a percentile. */
function percentile(values: readonly number[], share: number): number {
  const sorted = [...values].sort((one, other) => one - other);
  return sorted[Math.min(sorted.length - 1, Math.floor(share * sorted.length))] ?? 0;
}

const root = await mkdtemp(join(tmpdir(), "furb-bench-"));
const record = join(root, "life.jsonl");
const facts = life(FACTS);
writeFileSync(
  record,
  `${JSON.stringify({ recorded: true })}\n${facts.map((fact) => JSON.stringify(fact)).join("\n")}\n`,
);
const preferences = new Preferences(join(root, "ui.json"));
const add = (...added: unknown[][]) =>
  appendFileSync(record, added.map((fact) => `${JSON.stringify(fact)}\n`).join(""));
/** The highest number of a kind of act in the life, from which the bench names the acts that it adds. */
const top = (kind: string) =>
  Math.max(
    0,
    ...facts.flatMap(([, name]) =>
      String(name).startsWith(kind) ? [Number(String(name).slice(kind.length)) || 0] : [],
    ),
  );

/** A life opened in an App on a test terminal, the life of the bench by default, and the time from the open to its
 * first frame. */
async function open(at = record) {
  const began = performance.now();
  const session = await demoSession({ record: at, cwd: root, preferences });
  const library = await demoLibrary(session);
  const screen = await createTestRenderer({
    width: 152,
    height: 46,
    exitSignals: [],
    targetFps: fps,
    gatherStats: true,
    // The stats keep every frame of the rounds, which each round reads from where the last one ended.
    maxStatSamples: STEPS * ROUNDS * 10,
  });
  const app = new App(screen.renderer, session, { quit() {}, workspaces: library });
  app.render();
  await screen.flush();
  const first = performance.now() - began;
  const close = async () => {
    app.dispose();
    screen.renderer.destroy();
    await library.dispose();
  };
  return { session, app, screen, first, close };
}

/** One round: a command starts on the root chain under a rung of its own, and another process writes a line of its
 * output each frame, as a command writes it, so that no work of the bench takes time from the thread that draws.
 * The feed scrolls a third of its height each frame, up for the first half and down for the second. */
async function round(opened: Awaited<ReturnType<typeof open>>, at: number) {
  const { app, screen } = opened;
  const rung = `rung${top("rung") + at}`;
  const bash = `bash${top("bash") + at}`;
  add(
    ["rung", rung, "operator", "chain1", "await bash('seq 1000')", "", ""],
    ["started", rung, "chain1"],
    ["run", `run${top("run") + at}`, "chain1", "chain1", rung, "await bash('seq 1000')", ""],
    ["bash", bash, rung, "chain1", "seq 1000", "", 600],
    ["started", bash, "bash"],
  );
  const line = (step: string) =>
    `JSON.stringify(["out", ${JSON.stringify(bash)}, "bash", "line " + ${step} + " of a command that streams\\n", "stdout"]) + "\\n"`;
  const writer = Bun.spawn(
    [
      process.execPath,
      "-e",
      `const { appendFileSync } = require("node:fs");
let at = 0;
const step = () => {
  appendFileSync(${JSON.stringify(record)}, ${line("at")});
  if (++at < ${STEPS}) setTimeout(step, ${FRAME});
};
step();`,
    ],
    { stdout: "ignore", stderr: "inherit" },
  );
  const stats = screen.renderer.getStats();
  const skip = stats.frameTimes.length;
  const frames: number[] = [];
  const stamp = () => frames.push(performance.now());
  screen.renderer.on("frame", stamp);
  const began = performance.now();
  for (let step = 0; step < STEPS; step++) {
    app.scroll.scrollBy(step < STEPS / 2 ? -12 : 12);
    await Bun.sleep(Math.max(0, began + (step + 1) * FRAME - performance.now()));
  }
  const took = performance.now() - began;
  screen.renderer.off("frame", stamp);
  await writer.exited;
  add([
    "done",
    bash,
    "bash",
    { is: "Exit", code: 0, stdout: { is: "Text", content: "" }, stderr: { is: "Text", content: "" } },
  ]);
  const times = screen.renderer.getStats().frameTimes.slice(skip);
  return { rate: (frames.length * 1000) / took, times };
}

/** The frames each second that the renderer draws at its target rate with nothing to show: the timers of the runtime
 * wait a whole millisecond at the least, so a loop of 60 frames draws a few less than 60. */
async function ceiling(): Promise<number> {
  const screen = await createTestRenderer({ width: 152, height: 46, exitSignals: [], targetFps: fps });
  let frames = 0;
  screen.renderer.on("frame", () => frames++);
  screen.renderer.start();
  const began = performance.now();
  await Bun.sleep(2000);
  const rate = (frames * 1000) / (performance.now() - began);
  screen.renderer.destroy();
  return rate;
}

/** The time that this machine takes to parse each fact of the record of the bench, the best of ten: work that no
 * code of furb does, which says how fast the machine runs, as the empty renderer says how fast it draws. */
function pace(): number {
  const lines = readFileSync(record, "utf8").split("\n").filter(Boolean);
  let least = Number.POSITIVE_INFINITY;
  for (let at = 0; at < 10; at++) {
    const began = performance.now();
    for (const line of lines) JSON.parse(line);
    least = Math.min(least, performance.now() - began);
  }
  return least;
}

const empty = await ceiling();
const unit = pace();
const cold = await open();
await cold.close();
const warm = await open();
warm.screen.renderer.start();
const rounds: Awaited<ReturnType<typeof round>>[] = [];
for (let at = 1; at <= ROUNDS; at++) rounds.push(await round(warm, at));
const heard = warm.session.acts.length;
await warm.close();
// A second warm open, since a first frame that other work of the machine slows is one sample of a slow machine.
const again = await open();
await again.close();
await rm(root, { recursive: true, force: true });

/** The best of the rounds for each measure: other work of the machine makes a measure worse, and never better. */
const least = (of: (one: (typeof rounds)[number]) => number) => Math.min(...rounds.map(of));
const first = Math.min(warm.first, again.first);
/** Each measure, its unit, and its budget: a value that it must stay under, or the least that it must reach. */
const rows: [string, number, string, { under?: number; least?: number }][] = [
  ["pace of the machine", unit, "ms", {}],
  ["first frame, cold", cold.first, "ms", {}],
  ["first frame, warm", first, "ms", { under: FIRST * unit }],
  ["frames each second, empty", empty, "fps", {}],
  ["frames each second", -least((one) => -one.rate), "fps", { least: empty * 0.95 }],
  ["frame, median", least((one) => percentile(one.times, 0.5)), "ms", {}],
  [
    "frame, 95th percentile",
    least((one) => percentile(one.times, 0.95)),
    "ms",
    { under: Math.min(FRAME, SMOOTH * unit) },
  ],
  ["frame, longest", least((one) => Math.max(...one.times)), "ms", {}],
];
console.log(
  `A life of ${facts.length} facts and ${heard} acts. The best of ${ROUNDS} rounds of ${STEPS / fps} seconds that scroll and stream, at ${rounds.map((one) => one.rate.toFixed(1)).join(", ")} fps.`,
);
let over = false;
for (const [name, value, unit, { under, least }] of rows) {
  const fits = (under === undefined || value < under) && (least === undefined || value >= least);
  over ||= !fits;
  const budget =
    under !== undefined
      ? `   budget under ${under.toFixed(1)} ${unit}`
      : least !== undefined
        ? `   budget at least ${least.toFixed(1)} ${unit}`
        : "";
  console.log(
    `${name.padEnd(26)} ${value.toFixed(1).padStart(8)} ${unit}${budget}${budget ? (fits ? " ok" : " OVER") : ""}`,
  );
}
// A file that FURB_BENCH_JSON names takes one line of the measures of each run, from which the budgets are set again.
if (process.env.FURB_BENCH_JSON)
  appendFileSync(
    process.env.FURB_BENCH_JSON,
    `${JSON.stringify(Object.fromEntries(rows.map(([name, value]) => [name, Number(value.toFixed(2))])))}\n`,
  );
if (over) process.exitCode = 1;
