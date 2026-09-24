// The worker loads no module that loads OpenTUI, whose native library belongs to the thread that draws.
import { dirname, join } from "node:path";
import type { Life, Turn, WorldOptions } from "@furb/engine";
import { Act, engineSource, modelNamed, World } from "@furb/engine";
import type { WorldState } from "./bridge.ts";
import { type EngineOptions, hostModels } from "./models.ts";
import { queueDispatches, queueEvent, queueHash } from "./queue.ts";
import type { FollowUp } from "./session.ts";
import { Snapshots } from "./snapshots.ts";

declare const self: Worker & { close(): void };
let world: World | undefined;
let life: Life | undefined;
let snapshots: Snapshots | undefined;
let host: ReturnType<typeof hostModels> | undefined;
let timer: ReturnType<typeof setTimeout> | undefined;
let sentFacts = 0;
const state = () => {
  const owner = world;
  if (!owner) return;
  const snapshot: WorldState = {
    completed: owner.activity.completed,
    cost: owner.activity.cost,
    directory: owner.directory,
    imageDirectory: owner.imageDirectory,
    actor: owner.actor,
    effort: owner.effort,
    roster: owner.roster,
    path: owner.records.path,
    facts: owner.facts.slice(sentFacts),
    prompts: [...owner.prompts.values()].map(({ id, shape, message }) => ({ id, shape, message })),
    streams: [...owner.streams],
    pending: [...owner.pending],
    changes: owner.changes.length,
  };
  sentFacts = owner.facts.length;
  self.postMessage({ state: snapshot });
};
/** What the demo thinks and answers on a later turn, by what the turn says. */
const replies: [cue: string, thinking: string, answer: string][] = [
  [
    "refused",
    "The gate refused the fence, so the word is Python alone.",
    "Here it is again as plain Python, with no fence around it.",
  ],
  [
    "show live progress",
    "The index is ready, so the answer says what it holds.",
    "The index is built, and three notes are ready to search. Each word streamed into the feed as it ran.",
  ],
  [
    "keyboard navigation",
    "The keys work, so the answer lists them.",
    "Keyboard navigation works: Ctrl+K opens the search, and the arrows move between notes.",
  ],
  [
    "layout",
    "The image shows the welcome, so the answer reads its layout.",
    "The layout reads well. The logo leads, the starters sit under it, and the keys close the column.",
  ],
  [
    " done",
    "The command is done, so the answer says what it found.",
    "The checks finished and all three passed, so the project is ready for the search shortcut.",
  ],
];
function reply(turn: string): [thinking: string, answer: string] {
  const found = replies.find(([cue]) => turn.includes(cue));
  return found
    ? [found[1], found[2]]
    : [
        "The change is small, so the answer says where it is.",
        "Done. The change is small, its checks pass, and the result is in the feed.",
      ];
}

function createDemoWorld(options: WorldOptions): World {
  const { record, cwd } = options;
  const directory = cwd ?? (record ? dirname(record) : undefined);
  if (!directory) throw new Error("A demo World needs a directory or a record.");
  const world = new World({
    ...options,
    cwd: directory,
    record: record ?? join(directory, "demo.jsonl"),
    answer: async (_actor, _chain, turns, signal, write): Promise<Turn> => {
      const pause = (milliseconds: number) =>
        new Promise<void>((resolve, reject) => {
          const timer = setTimeout(resolve, milliseconds);
          signal.addEventListener(
            "abort",
            () => {
              clearTimeout(timer);
              reject(new Error("cancelled"));
            },
            { once: true },
          );
        });
      // Only the turn that asks for live progress is slow, and not every later turn of its chain: it thinks, then
      // writes its word a few words at a time, as a model streams it. FURB_DEMO_STREAM=1 makes every turn so, for a
      // recording of the demo.
      const slow =
        process.env.FURB_DEMO_STREAM === "1" || JSON.stringify(turns.at(-1)).includes("show live progress");
      await pause(slow ? 300 : 180);
      const first = turns.filter((turn) => turn[0] === "assistant").length === 0;
      const [thinking, answer] = first
        ? ["I read the README and run the checks before I answer.", ""]
        : reply(JSON.stringify(turns.at(-1)));
      // A message that asks for an answer in a fence gets one, as a model sometimes writes it, which the gate refuses,
      // and the model answers again once it reads the refusal.
      const fenced = !first && JSON.stringify(turns.at(-1)).includes("an answer in a fence");
      const code = fenced
        ? '```python\nclose("Here is the answer, in a fence.")\n```'
        : first
          ? 'notes = read("README.md")\ncheck = await bash("printf \'✓ capture\\n✓ search\\n✓ local storage\\n\'")\nclose("## A clear starting point\\nFieldnotes keeps ideas close. The project has three small parts: capture, search, and local storage.\\n\\nAll three checks passed. A useful next step is to add a **search shortcut**, then cover it with a focused test.")'
          : `close(${JSON.stringify(answer)})`;
      if (slow) {
        write({ thinking });
        await pause(200);
        // FURB_DEMO_PACE sets the time between two pieces, in milliseconds, which a capture of work in progress
        // lengthens.
        const pace = Number(process.env.FURB_DEMO_PACE) || 30;
        for (const piece of code.match(/.{1,10}/gs) ?? []) {
          write({ text: piece });
          await pause(pace);
        }
        await pause(200);
      }
      return ["assistant", code, [3240, 184, 2800, 0, 0.0024], null];
    },
  });
  return world;
}

/** What a request of the session comes to. */
async function answer(data: { target: string; method: string; args: unknown[] }): Promise<unknown> {
  if (data.method === "open") {
    const { demo, claude, ...options } = data.args[0] as EngineOptions;
    host = hostModels(claude);
    const given = { ...options, models: host.models, roster: options.roster ?? host.roster };
    world = demo ? createDemoWorld(given) : new World(given);
    life = world.open();
    snapshots = new Snapshots(life, world);
    const tail = world.records.entries.at(-1)?.[0];
    const event = tail && queueEvent(tail);
    if (tail && event?.step === "begin") life.send("queue", tail[1], ["aborted", event.key]);
    world.on("fault", (error) =>
      self.postMessage({ fault: error instanceof Error ? error.message : String(error) }),
    );
    world.on("change", () => {
      timer ??= setTimeout(() => {
        timer = undefined;
        state();
      }, 20);
    });
    return life.root;
  }
  if (data.target === "library" && data.method === "queue") {
    if (!life || !world) throw new Error("The session is not open.");
    const entry = data.args[0] as FollowUp;
    const prior = queueDispatches(world.records.entries).get(entry.id);
    if (prior) return prior;
    life.send("queue", entry.chain, [
      "begin",
      entry.id,
      queueHash(entry.chain, entry.shape, entry.text, entry.actor),
    ]);
    const act = life.prompt(entry.shape, entry.text, { on: entry.chain, to: entry.actor });
    life.send("queue", entry.chain, ["sent", entry.id, act.id]);
    return act.id;
  }
  if (data.target === "library" && data.method === "model") {
    const owner = world;
    if (!owner) throw new Error("The session is not open.");
    // A roster name is found by the rule the World finds every model by.
    const offered = owner.roster.flatMap((name) => {
      const model = owner.offers(name);
      return model ? [{ name, provider: model.provider, id: model.id }] : [];
    });
    return modelNamed(offered, String(data.args[0]))?.name ?? null;
  }
  if (data.target === "library" && data.method === "source") return engineSource();
  if (data.target === "library" && data.method === "changes")
    return world?.changes.read(Number(data.args[0]), Number(data.args[1]));
  if (data.target === "library" && data.method === "act")
    return world?.activity.acts.get(String(data.args[0]));
  if (data.target === "library" && data.method === "snapshot") {
    if (!snapshots) throw new Error("The session is not open.");
    return snapshots.take(String(data.args[0]), Number(data.args[1]));
  }
  const target = data.target === "world" ? world : life;
  if (!target) throw new Error("The session is not open.");
  const method = Reflect.get(target, data.method) as (...args: unknown[]) => unknown;
  if (typeof method !== "function") throw new Error(`Unknown ${data.target} method ${data.method}.`);
  const result = Reflect.apply(method, target, data.args);
  return result instanceof Act ? result.id : await result;
}

self.onmessage = async ({ data }) => {
  let reply: { value?: unknown; error?: string };
  try {
    reply = { value: await answer(data) };
  } catch (error) {
    reply = { error: error instanceof Error ? error.message : String(error) };
  }
  if (data.method === "open" && !reply.error) state();
  if (data.target !== "world" || data.method !== "dispose") {
    self.postMessage({ id: data.id, ...reply });
    return;
  }
  // A World whose save failed has ended its life, its commands and its lease all the same, so the worker ends too.
  if (timer) clearTimeout(timer);
  world = undefined;
  life = undefined;
  snapshots = undefined;
  host?.dispose();
  host = undefined;
  setTimeout(() => {
    self.postMessage({ id: data.id, ...reply, disposed: true });
    self.close();
  }, 0);
};
