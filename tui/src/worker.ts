// The worker loads no module that loads OpenTUI, whose native library belongs to the thread that draws.
import { type FSWatcher, readFileSync, watch as watchFile } from "node:fs";
import { dirname, join, resolve } from "node:path";
import type { Engine, FileChange, SessionOptions, Turn } from "furb";
import { Act, actorParts, ENGINE, model, models, retold, Session, setup } from "furb";
import { type Compaction, type Measure, measureOf } from "furb/life";
import type { HostState } from "./bridge.ts";
import { type EngineOptions, roster } from "./models.ts";
import { queueDispatches, queueHash } from "./queue.ts";
import { isRecorded, Reader } from "./reading.ts";
import { branchChanges, net } from "./review.ts";
import type { FollowUp } from "./session.ts";
import { Snapshots } from "./snapshots.ts";

declare const self: Worker & { close(): void };

/** Each change of a journal that one of the writes made, or each change when no list of writes is given, read a page
 * at a time. */
function* written(
  changes: { length: number; read(start: number, count: number): FileChange[] },
  writes: unknown,
): Generator<FileChange> {
  const named = Array.isArray(writes) ? new Set(writes.map(String)) : undefined;
  for (let start = 0; start < changes.length; start += 50)
    for (const change of changes.read(start, 50))
      if (!named || (change.act && named.has(change.act))) yield change;
}
let session: Session | undefined;
let engine: Engine | undefined;
let snapshots: Snapshots | undefined;
/** The record open to read, when the life on it was refused for a drift: one that another engine made. */
let reader: Reader | undefined;
/** The claude command line that the session names, which the catalog offers, and whether the session is the demo. */
let claude: string | undefined;
let demo = false;
let timer: ReturnType<typeof setTimeout> | undefined;
/** The watch on the file of a recorded life, which hears each line that the file adds. */
let follower: FSWatcher | undefined;
let sentFacts = 0;
const state = () => {
  if (reader) {
    self.postMessage({
      state: {
        completed: reader.activity.completed,
        cost: reader.activity.cost,
        directory: reader.directory,
        imageDirectory: reader.imageDirectory,
        actor: reader.actor,
        record: reader.record,
        facts: reader.facts.length,
        seq: reader.facts.length,
        prompts: [],
        notices: [],
        streams: [],
        pending: [],
        changes: 0,
        reading: reader.reading,
        recorded: reader.recorded,
        measures: measuresOf(reader.activity.answers),
        compactions: compactionsOf(reader.activity.compactions),
      } satisfies HostState,
    });
    return;
  }
  const owner = session;
  if (!owner) return;
  const snapshot: HostState = {
    completed: owner.activity.completed,
    cost: owner.activity.cost,
    directory: owner.directory,
    imageDirectory: owner.imageDirectory,
    actor: owner.actor,
    record: owner.record,
    facts: owner.facts.length - sentFacts,
    seq: owner.facts.length,
    prompts: [...owner.console.prompts.values()].map(({ id, shape, message }) => ({ id, shape, message })),
    notices: [...owner.console.notices.values()],
    streams: [...owner.streams],
    pending: [...owner.pending],
    changes: owner.changes.length,
    measures: measuresOf(owner.activity.answers),
    compactions: compactionsOf(owner.activity.compactions),
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
/** What each model of the demo answers when a word asks every model what it would change first, by its name. */
const views: [model: string, answer: string][] = [
  ["haiku", "Add a test for a search that finds nothing."],
  ["sonnet", "Save the notes to a file, so they stay after a restart."],
  ["fable", "Give search a shortcut, Ctrl+K, so it is one key away."],
];
function reply(turn: string, actor: string, chain: string): [thinking: string, answer: string] {
  // Three rungs of one chain answer the word, so each reads the question in the turns of its chain.
  const view = chain.includes("change first") && views.find(([name]) => actor.includes(name));
  if (view) return ["One change comes first, so the answer names it.", view[1]];
  const found = replies.find(([cue]) => turn.includes(cue));
  return found
    ? [found[1], found[2]]
    : [
        "The change is small, so the answer says where it is.",
        "Done. The change is small, its checks pass, and the result is in the feed.",
      ];
}

/** The turns that think until a cancel. Each stays here, since a promise that nothing holds can be collected, and its
 * turn then fails. */
const held = new Set<(turn: Turn) => void>();

/** A session whose models are the demo, which asks no model and answers each turn from the script above. It enables
 * no extension, so the demo shows the same on every machine. A session that the operator watches streams each turn,
 * as a model writes it, and each of its answers costs nothing, since no model is asked. */
function scriptedSession(options: SessionOptions, watch = false): Session {
  const { record, cwd } = options;
  const directory = cwd ?? (record ? dirname(record) : undefined);
  if (!directory) throw new Error("A demo session needs a directory or a record.");
  return new Session({
    ...options,
    extensions: false,
    roster: roster({ ...options, demo: true }),
    cwd: directory,
    record: record ?? join(directory, "demo.jsonl"),
    answer: async ({ actor, messages }, write): Promise<Turn> => {
      const pause = (milliseconds: number) =>
        new Promise<void>((resolve) => setTimeout(resolve, milliseconds));
      const last = JSON.stringify(messages.at(-1));
      // A turn whose last message asks to think until a cancel never answers, so its work stays in progress until the
      // operator cancels it, however slow the machine that shows it.
      const message = [...last.matchAll(/prompt\d+_message = '([^']*)'/g)].at(-1)?.[1];
      if (message === "Think until I cancel.") return await new Promise<Turn>((resolve) => held.add(resolve));
      // Only the turn that asks for live progress is slow, and not every later turn of its chain: it thinks, then
      // writes its word a few words at a time, as a model streams it. FURB_DEMO_STREAM=1 makes every turn so, for a
      // recording of the demo.
      const slow = watch || process.env.FURB_DEMO_STREAM === "1" || last.includes("show live progress");
      await pause(slow ? 300 : 180);
      const first = !messages.some((message) => (message as { role?: string }).role === "assistant");
      const [thinking, answer] = first
        ? ["I read the README and run the checks before I answer.", ""]
        : reply(last, actor, JSON.stringify(messages));
      // A message that asks for an answer in a fence gets one, as a model sometimes writes it, which the gate refuses,
      // and the model answers again once it reads the refusal.
      const fenced = !first && last.includes("an answer in a fence");
      // A message that says to ask first gets a question to the operator and no close, so the model answers again
      // once the operator has answered.
      const asks = !first && last.includes("Ask me first");
      // A message that hires workers makes a life as a conductor makes it: a prompt to a worker on each of two chains of
      // its own, each of which thinks until a cancel, under a wait that holds the thread of the message open.
      const hires = last.includes("Hire two workers");
      // A message that adds the search shortcut writes the README, so a rewind to a point before it can give the file
      // back as it was.
      const shortcut = !first && last.includes("Add the search shortcut");
      const code = hires
        ? 'for label in ("w/one", "w/two"):\n  prompt(str, "Think until I cancel.", on=chain(label))\nawait wait(3600)'
        : shortcut
          ? 'notes = read("README.md")\nwrite(notes.replace("- Find it when it matters", "- Find it when it matters, with Ctrl+K"))\nclose("Ctrl+K opens the search now, and the README says so.")'
          : fenced
            ? '```python\nclose("Here is the answer, in a fence.")\n```'
            : asks
              ? 'await prompt(str, "Which effort should the next step take?", to=OPERATOR)'
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
      return ["assistant", code, [3240, 184, 2800, 0, watch ? 0 : 0.0024], null];
    },
  });
}

/** How many answers the measure of each chain that the host holds counts. */
const measured = new Map<string, number>();
/** The measure of each chain whose models answered since the last state, from the answers of the activity of a life,
 * which the host adds to the measures that it holds. */
function measuresOf(answers: ReadonlyMap<string, readonly { actor: string; usage: readonly number[] }[]>) {
  const changed: Record<string, Measure> = {};
  for (const [chain, list] of answers)
    if (measured.get(chain) !== list.length) {
      measured.set(chain, list.length);
      changed[chain] = measureOf(list);
    }
  return changed;
}

/** How many compactions of the life the host holds. */
let compacted = 0;
/** The compactions of a life since the last state, which the host adds to those that it holds. */
function compactionsOf(compactions: readonly Compaction[]): Compaction[] {
  const fresh = compactions.slice(compacted);
  compacted = compactions.length;
  return fresh;
}

/** What a request of the session comes to. */
async function answer(data: { target: string; method: string; args: unknown[] }): Promise<unknown> {
  if (data.method === "open") {
    const { demo: scripted, watch, ...options } = data.args[0] as EngineOptions;
    // A recorded life is a log of a life that a host reads and runs not, at the moment where it ends.
    if (options.record && isRecorded(options.record)) {
      const recorded = Reader.recorded(options.record, options.cwd ?? dirname(options.record));
      reader = recorded;
      demo = true;
      snapshots = new Snapshots(recorded, recorded);
      // A recorded life follows its file: each fact that a later line adds is heard as the life said it, and the view
      // hears of it as it hears of a life that runs.
      follower = watchFile(options.record, () => {
        if (recorded.follow())
          timer ??= setTimeout(() => {
            timer = undefined;
            state();
          }, 20);
      });
      return recorded.root;
    }
    claude = options.claude;
    demo = (scripted || watch) ?? false;
    const opened = demo
      ? scriptedSession(options, watch)
      : new Session({ ...options, roster: roster(options) });
    try {
      engine = opened.open();
    } catch (error) {
      // A record that a build of another version made drifts on a replay, so it opens to read, with the facts that
      // it kept.
      const why = error instanceof Error ? error.message : String(error);
      if (!why.startsWith("Drift") || !opened.record) throw error;
      const { facts, made, engine: ours, version, current, logged } = retold(opened.record);
      reader = new Reader(facts, opened.directory, opened.record, {
        made,
        engine: ours,
        version,
        current,
        why,
        logged,
      });
      snapshots = new Snapshots(reader, reader);
      return reader.root;
    }
    session = opened;
    snapshots = new Snapshots(engine, opened);
    opened.on("fault", (error) =>
      self.postMessage({ fault: error instanceof Error ? error.message : String(error) }),
    );
    opened.on("log", (line: string) => self.postMessage({ log: line }));
    opened.on("change", () => {
      timer ??= setTimeout(() => {
        timer = undefined;
        state();
      }, 20);
    });
    return engine.root;
  }
  if (reader) return read(data);
  if (data.target === "library" && data.method === "queue") {
    if (!engine || !session) throw new Error("The session is not open.");
    const entry = data.args[0] as FollowUp;
    const prior = queueDispatches(session.entries).get(entry.id);
    if (prior) return prior;
    engine.say("queue", entry.chain, [
      "begin",
      entry.id,
      queueHash(entry.chain, entry.shape, entry.text, ""),
    ]);
    const act = engine.prompt(entry.shape, { message: entry.text, on: entry.chain });
    engine.say("queue", entry.chain, ["sent", entry.id, act.id]);
    return act.id;
  }
  // The demo asks no model, so it offers its roster and nothing of the catalog of this machine.
  if (data.target === "library" && data.method === "catalog")
    return demo ? [] : models(claude).map(({ name, efforts, window }) => [name, efforts, window]);
  // The first run checks this machine, which the demo does not need.
  if (data.target === "library" && data.method === "setup") return demo ? null : setup(claude);
  if (data.target === "library" && (data.method === "model" || data.method === "sees")) {
    if (!engine) throw new Error("The session is not open.");
    const [standing] = engine.standing() as [[string][]];
    const names = standing.map(([name]) => name);
    // A name of the roster is found by the rule the catalog finds every model by, and an actor by its model.
    const name =
      data.method === "sees" ? actorParts(String(data.args[0]), names).model : String(data.args[0]);
    const found = model(name);
    if (data.method === "sees") return found?.images ?? false;
    const held = found?.name ?? name;
    // A model that the roster does not hold is one the operator may add from the catalog, which offers it.
    const offered = names.includes(held) || (!demo && models(claude).some((one) => one.name === held));
    return offered ? held : null;
  }
  if (data.target === "library" && data.method === "source") return ENGINE;
  // The engine that made the record, as its fact made names it, which the host of the life says.
  if (data.target === "library" && data.method === "made")
    return session?.entries.map(([fact]) => fact).find((fact) => fact?.[0] === "made")?.[3] ?? null;
  if (data.target === "library" && data.method === "changes")
    return session?.changes.read(Number(data.args[0]), Number(data.args[1]));
  if (data.target === "library" && data.method === "net")
    return session ? net(written(session.changes, data.args[0])) : [];
  if (data.target === "library" && data.method === "branch")
    return session ? branchChanges(session.directory) : [];
  if (data.target === "library" && data.method === "repairChanges") return session?.changes.repair();
  if (data.target === "library" && data.method === "renewChanges") return session?.changes.renew();
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
    return snapshots.take(String(data.args[0]), Number(data.args[1]), Number(data.args[2] ?? 0));
  }
  const target = data.target === "session" ? session : data.target === "console" ? session?.console : engine;
  if (!target) throw new Error("The session is not open.");
  const method = Reflect.get(target, data.method) as (...args: unknown[]) => unknown;
  if (typeof method !== "function") throw new Error(`Unknown ${data.target} method ${data.method}.`);
  const result = Reflect.apply(method, target, data.args);
  return result instanceof Act ? result.id : await result;
}

/** What a request comes to on a record open to read: a view of what it kept, or a refusal of what would change it. */
function read(data: { target: string; method: string; args: unknown[] }): unknown {
  const open = reader as Reader;
  const views = ["root", "standing", "program", "turns", "cwd", "inspect", "outcome", "transcript"];
  if (data.target === "library" && data.method === "snapshot")
    return snapshots?.take(String(data.args[0]), Number(data.args[1]), Number(data.args[2] ?? 0));
  if (data.target === "library" && data.method === "source") return ENGINE;
  if (data.target === "library" && data.method === "made") return open.reading?.made ?? null;
  if (data.target === "library" && data.method === "catalog") return [];
  if (data.target === "library" && data.method === "setup") return null;
  if (data.target === "library" && data.method === "sees") return false;
  if (data.target === "library" && data.method === "model") return null;
  if (data.target === "library" && data.method === "changes") return [];
  if (data.target === "library" && data.method === "net") return [];
  if (data.target === "library" && data.method === "branch") return branchChanges(open.directory);
  if (data.target === "library" && data.method === "act") return open.activity.acts.get(String(data.args[0]));
  if (data.target === "library" && data.method === "look") {
    const path = resolve(open.directory, open.cwd({ on: String(data.args[1]) }), String(data.args[0]));
    try {
      return { path, content: readFileSync(path, "utf8") };
    } catch {
      throw new Error(`There is no file at ${path}.`);
    }
  }
  if (data.target === "session" && data.method === "dispose") return;
  if (data.target === "engine" && views.includes(data.method)) {
    const method = Reflect.get(open, data.method) as (...args: unknown[]) => unknown;
    return Reflect.apply(method, open, data.args);
  }
  return open.refuse();
}

self.onmessage = async ({ data }) => {
  // The end of the life is the dispose of its session: a window of its own ends it so, and a window of a host ends it
  // by end, since a dispose there only detaches the window.
  if (data.target === "session" && data.method === "end") data.method = "dispose";
  let reply: { value?: unknown; error?: string; name?: string };
  try {
    reply = { value: await answer(data) };
  } catch (error) {
    // The name of an error crosses with its message, so the view knows a damaged journal from another failure.
    reply = error instanceof Error ? { error: error.message, name: error.name } : { error: String(error) };
  }
  if (data.method === "open" && !reply.error) state();
  if (data.target !== "session" || data.method !== "dispose") {
    self.postMessage({ id: data.id, ...reply });
    return;
  }
  // A session whose save failed has ended its life, its commands and its lease all the same, so the worker ends too.
  if (timer) clearTimeout(timer);
  follower?.close();
  follower = undefined;
  session = undefined;
  engine = undefined;
  snapshots = undefined;
  reader = undefined;
  setTimeout(() => {
    self.postMessage({ id: data.id, ...reply, disposed: true });
    self.close();
  }, 0);
};
