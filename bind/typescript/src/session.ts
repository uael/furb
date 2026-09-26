import { EventEmitter } from "node:events";
import { existsSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import type { Models, ModelThinkingLevel } from "@earendil-works/pi-ai";
import { builtinModels } from "@earendil-works/pi-ai/providers/all";
import { bash, Engine, files, kept, type NativeEar, store, time } from "../index.cjs";
import { Activity, type RunState } from "./activity.js";
import { FileChanges } from "./changes.js";
import { Console, type ConsoleOptions } from "./console.js";
import { driving, type Ear } from "./ears.js";
import { attachImage, type ImageAttachment, ImageCache } from "./images.js";
import { furbDirectory, saveFile } from "./project.js";
import { type Answer, Provider } from "./provider.js";
import { type Entry, type Fact, modelNamed } from "./types.js";

export type { FileChange } from "./changes.js";
export { display, opens, paragraphs, safeText, uncommented } from "./types.js";

/** An ear that takes nothing and says nothing. */
function* silent(): Ear {
  for (;;) yield null;
}

/** The kinds of act whose work the record may show begun and not done, which waits for a wake. */
const PENDING = ["prompt", "rung", "bash", "wait"];

export interface SessionOptions {
  /** Replay a record for inspection without owning it or starting outside work. */
  readOnly?: boolean;
  cwd?: string;
  record?: string;
  model?: string;
  effort?: ModelThinkingLevel;
  models?: Models;
  roster?: string[];
  /** Replace only the model request, for a deterministic test or another host; the host still names the models. */
  answer?: Answer;
  operator?: ConsoleOptions["operator"];
  /** Ears of the host's own, which come before the ears of the session in the order the engine offers a question,
   * so each takes a question in their place, or wraps it. */
  ears?: Array<[string, Ear | NativeEar]>;
}

/** What the session keeps beside its record for the next life. */
interface Saved {
  /** The directory, and the model and the effort the host chose last, which a later session takes when its host
   * names none. */
  options: Pick<SessionOptions, "cwd" | "model" | "effort">;
  streams?: [string, { chain: string; text: string; thinking: string }][];
}

/** One session: an engine, and the ears it hears by. The World is ears: files, commands, time and the store of the
 * record, which the crate writes, and the provider of models and the console, which this package writes. The
 * activity keeps every act for the host, and the changes keep every write. A fault event tells what the life refused
 * when the provider or the console said what an act came to. */
export class Session extends EventEmitter {
  readonly directory: string;
  /** The path of the record, when the session keeps one. */
  readonly record?: string;
  readonly provider: Provider;
  readonly console: Console;
  readonly activity = new Activity();
  readonly changes: FileChanges;
  readonly facts: Fact[] = [];
  /** The acts that the record showed begun and not done when the life opened, and that no pause holds, by their
   * kind: the engine starts none of them until a wake that this life says, which resume says. One that a pause of
   * the operator holds waits for the wake of the operator. */
  readonly pending = new Map<string, string>();
  /** The entries of the record: those the life opened on, and each that the life kept since. */
  entries: Entry[] = [];
  /** How many entries the record held when the life opened on it. */
  opened = 0;
  /** The engine, once the session opened it. */
  engine?: Engine;
  private readonly images = new ImageCache();
  private readonly options: SessionOptions;
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
    this.options = options;
    this.directory = resolve(options.cwd ?? process.cwd());
    this.record = options.record ? resolve(options.record) : undefined;
    const changed = () => this.changed();
    const fault = (error: unknown) => this.emit("fault", error);
    const models = options.models ?? builtinModels();
    // The model a saved record names is a preference of the host, which a later session takes only while it holds
    // that model: the record keeps what it was lived on, and the stand of the later life tells each chain what it
    // stands on now.
    const preferred = saved?.options.model;
    const held = preferred && modelNamed(models.getModels(), preferred) ? preferred : undefined;
    const model = named.model ?? held ?? named.roster?.[0];
    this.provider = new Provider({
      directory: this.directory,
      models,
      model,
      effort: options.effort,
      roster: named.roster,
      answer: options.answer,
      images: this.imaging(),
      readOnly: options.readOnly,
      changed,
      fault,
    });
    for (const [id, stream] of saved?.streams ?? []) this.provider.streams.set(id, stream);
    this.console = new Console({ operator: options.operator, changed, fault });
    this.changes = new FileChanges(this.record, options.readOnly);
  }

  private imaging() {
    return { directory: () => this.imageDirectory, cache: this.images };
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

  /** The engine, opened on the record and on the ears of the session, after the ears of the host. An inspection
   * opens on the record alone, and hears no ear that does work. */
  open(): Engine {
    if (this.engine) throw new Error("This session already owns an engine.");
    const stored = this.record && !this.options.readOnly ? store(this.record) : undefined;
    const record = stored?.record ?? (this.record && existsSync(this.record) ? kept(this.record) : []);
    this.entries = [...(record as Entry[])];
    this.opened = record.length;
    // A later life hears under the names the life before it heard, since the record says who asked what. An
    // inspection hears each by an ear that does no work.
    const working = !this.options.readOnly;
    const ears: Array<[string, Ear | NativeEar]> = [
      ...(this.options.ears ?? []),
      ["observer", driving(this.observer(), "activity")],
      ["provider", this.provider.ear()],
      ["console", working ? this.console.ear() : silent()],
      ["changes", working ? this.changes.ear(this.directory, () => this.emit("change")) : silent()],
      ["files", working ? files() : silent()],
      ["bash", working ? bash() : silent()],
      ["time", working ? time() : silent()],
      ["store", stored?.ear ?? silent()],
    ];
    try {
      const engine = Engine.boot(record, ears);
      this.engine = this.provider.engine = this.console.engine = engine;
      // A life that drifted keeps nothing more, so this session refuses to open on it.
      const raised = engine.raised;
      if (raised) throw new Error(`${raised.is}: ${raised.args.map(String).join(" ")}`);
      // The journal said the whole record again before boot returned, so every act that is not done now is one the
      // record showed begun and not done.
      for (const act of this.activity.acts.values())
        if (PENDING.includes(act.kind) && !act.done && !act.paused) this.pending.set(act.id, act.kind);
      this.save();
      return engine;
    } catch (error) {
      this.stopped = true;
      this.engine?.dispose();
      this.changes.dispose();
      this.provider.dispose();
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
      const [kind, id, , ...words] = fact;
      if (kind === "done") this.pending.delete(id);
      if (kind === "keep") this.entries.push(words[0] as Entry);
      this.heard();
    }
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
    const saved: Saved = {
      options: { cwd: this.directory, model: this.provider.model, effort: this.provider.effort },
      streams: [...this.provider.streams],
    };
    saveFile(`${this.record}.session.json`, JSON.stringify(saved));
  }

  /** Start the pending work: a wake of each chain that holds some, which the engine answers by starting each
   * command, wait and prompt to the operator of it again, and by asking for each pending rung. */
  async resume(): Promise<void> {
    if (this.options.readOnly) throw new Error("Record inspection cannot resume work.");
    const engine = this.engine;
    if (!engine) return;
    const chains = new Set([...this.pending.keys()].map((id) => this.activity.acts.get(id)?.on));
    this.pending.clear();
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
      this.provider.dispose();
      this.console.dispose();
      this.images.clear();
      this.removeAllListeners();
      this.changes.dispose();
    }
  }

  isPaused(id: string): boolean {
    return this.activity.acts.get(id)?.paused ?? false;
  }
  rungState(id: string): RunState {
    return this.activity.acts.get(id)?.run ?? { status: "running", reason: "" };
  }
}

/** Read pending work through the real replay path, without taking a lock or writing the record. */
export async function inspectRecord(
  record: string,
  models?: Models,
): Promise<{ pending: [string, string][] }> {
  const session = new Session({ record, models, readOnly: true });
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
