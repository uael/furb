/** bun run bench: how fast the TUI draws a long life, and how the cost of each new fact grows as the life grows. It
 * makes a recorded life from the facts of the life in tui/fixtures, again and again, each copy with names of its own,
 * at the weight of real records: whole words, turns and outputs, about 2.6 KB for each fact, as the record d3630cdb
 * holds them. It makes one life of about 160000 facts, the size of the record of the orchestrator of 10-22, and one of
 * a tenth of it, and opens each in the App on a test terminal.
 *
 * In each life it measures what one new fact costs, from the moment the view hears of it to the frame that shows it:
 * the read of the view, the draw of the App and the frame of the renderer, for a line of a command that runs, for a
 * new act, for a paragraph told on the root chain, which the turns of the chain read, and for a token of a model
 * that writes. It measures a fact of the long life and one of the short life in
 * turn, so that other work of the machine weighs on both alike, and takes for each kind the least of the medians of
 * three batches of 20 facts. In the long life it measures too the first frame, from the open of the life to its first
 * frame, once cold and twice warm, and the frames while the feed scrolls and a command streams a line of output each
 * frame at the target rate of the renderer, each round after a measure of the renderer with nothing to show, and how
 * much memory a process of its own holds once it opened the life. It takes the best of the warm first frames and the round that
 * draws the most of the frames of its empty renderer, since other work of the machine makes a measure worse and
 * never better. The pace is the time that the machine takes to parse the facts of the long life, with no code of
 * furb, which says how fast the machine runs. Each budget is set from these measures of the machine, so that it holds
 * on a fast machine and on a slow one, and on a machine whose other work loads each core three times.
 *
 * It fails when the warm first frame takes 32 paces or more, when the renderer draws fewer than 95 in each hundred of
 * the frames that it draws with nothing to show, when the 95th percentile of the frames takes a pace or more, or
 * longer than a frame at the target rate, or when a new fact of a kind costs in the long life more than twice what it
 * costs in the short life, and more than 2 ms. FURB_BENCH_JSON names a file that takes one line of the measures of each
 * run, and FURB_BENCH_FACTS sets about how many facts the long life holds. */
import {
  appendFileSync,
  closeSync,
  openSync,
  readFileSync,
  readSync,
  statSync,
  writeFileSync,
  writeSync,
} from "node:fs";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { createTestRenderer } from "@opentui/core/testing";
import { App } from "../src/app.ts";
import type { HostState } from "../src/bridge.ts";
import { demoLibrary, demoSession } from "../src/demo.ts";
import { Preferences } from "../src/preferences.ts";
import { fps } from "../src/theme.ts";

/** About how many facts the long life holds, as many copies of the fixture as make them, and the copies of the short
 * life, a tenth of it. */
const FACTS = Number(process.env.FURB_BENCH_FACTS) || 160000;
const COPY = readFileSync(resolve(import.meta.dir, "..", "fixtures", "conductor.jsonl"), "utf8").split(
  "\n[",
).length;
const LARGE = Math.max(1, Math.round(FACTS / COPY));
const SMALL = Math.max(1, Math.round(LARGE / 10));
/** The budgets of the warm first frame and of the 95th percentile of the frames, in units of the pace of the
 * machine. On 120 runs of the bench on ubuntu-latest, the warm first frame took 17 to 26 paces, and the 95th
 * percentile 0.43 to 0.81 of a pace, against 0.89 to 1.38 before the cards of the feed were kept. */
const FIRST = 32;
const SMOOTH = 1;
/** How many times its cost in the short life a new fact may cost in the long life, and the least that it may cost
 * there in any case, below which a measure of a new fact is the noise of the machine. */
const GROWTH = 2;
const FLOOR = 2;
const FRAME = 1000 / fps;
/** The steps of a round, one for each frame at the target rate: three seconds. */
const STEPS = fps * 3;
const ROUNDS = 3;
/** The new facts of each kind in a batch that the bench measures in each life, and the batches. */
const NEW = 20;
const BATCHES = 3;
/** How many bytes of a record the pace reads at a time. */
const PART = 16 * 2 ** 20;

/** The size of the heavy text of each kind of fact, in bytes, as real records hold them: the word of a run, the
 * python of a turn of a model, the output of a command, a line of output, the message of a prompt and its value. */
const HEAVY = { run: 4600, reply: 14000, bash: 9000, out: 1200, prompt: 2400, value: 1800 };
/** The text that fills a heavy text to its size: lines as long as the lines of a real life. */
const FILLER = Array.from(
  { length: 2000 },
  (_, at) => `line ${at} of a word, a turn or an output of a real life, as long as a line of code\n`,
).join("");
const fill = (text: unknown, size: number) =>
  typeof text === "string" && text.length < size ? text + FILLER.slice(0, size - text.length) : text;

/** A fact of the fixture with its heavy text at its size. */
function weigh(fact: unknown[]): unknown[] {
  const [kind, id] = fact;
  const asked = String(id).replace(/\d+$/, "");
  const value = fact[3] as { stdout?: { content?: unknown } } | unknown[] | null;
  if (kind === "run") fact[5] = fill(fact[5], HEAVY.run);
  else if (kind === "out") fact[3] = fill(fact[3], HEAVY.out);
  else if (kind === "prompt") fact[5] = fill(fact[5], HEAVY.prompt);
  else if (kind === "done" && asked === "reply" && Array.isArray(value))
    value[1] = fill(value[1], HEAVY.reply);
  else if (kind === "done" && asked === "bash" && value && !Array.isArray(value) && value.stdout)
    value.stdout.content = fill(value.stdout.content, HEAVY.bash);
  else if (kind === "done" && asked === "prompt") fact[3] = fill(value, HEAVY.value);
  return fact;
}

/** A recorded life of a count of copies of the fixture in a file: the facts of the fixture again and again, each copy
 * with names of its own, on the one root chain, so that each copy adds its chains, its prompts and its commands to the
 * life, each fact at its weight. The fixture ends while work runs, and a long life ends its earlier work, so each copy
 * but the last ends with a cancel of each act that it left not done, and every life ends as the fixture does. It
 * gives how many facts the life holds, and the highest number of each kind of act, from which the bench names the
 * acts that it adds. */
function life(copies: number, path: string): { facts: number; top: Map<string, number> } {
  const lines = readFileSync(resolve(import.meta.dir, "..", "fixtures", "conductor.jsonl"), "utf8")
    .split("\n")
    .filter((line) => line.startsWith("["));
  const names = new Map<string, number>();
  for (const line of lines) {
    const found = /^([a-z_]+)(\d+)$/.exec(String((JSON.parse(line) as unknown[])[1]));
    if (found?.[1] && found[2]) names.set(found[1], Math.max(names.get(found[1]) ?? 0, Number(found[2])));
  }
  const top = new Map<string, number>();
  const file = openSync(path, "w");
  try {
    writeSync(file, `${JSON.stringify({ recorded: true })}\n`);
    let made = 0;
    for (let copy = 0; copy < copies; copy++) {
      const batch: string[] = [];
      const open = new Set<string>();
      for (const line of lines) {
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
        const found = /^([a-z_]+)(\d+)$/.exec(String(fact[1]));
        if (found?.[1] && found[2]) top.set(found[1], Math.max(top.get(found[1]) ?? 0, Number(found[2])));
        if (found?.[1] === fact[0]) open.add(String(fact[1]));
        if (fact[0] === "done") open.delete(String(fact[1]));
        batch.push(JSON.stringify(weigh(fact)));
        made++;
      }
      for (const name of copy < copies - 1 ? open : []) {
        batch.push(JSON.stringify(["done", name, "bench", { is: "CancelledError", args: [] }]));
        made++;
      }
      writeSync(file, `${batch.join("\n")}\n`);
    }
    return { facts: made, top };
  } finally {
    closeSync(file);
  }
}

/** The value at a share of the sorted values, as a percentile. */
function percentile(values: readonly number[], share: number): number {
  const sorted = [...values].sort((one, other) => one - other);
  return sorted[Math.min(sorted.length - 1, Math.floor(share * sorted.length))] ?? 0;
}

const root = await mkdtemp(join(tmpdir(), "furb-bench-"));
const preferences = new Preferences(join(root, "ui.json"));
const add = (record: string, ...added: unknown[][]) =>
  appendFileSync(record, added.map((fact) => `${JSON.stringify(fact)}\n`).join(""));

/** A life opened in an App on a test terminal, and the time from the open to its first frame. */
async function open(record: string) {
  const began = performance.now();
  const session = await demoSession({ record, cwd: root, preferences });
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
  return { record, session, app, screen, first, close };
}
type Opened = Awaited<ReturnType<typeof open>>;

/** What a new fact costs in a life, for each kind, one at a time: the time from the moment the view hears of the
 * fact to the frame that shows it, in which the view reads it, the App draws, and the renderer draws its frame. A line
 * of a command and a new act come in the file of the life, as the life says them, and a token of a model comes as the
 * state of the session that the worker sends, which holds what each model writes. */
async function facts({ record, session, app, screen }: Opened, top: Map<string, number>) {
  const named = (kind: string) => `${kind}${(top.get(kind) ?? 0) + 1}`;
  const [rung, bash] = [named("rung"), named("bash")];
  add(
    record,
    ["rung", rung, "operator", "chain1", "await bash('seq 1000')", "", ""],
    ["started", rung, "chain1"],
    ["bash", bash, rung, "chain1", "seq 1000", "", 600],
    ["started", bash, "bash"],
  );
  const changed = () => new Promise<void>((done) => session.once("change", () => done()));
  while (!session.actOf(bash)) await changed();
  /** One new fact, from the moment the view hears of it, until the frame that shows it. */
  const one = async (heard: () => void, seen: () => boolean) => {
    const told = new Promise<number>((done) => session.host.once("change", () => done(performance.now())));
    heard();
    const began = await told;
    while (!seen()) await changed();
    const read = performance.now() - began;
    const drawing = performance.now();
    app.render();
    const drawn = performance.now() - drawing;
    await screen.flush();
    // The frame of the renderer is its own work, and the test terminal waits for the next frame of its rate besides.
    return read + drawn + (screen.renderer.getStats().frameTimes.at(-1) ?? 0);
  };
  const output = () =>
    (session.actOf(bash)?.value as { stdout?: { content?: string } } | undefined)?.stdout?.content;
  const { host } = session;
  const state = (streams: HostState["streams"]): HostState => ({
    completed: host.completed,
    cost: host.cost,
    directory: host.directory,
    imageDirectory: host.imageDirectory,
    actor: host.actor,
    record: host.record,
    changes: host.changes,
    reading: host.reading,
    recorded: host.recorded,
    measures: {},
    seq: host.seq,
    facts: 0,
    prompts: [...host.prompts.values()],
    notices: [...host.notices.values()],
    streams,
    pending: [...host.pending],
  });
  let text = "";
  return {
    line: (at: number) => {
      const line = `line ${at} of a command that streams\n`;
      return one(
        () => add(record, ["out", bash, "bash", line, "stdout"]),
        () => Boolean(output()?.endsWith(line)),
      );
    },
    act: (at: number) => {
      const read = `${named("read")}${at}`;
      return one(
        () =>
          add(
            record,
            ["read", read, rung, "chain1", "README.md"],
            ["done", read, "files", { is: "Text", path: "README.md", content: "x\n" }],
          ),
        () => Boolean(session.actOf(read)?.done),
      );
    },
    tell: (at: number) => {
      const said = `#tell ${at} of the bench`;
      return one(
        () => add(record, ["tell", "chain1", "chain1", [said]]),
        () => Boolean(session.turns.at(-1)?.[1]?.endsWith(said)),
      );
    },
    token: (at: number) => {
      text += `token ${at} `;
      const said = text;
      return one(
        () => host.update(state([[rung, { chain: "chain1", text: said, thinking: "" }]])),
        () => host.streams.get(rung)?.text === said,
      );
    },
  };
}

/** One round: a command starts on the root chain under a rung of its own, and another process writes a line of its
 * output each frame, as a command writes it, so that no work of the bench takes time from the thread that draws.
 * The feed scrolls a third of its height each frame, up for the first half and down for the second. */
async function round({ record, app, screen }: Opened, top: Map<string, number>, at: number) {
  const named = (kind: string) => `${kind}${(top.get(kind) ?? 0) + 100 + at}`;
  const [rung, bash] = [named("rung"), named("bash")];
  add(
    record,
    ["rung", rung, "operator", "chain1", "await bash('seq 1000')", "", ""],
    ["started", rung, "chain1"],
    ["run", named("run"), "chain1", "chain1", rung, "await bash('seq 1000')", ""],
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
  add(record, [
    "done",
    bash,
    "bash",
    { is: "Exit", code: 0, stdout: { is: "Text", content: "" }, stderr: { is: "Text", content: "" } },
  ]);
  const times = screen.renderer.getStats().frameTimes.slice(skip);
  return { rate: (frames.length * 1000) / took, times };
}

/** The frames each second that the renderer draws at its target rate with nothing to show. start draws one frame
 * before it returns, outside the timed window, as the App does before a round. */
async function ceiling(): Promise<number> {
  const screen = await createTestRenderer({ width: 152, height: 46, exitSignals: [], targetFps: fps });
  screen.renderer.start();
  let frames = 0;
  screen.renderer.on("frame", () => frames++);
  const began = performance.now();
  await Bun.sleep(2000);
  const rate = (frames * 1000) / (performance.now() - began);
  screen.renderer.destroy();
  return rate;
}

/** The memory that a process holds once it opened a life, after a collection of its garbage: a process of its own,
 * which holds no garbage of the bench. */
async function held(record: string): Promise<number> {
  const probe = join(root, "held.ts");
  const from = (path: string) => JSON.stringify(resolve(import.meta.dir, "..", "src", path));
  writeFileSync(
    probe,
    `import { demoSession } from ${from("demo.ts")};
import { Preferences } from ${from("preferences.ts")};
await demoSession({ record: ${JSON.stringify(record)}, cwd: ${JSON.stringify(root)}, preferences: new Preferences(${JSON.stringify(join(root, "held.json"))}) });
Bun.gc(true);
console.log(process.memoryUsage().rss);
process.exit(0);`,
  );
  const child = Bun.spawn([process.execPath, probe], { stdout: "pipe", stderr: "inherit" });
  const said = await new Response(child.stdout).text();
  await child.exited;
  return Number(said.trim());
}

/** The time that this machine takes to parse the facts of a record, the best of three: work that no code of furb
 * does, which says how fast the machine runs, as the empty renderer says how fast it draws. */
function pace(record: string): number {
  // The file is read a part at a time, since a string holds less than a record of a long life.
  const parts: string[] = [];
  const file = openSync(record, "r");
  try {
    const part = Buffer.alloc(PART);
    let left = "";
    for (let got = readSync(file, part); got > 0; got = readSync(file, part)) {
      const text = left + part.toString("utf8", 0, got);
      const end = text.lastIndexOf("\n") + 1;
      parts.push(text.slice(0, end));
      left = text.slice(end);
    }
    if (left) parts.push(left);
  } finally {
    closeSync(file);
  }
  let least = Number.POSITIVE_INFINITY;
  for (let at = 0; at < 3; at++) {
    const began = performance.now();
    for (const text of parts) for (const line of text.split("\n")) if (line) JSON.parse(line);
    least = Math.min(least, performance.now() - began);
  }
  return least;
}

// Each measure that compares the long life with the short one, or the renderer with an empty one, takes its two
// sides in turn, so that other work of the machine weighs on both alike.
const short = join(root, "short.jsonl");
const shortLife = life(SMALL, short);
const record = join(root, "life.jsonl");
const { facts: made, top } = life(LARGE, record);
const bytes = statSync(record).size;
const unit = pace(record);
const cold = await open(record);
await cold.close();
const memory = await held(record);
const warm = await open(record);
const shortOpened = await open(short);
const kinds = ["line", "act", "tell", "token"] as const;
type Kind = (typeof kinds)[number];
const sides = [
  { measure: await facts(shortOpened, shortLife.top), costs: { line: [], act: [], tell: [], token: [] } },
  { measure: await facts(warm, top), costs: { line: [], act: [], tell: [], token: [] } },
] as { measure: Awaited<ReturnType<typeof facts>>; costs: Record<Kind, number[]> }[];
for (let at = 0; at < NEW * BATCHES; at++)
  for (const kind of kinds) for (const { measure, costs } of sides) costs[kind].push(await measure[kind](at));
/** The least of the medians of the batches of new facts of a kind. */
const calmest = (costs: number[]) =>
  Math.min(
    ...Array.from({ length: BATCHES }, (_, at) => percentile(costs.slice(at * NEW, (at + 1) * NEW), 0.5)),
  );
const median = ({ costs }: (typeof sides)[number]) => ({
  line: calmest(costs.line),
  act: calmest(costs.act),
  tell: calmest(costs.tell),
  token: calmest(costs.token),
});
const [before, after] = sides.map(median) as [Record<Kind, number>, Record<Kind, number>];
await shortOpened.close();
const rounds: (Awaited<ReturnType<typeof round>> & { empty: number })[] = [];
for (let at = 1; at <= ROUNDS; at++) {
  const empty = await ceiling();
  warm.screen.renderer.start();
  rounds.push({ ...(await round(warm, top, at)), empty });
  warm.screen.renderer.stop();
}
const heard = warm.session.acts.length;
await warm.close();
// A second warm open, since a first frame that other work of the machine slows is one sample of a slow machine.
const again = await open(record);
await again.close();
await rm(root, { recursive: true, force: true });

/** The best of the rounds for each measure: other work of the machine makes a measure worse, and never better. */
const least = (of: (one: (typeof rounds)[number]) => number) => Math.min(...rounds.map(of));
const first = Math.min(warm.first, again.first);
const grown = (kind: Kind) => Math.max(FLOOR, GROWTH * before[kind]);
/** The round that drew the most of the frames that an empty renderer drew just before it. */
const fullest = rounds.reduce((best, one) => (one.rate / one.empty > best.rate / best.empty ? one : best));
/** Each measure, its unit, and its budget: a value that it must stay under, or the least that it must reach. */
const rows: [string, number, string, { under?: number; least?: number }][] = [
  ["pace of the machine", unit, "ms", {}],
  ["first frame, cold", cold.first, "ms", {}],
  ["first frame, warm", first, "ms", { under: FIRST * unit }],
  ["memory of a life open", memory / 2 ** 20, "MB", {}],
  ["frames each second, empty", fullest.empty, "fps", {}],
  ["frames each second", fullest.rate, "fps", { least: fullest.empty * 0.95 }],
  ["frame, median", least((one) => percentile(one.times, 0.5)), "ms", {}],
  [
    "frame, 95th percentile",
    least((one) => percentile(one.times, 0.95)),
    "ms",
    { under: Math.min(FRAME, SMOOTH * unit) },
  ],
  ["frame, longest", least((one) => Math.max(...one.times)), "ms", {}],
  ["a line, short life", before.line, "ms", {}],
  ["a line, long life", after.line, "ms", { under: grown("line") }],
  ["a new act, short life", before.act, "ms", {}],
  ["a new act, long life", after.act, "ms", { under: grown("act") }],
  ["a tell, short life", before.tell, "ms", {}],
  ["a tell, long life", after.tell, "ms", { under: grown("tell") }],
  ["a token, short life", before.token, "ms", {}],
  ["a token, long life", after.token, "ms", { under: grown("token") }],
];
console.log(
  `A life of ${made} facts, ${(bytes / 2 ** 20).toFixed(0)} MB or ${(bytes / made / 1024).toFixed(1)} KB for each fact, and ${heard} acts, against a life of ${shortLife.facts} facts. The best of ${ROUNDS} rounds of ${STEPS / fps} seconds that scroll and stream, at ${rounds.map((one) => one.rate.toFixed(1)).join(", ")} fps, and the least median of ${BATCHES} batches of ${NEW} new facts of each kind.`,
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
