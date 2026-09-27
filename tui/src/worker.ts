// The worker loads no module that loads OpenTUI, whose native library belongs to the thread that draws.
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import type { Engine, SessionOptions, Turn } from "@furb/engine";
import { Act, actorParts, engineSource, model, Session } from "@furb/engine";
import type { HostState } from "./bridge.ts";
import { type EngineOptions, offered } from "./models.ts";
import { queueDispatches, queueHash } from "./queue.ts";
import type { FollowUp } from "./session.ts";
import { Snapshots } from "./snapshots.ts";

declare const self: Worker & { close(): void };
let session: Session | undefined;
let engine: Engine | undefined;
let snapshots: Snapshots | undefined;
let timer: ReturnType<typeof setTimeout> | undefined;
let sentFacts = 0;
const state = () => {
  const owner = session;
  if (!owner) return;
  const snapshot: HostState = {
    completed: owner.activity.completed,
    cost: owner.activity.cost,
    directory: owner.directory,
    imageDirectory: owner.imageDirectory,
    actor: owner.actor,
    record: owner.record,
    facts: owner.facts.slice(sentFacts),
    prompts: [...owner.console.prompts.values()].map(({ id, shape, message }) => ({ id, shape, message })),
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

/** The models of the demo, which it names in the catalog of the crate and asks none of. */
const demoRoster = ["claude-cli:sonnet", "claude-cli:opus", "claude-cli:haiku", "claude-cli:fable"];

/** A session whose models are the demo, which asks no model and answers each turn from the script above. */
function scriptedSession(options: SessionOptions): Session {
  const { record, cwd } = options;
  const directory = cwd ?? (record ? dirname(record) : undefined);
  if (!directory) throw new Error("A demo session needs a directory or a record.");
  return new Session({
    ...options,
    roster: options.roster ?? demoRoster,
    cwd: directory,
    record: record ?? join(directory, "demo.jsonl"),
    answer: async ({ messages }, write): Promise<Turn> => {
      const pause = (milliseconds: number) =>
        new Promise<void>((resolve) => setTimeout(resolve, milliseconds));
      const last = JSON.stringify(messages.at(-1));
      // Only the turn that asks for live progress is slow, and not every later turn of its chain: it thinks, then
      // writes its word a few words at a time, as a model streams it. FURB_DEMO_STREAM=1 makes every turn so, for a
      // recording of the demo.
      const slow = process.env.FURB_DEMO_STREAM === "1" || last.includes("show live progress");
      await pause(slow ? 300 : 180);
      const first = !messages.some((message) => (message as { role?: string }).role === "assistant");
      const [thinking, answer] = first
        ? ["I read the README and run the checks before I answer.", ""]
        : reply(last);
      // A message that asks for an answer in a fence gets one, as a model sometimes writes it, which the gate refuses,
      // and the model answers again once it reads the refusal.
      const fenced = !first && last.includes("an answer in a fence");
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
}

/** What a request of the session comes to. */
async function answer(data: { target: string; method: string; args: unknown[] }): Promise<unknown> {
  if (data.method === "open") {
    const { demo, ...options } = data.args[0] as EngineOptions;
    const opened = demo
      ? scriptedSession(options)
      : new Session({ ...options, roster: options.roster ?? offered() });
    session = opened;
    engine = opened.open();
    snapshots = new Snapshots(engine, opened);
    opened.on("fault", (error) =>
      self.postMessage({ fault: error instanceof Error ? error.message : String(error) }),
    );
    opened.on("change", () => {
      timer ??= setTimeout(() => {
        timer = undefined;
        state();
      }, 20);
    });
    return engine.root;
  }
  if (data.target === "library" && data.method === "queue") {
    if (!engine || !session) throw new Error("The session is not open.");
    const entry = data.args[0] as FollowUp;
    const prior = queueDispatches(session.entries).get(entry.id);
    if (prior) return prior;
    engine.say("queue", entry.chain, [
      "begin",
      entry.id,
      queueHash(entry.chain, entry.shape, entry.text, entry.actor),
    ]);
    const act = engine.prompt(entry.shape, { message: entry.text, on: entry.chain, to: entry.actor });
    engine.say("queue", entry.chain, ["sent", entry.id, act.id]);
    return act.id;
  }
  if (data.target === "library" && (data.method === "model" || data.method === "sees")) {
    if (!engine) throw new Error("The session is not open.");
    const [roster] = engine.standing() as [[string][]];
    const names = roster.map(([name]) => name);
    // A name of the roster is found by the rule the catalog finds every model by, and an actor by its model.
    const name =
      data.method === "sees" ? actorParts(String(data.args[0]), names).model : String(data.args[0]);
    const found = model(name);
    if (data.method === "sees") return found?.images ?? false;
    const held = found?.name ?? name;
    return names.includes(held) ? held : null;
  }
  if (data.target === "library" && data.method === "source") return engineSource();
  if (data.target === "library" && data.method === "changes")
    return session?.changes.read(Number(data.args[0]), Number(data.args[1]));
  if (data.target === "library" && data.method === "act")
    return session?.activity.acts.get(String(data.args[0]));
  if (data.target === "library" && data.method === "look") {
    if (!engine || !session) throw new Error("The session is not open.");
    // The host reads the path where the chain stands, as the files would, and no act is made, so nothing is kept.
    const path = resolve(session.directory, engine.cwd({ on: String(data.args[1]) }), String(data.args[0]));
    try {
      return { path, content: readFileSync(path, "utf8") };
    } catch {
      throw new Error(`There is no file at ${path}.`);
    }
  }
  if (data.target === "library" && data.method === "snapshot") {
    if (!snapshots) throw new Error("The session is not open.");
    return snapshots.take(String(data.args[0]), Number(data.args[1]));
  }
  const target = data.target === "session" ? session : data.target === "console" ? session?.console : engine;
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
  if (data.target !== "session" || data.method !== "dispose") {
    self.postMessage({ id: data.id, ...reply });
    return;
  }
  // A session whose save failed has ended its life, its commands and its lease all the same, so the worker ends too.
  if (timer) clearTimeout(timer);
  session = undefined;
  engine = undefined;
  snapshots = undefined;
  setTimeout(() => {
    self.postMessage({ id: data.id, ...reply, disposed: true });
    self.close();
  }, 0);
};
