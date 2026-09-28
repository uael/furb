import { EventEmitter } from "node:events";
import { existsSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import {
  attachImage,
  Engine,
  type ImageAttachment,
  model,
  type NativeEar,
  type OpenOptions,
} from "../index.cjs";
import { Activity, WORK } from "./activity.js";
import { FileChanges } from "./changes.js";
import { Console, type ConsoleOptions } from "./console.js";
import { driving, type Ear } from "./ears.js";
import { furbDirectory, saveFile } from "./project.js";
import { actorParts, type Entry, type Fact, isQuestion } from "./types.js";

/** An ear that takes nothing and says nothing. */
function* silent(): Ear {
  for (;;) yield null;
}

/** A function that answers each request of the provider in place of a model: it is given the request, the actor,
 * the chain, the messages and the settings of the effort, and a function that tells what it writes as it writes it,
 * and gives the turn of the model. */
export type Answer = NonNullable<OpenOptions["answer"]>;
/** What a model writes while it answers a rung, on the chain of the rung. */
export type Stream = { chain: string; text: string; thinking: string };

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
  /** The models the session offers beside that one, as the catalog names them. When it is unsaid, the model stands
   * alone, or the first model the catalog offers when the model is unsaid too; a roster that names none, with no
   * model, offers the operator alone. */
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
  streams?: [string, Stream][];
}

/** One session: an engine, and the ears it hears by. The World is ears: the provider of models, and the files, the
 * commands, time, the store of the record and the extensions, which the crate opens every life on, and the console,
 * which this package writes. The activity keeps every act for the host, and the changes keep every write. A fault
 * event tells what the life refused when the console said what an act came to. */
export class Session extends EventEmitter {
  readonly directory: string;
  /** The path of the record, when the session keeps one. */
  readonly record?: string;
  /** What each rung that asked a model streams while the model writes, by that rung, until its reply is done. */
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
  /** The rung that each reply the life made asks a model for. */
  private readonly replies = new Map<string, string>();
  private stopped = false;
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
    const held = preferred && model(preferred) ? preferred : undefined;
    this.options = { ...options, model: named.model ?? held ?? named.roster?.[0] };
    for (const [id, stream] of saved?.streams ?? []) this.streams.set(id, stream);
    this.console = new Console({ operator: options.operator, changed, fault });
    this.changes = new FileChanges(this.record, options.readOnly);
  }

  /** The actor a prompt goes to when it names none, which the standing of every chain says, and the operator before
   * the life stands. */
  get actor(): string {
    const standing = this.engine?.standing() as [unknown, string, string] | [];
    return standing?.[2] ?? "operator";
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
      ["observer", driving(this.observer(), "activity")],
      ["console", working ? this.console.ear() : silent()],
      ["changes", working ? this.changes.ear(this.directory, () => this.emit("change")) : silent()],
    ];
    try {
      // The crate refuses a life that drifted, since it keeps nothing more.
      const engine = Engine.open(
        {
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
        },
        ears,
      );
      this.engine = this.console.engine = engine;
      // The observer heard the entries that the life kept as it opened, after the record it opened on.
      this.entries = [...(engine.record as Entry[]), ...this.entries];
      this.opened = engine.record.length;
      this.save();
      return engine;
    } catch (error) {
      this.stopped = true;
      this.engine?.dispose();
      this.changes.dispose();
      throw error;
    }
  }

  /** The ear that keeps every fact for the host, and every act in the activity, as an ear of the engine. */
  private *observer(): Ear {
    for (;;) {
      const fact = (yield null) as Fact | undefined;
      if (!fact || this.stopped) continue;
      this.facts.push(fact);
      yield* this.activity.hear(fact);
      const [kind, id, by, ...words] = fact;
      if (kind === "reply" && isQuestion(kind, id)) this.replies.set(id, by);
      if (kind === "done" && this.streams.delete(this.replies.get(id) ?? "")) this.changed();
      if (kind === "keep") this.entries.push(words[0] as Entry);
      this.heard();
    }
  }

  /** What a model wrote for a rung, added to what it streams. */
  private wrote(rung: string, chain: string, text: string, thinking: string): void {
    if (this.stopped) return;
    const held = this.streams.get(rung) ?? { chain, text: "", thinking: "" };
    held.text += text;
    held.thinking += thinking;
    this.streams.set(rung, held);
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
    const saved: Saved = { options: { cwd: this.directory, ...chosen }, streams: [...this.streams] };
    saveFile(`${this.record}.session.json`, JSON.stringify(saved));
  }

  /** The work that an earlier life left, which waits for a wake that this life says, by the kind of each act, as the
   * engine of the crate finds it. */
  get pending(): Map<string, string> {
    return new Map(this.engine?.pending());
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
