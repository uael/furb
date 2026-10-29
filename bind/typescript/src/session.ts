import { EventEmitter } from "node:events";
import { existsSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { attachImage, Engine, type ImageAttachment, model, type NativeEar, type Opening } from "../index.cjs";
import { Activity } from "./activity.js";
import { FileChanges } from "./changes.js";
import { Console, type ConsoleOptions } from "./console.js";
import { call, type Ear } from "./ears.js";
import { type PauseCause, WORK } from "./life/acts.js";
import { furbDirectory, saveFile } from "./project.js";
import { actorParts, type Entry, type Fact, isQuestion, standingOf } from "./types.js";

/** An ear that takes nothing and says nothing. */
function* silent(): Ear {
  for (;;) yield null;
}

/** A function that answers each request of the provider in place of a model: it is given the request, the actor,
 * the chain, the messages and the settings of the request, its effort, its conversation and the most tokens of thought
 * of a claude model among them, and a function that tells what it writes as it writes it,
 * and gives the turn of the model. */
export type Answer = NonNullable<Opening["answer"]>;
/** What a model writes while it answers a rung, on the chain of the rung. */
/** A retry of the call of a model, which the provider waits for: the attempt that comes next, the most attempts, when
 * the wait ends, in milliseconds since the epoch, and why the attempt before it failed. */
export type Retry = { attempt: number; most: number; until: number; why: string };

/** What a model wrote for a rung while its reply runs, its chain, and the retry that the call of its model waits
 * for, until the model writes again. */
export type Stream = { chain: string; text: string; thinking: string; retry?: Retry };

export interface SessionOptions {
  /** Replay a record for inspection without owning it or starting outside work. */
  readOnly?: boolean;
  cwd?: string;
  record?: string;
  /** The model a prompt goes to when it names none, as the catalog of the crate names it; the first of the roster
   * when unsaid, and the default of the crate when the roster is unsaid too. */
  model?: string;
  /** The effort of that model, which moves to the nearest one the model takes; the effort of the crate when unsaid. */
  effort?: string;
  /** The models the session offers beside that one, as the catalog names them. When it is unsaid, the session offers
   * every model that the catalog offers, after that one; a roster that names none offers that model alone, and, with
   * no model, the operator alone. */
  roster?: string[];
  /** The claude command line to run, in place of the one that `FURB_CLAUDE_BIN` names or this machine holds. */
  claude?: string;
  /** Replace only the model request, for a deterministic test or another host; the host still names the models. */
  answer?: Answer;
  operator?: ConsoleOptions["operator"];
  /** Ears of the host's own, which come before the ears of the session in the order the engine offers a question,
   * so each takes a question in their place, or wraps it. */
  ears?: Array<[string, Ear | NativeEar]>;
  /** Whether the life enables at its start the extensions that the configs of the user and of the directory turn
   * on; true when unsaid. A life runs what its record enables either way. */
  extensions?: boolean;
  /** The config directory of the user, in place of the one of this process. */
  config?: string;
}

/** What the session keeps beside its record for the next life. */
interface Saved {
  /** The directory, and the model and the effort the host chose last, which a later session takes when its host
   * names none. */
  options: Pick<SessionOptions, "cwd" | "model" | "effort">;
}

/** One session: an engine, and the ears it hears by. The World is ears: the provider of models, and the files, the
 * commands, time, the store of the record and the extensions, which the crate opens every life on, and the console,
 * which this package writes. The activity keeps every act for the host, and the changes keep every write. A fault
 * event tells what the life refused when the console said what an act came to, and that the life stopped, as an error
 * named Stopped, when a fault cut a run of its sandbox. */
export class Session extends EventEmitter {
  readonly directory: string;
  /** The path of the record, when the session keeps one. */
  readonly record?: string;
  /** What each rung that asked a model streams while the model writes, by that rung, while its reply runs. A stream
   * lives no longer than its reply, so the session keeps none for a later life. */
  readonly streams = new Map<string, Stream>();
  readonly console: Console;
  readonly activity = new Activity();
  readonly changes: FileChanges;
  readonly facts: Fact[] = [];
  /** The entries of the record: those the life opened on, and each that the life kept since. */
  entries: Entry[] = [];
  /** How many entries the record held when the life opened on it. */
  opened = 0;
  /** The engine, once the session opened it. */
  engine?: Engine;
  private readonly options: SessionOptions;
  /** The rung that each reply of the life that runs asks a model for, until the done of that reply. */
  readonly replies = new Map<string, string>();
  private stopped = false;
  /** The pending work as the engine last gave it, which the engine reads off the transcripts at each call, so the
   * session asks it again only after a fact that can change it. */
  private held?: Map<string, string>;
  /** How many of the facts the session has given its host. */
  private emitted = 0;
  private queued = false;

  constructor(options: SessionOptions = {}) {
    super();
    const companion = options.record ? `${resolve(options.record)}.session.json` : undefined;
    const saved =
      companion && existsSync(companion) ? (JSON.parse(readFileSync(companion, "utf8")) as Saved) : undefined;
    const named = options;
    if (saved)
      options = {
        ...saved.options,
        ...Object.fromEntries(Object.entries(options).filter(([, value]) => value !== undefined)),
      };
    this.directory = resolve(options.cwd ?? process.cwd());
    this.record = options.record ? resolve(options.record) : undefined;
    const changed = () => this.changed();
    const fault = (error: unknown) => this.emit("fault", error);
    // The model a saved record names is a preference of the host, which a later session takes only while the
    // catalog knows that model: the record keeps what it was lived on, and the stand of the later life tells each
    // chain what it stands on now.
    const preferred = saved?.options.model;
    const held = preferred && model(preferred, this.directory, options.config) ? preferred : undefined;
    this.options = { ...options, model: named.model ?? held ?? named.roster?.[0] };
    this.console = new Console({ operator: options.operator, changed, fault });
    this.changes = new FileChanges(this.record, options.readOnly);
  }

  /** The actor a prompt goes to when it names none, which the standing of every chain says, and the operator before
   * the life stands. */
  get actor(): string {
    return (this.engine && standingOf(this.engine)[2]) ?? "operator";
  }

  get imageDirectory(): string {
    return this.record ? `${this.record}.images` : join(this.directory, ".furb/images");
  }
  attachImage(path: string): ImageAttachment {
    if (this.options.readOnly) throw new Error("Record inspection cannot attach an image.");
    // With no record, the images stay in the .furb of the directory, which is made with its ignore rule.
    if (!this.record) furbDirectory(this.directory);
    return attachImage(this.imageDirectory, resolve(this.directory, path));
  }

  /** The engine, opened on the record, on the ears of the host, then on the ears of the session, and then on the
   * ears of the crate, the provider among them. An inspection keeps nothing, enables no extension, asks no model,
   * and hears no ear that does work. */
  open(): Engine {
    if (this.engine) throw new Error("This session already owns an engine.");
    // A later life hears under the names the life before it heard, since the record says who asked what. An
    // inspection hears each by an ear that does no work.
    const working = !this.options.readOnly;
    const ears: Array<[string, Ear | NativeEar]> = [
      ...(this.options.ears ?? []),
      ["observer", this.observer()],
      ["console", working ? this.console.ear() : silent()],
      ["changes", working ? this.changes.ear(this.directory, () => this.emit("change")) : silent()],
    ];
    try {
      // The crate refuses a life that drifted, since it keeps nothing more.
      const engine = Engine.open(ears, {
        directory: this.directory,
        record: this.record,
        inspecting: !working,
        extensions: this.options.extensions,
        config: this.options.config,
        actor: [this.options.model, this.options.effort].filter(Boolean).join("/") || undefined,
        roster: this.options.roster,
        claude: this.options.claude,
        images: this.imageDirectory,
        answer: this.options.answer,
        stream: (rung, chain, text, thinking) => this.wrote(rung, chain, text, thinking),
        retrying: (rung, chain, attempt, most, seconds, why) =>
          this.retried(rung, chain, { attempt, most, until: Date.now() + seconds * 1000, why }),
        log: (line) => this.emit("log", line),
      });
      this.engine = this.console.engine = engine;
      // A fault that cuts a run of the sandbox stops the life, and no fact tells it, since no ear hears a fact after it.
      engine.onStop((why) => {
        const stop = new Error(
          `The life stopped at ${why}. It hears no fact after it: open it again on its record.`,
        );
        stop.name = "Stopped";
        this.emit("fault", stop);
      });
      // The observer heard the entries that the life kept as it opened, after the record it opened on.
      this.entries = [...(engine.record as Entry[]), ...this.entries];
      this.opened = engine.record.length;
      this.save();
      return engine;
    } catch (error) {
      this.stopped = true;
      this.engine?.dispose();
      this.console.dispose();
      this.changes.dispose();
      throw error;
    }
  }

  /** The ear that keeps every fact for the host, and every act in the activity. It is an ear of the outside, so a
   * snapshot of the life holds it by its name. An ear of the outside hears no question that an ear before it took,
   * so the observer reads such an act by `get` before the first fact about it, as the log of a life holds it. */
  private *observer(): Ear {
    const known = new Set<string>();
    for (;;) {
      const heard = (yield null) as Fact | undefined;
      if (!heard || this.stopped) continue;
      const about = heard[1];
      const made =
        about && !known.has(about) && !isQuestion(heard[0], about)
          ? ((yield call("get", [about])) as Fact | null | undefined)
          : undefined;
      for (const fact of made ? [made, heard] : [heard]) {
        if (isQuestion(fact[0], fact[1])) known.add(fact[1]);
        yield* this.observed(fact);
      }
    }
  }

  /** One fact that the observer keeps, after the act it is about. */
  private *observed(fact: Fact): Ear {
    this.facts.push(fact);
    yield* this.activity.hear(fact);
    const [kind, id, by, ...words] = fact;
    // Only a pause, a wake, or a start or a done of an act of the pending work changes what is pending.
    if (
      kind === "pause" ||
      kind === "wake" ||
      ((kind === "started" || kind === "done") && this.held?.has(id))
    )
      this.held = undefined;
    if (kind === "reply" && isQuestion(kind, id)) this.replies.set(id, by);
    const rung = kind === "done" ? this.replies.get(id) : undefined;
    if (rung !== undefined) {
      this.replies.delete(id);
      if (this.streams.delete(rung)) this.changed();
    }
    if (kind === "keep") this.entries.push(words[0] as Entry);
    this.heard();
  }

  /** What a model wrote for a rung, added to what it streams while its reply runs. The door tells it on a later turn
   * of the loop of the host, so a part can come after the done of its reply, and then it streams nothing. */
  private wrote(rung: string, chain: string, text: string, thinking: string): void {
    if (this.stopped || ![...this.replies.values()].includes(rung)) return;
    const held = this.streams.get(rung) ?? { chain, text: "", thinking: "" };
    held.text += text;
    held.thinking += thinking;
    delete held.retry;
    this.streams.set(rung, held);
    this.changed();
  }

  /** A retry of the call of the model of a rung: what the model wrote before it is void, and the stream holds the
   * retry until the model writes again. */
  private retried(rung: string, chain: string, retry: Retry): void {
    if (this.stopped || ![...this.replies.values()].includes(rung)) return;
    this.streams.set(rung, { chain, text: "", thinking: "", retry });
    this.changed();
  }

  /** The facts the observer heard, given the host once the ear is done hearing, since the host may ask the life. */
  private heard(): void {
    if (this.queued) return;
    this.queued = true;
    queueMicrotask(() => {
      this.queued = false;
      if (this.stopped) return;
      const facts = this.facts.slice(this.emitted);
      this.emitted = this.facts.length;
      if (!facts.length) return;
      this.emit("facts", facts);
      this.emit("change");
    });
  }

  private changed(): void {
    if (!this.stopped) this.emit("change");
  }

  private save(): void {
    if (!this.record || this.options.readOnly) return;
    const chosen = this.actor === "operator" ? {} : actorParts(this.actor);
    const saved: Saved = { options: { cwd: this.directory, ...chosen } };
    saveFile(`${this.record}.session.json`, JSON.stringify(saved));
  }

  /** The work that an earlier life left, which waits for a wake that this life says, by the kind of each act, as the
   * engine of the crate finds it. */
  get pending(): Map<string, string> {
    if (!this.engine) return new Map();
    this.held ??= new Map(this.engine.pending());
    return new Map(this.held);
  }

  /** The work that a crash loop parked, by the kind of each act, as the engine of the crate finds it: a wake puts
   * none of it to the outside, so resume does not run it again. */
  get parked(): Map<string, string> {
    return new Map(this.engine?.parked() ?? []);
  }

  /** Run a parked act again: it is parked no more in this life, and a wake of it puts it to the outside. */
  retry(id: string): void {
    if (this.options.readOnly) throw new Error("Record inspection cannot resume work.");
    this.engine?.retry(id);
    this.emit("change");
  }

  /** Start the pending work: a wake of each chain that holds some, which the engine answers by starting each
   * command, wait and prompt to the operator of it again, and by asking for each pending rung. */
  async resume(): Promise<void> {
    if (this.options.readOnly) throw new Error("Record inspection cannot resume work.");
    const engine = this.engine;
    if (!engine) return;
    const chains = new Set([...this.pending.keys()].map((id) => this.activity.acts.get(id)?.on));
    for (const chain of chains) if (chain) engine.wake(chain);
    this.emit("change");
  }

  /** Save what the next life needs, then end the life whatever the save came to: its commands, its waits, its
   * requests and its prompts end with it, and the store lets its record go. */
  async dispose(): Promise<void> {
    if (this.stopped) return;
    this.stopped = true;
    try {
      this.save();
    } finally {
      this.engine?.dispose();
      this.console.dispose();
      this.removeAllListeners();
      this.changes.dispose();
    }
  }

  /** The work of a chain cancelled: each prompt, rung, command and wait on it that is not done, with everything each
   * made. A cancel of the chain itself would end every act on it, and an act that an extension started, such as the
   * watcher of the memory, is no work of the chain and runs on. */
  interrupt(chain: string): void {
    for (const act of this.activity.acts.values())
      if (act.on === chain && !act.done && WORK.includes(act.kind)) this.engine?.cancel(act.id);
  }

  isPaused(id: string): boolean {
    return this.activity.acts.get(id)?.paused ?? false;
  }

  /** The cause of the pause that stands over an act, as the fact pause gave it. */
  pauseOf(id: string): PauseCause | undefined {
    const act = this.activity.acts.get(id);
    return act?.paused ? act.pause : undefined;
  }
}

/** Read pending work through the real replay path, without taking a lock or writing the record. */
export async function inspectRecord(record: string): Promise<{ pending: [string, string][] }> {
  const session = new Session({ record, readOnly: true });
  try {
    session.open();
    await Promise.resolve();
    return { pending: [...session.pending] };
  } finally {
    await session.dispose();
  }
}

/** A session, opened. */
export function boot(options: SessionOptions = {}): Session & { engine: Engine } {
  const session = new Session(options);
  session.open();
  return session as Session & { engine: Engine };
}
