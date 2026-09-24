import { randomUUID } from "node:crypto";
import { EventEmitter } from "node:events";
import { existsSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import {
  type Api,
  type AssistantMessage,
  clampThinkingLevel,
  getSupportedThinkingLevels,
  type Message,
  type Model,
  type Models,
  type ModelThinkingLevel,
  type ThinkingLevel,
} from "@earendil-works/pi-ai";
import { builtinModels } from "@earendil-works/pi-ai/providers/all";
import { builtinExtensions, decodeRecord, type Extension, type Life, resolveExtensions } from "../index.cjs";
import { Activity, type RunState } from "./activity.js";
import { FileChanges } from "./changes.js";
import { type Ears, WorldAdapter, type WorldHandler, type WorldRequest, worldContext } from "./ears.js";
import type { WorldExtension, WorldPart } from "./extension.js";
import { builtinWorldParts, loadWorldParts } from "./extensions.js";
import { attachImage, type ImageAttachment, ImageCache, turnImages } from "./images.js";
import { furbDirectory, saveFile } from "./project.js";
import { RecordFile } from "./record.js";
import { at } from "./shell.js";
import {
  actorParts,
  type Entry,
  type Fact,
  marked,
  modelNamed,
  type OperatorPrompt,
  shapes,
  type Turn,
  zeroUsage,
} from "./types.js";

export { display, opens, paragraphs, safeText, uncommented } from "./types.js";

let prompt: string | undefined;
/** The system prompt of every model: the engine minified in layout alone, which `bun run build` writes beside the
 * package, read once. */
function system(): string {
  prompt ??= JSON.parse(readFileSync(new URL("../system.json", import.meta.url), "utf8")) as string;
  return prompt;
}

/** A value the operator gives, in the shape its prompt wants: a whole number crosses as an int, so a float prompt
 * takes it marked as a float. */
function shaped(shape: string, value: unknown): unknown {
  return shape === "float" && typeof value === "number" ? { is: "float", args: [String(value)] } : value;
}

export interface WorldOptions {
  /** Replay a record for inspection without owning it or starting outside work. */
  readOnly?: boolean;
  cwd?: string;
  record?: string;
  model?: string;
  effort?: ModelThinkingLevel;
  models?: Models;
  roster?: string[];
  /** Replace only the model request, for a deterministic test or another host; the host still names the models. The
   * answer may tell the text and the thinking that it writes as it writes them, which the World streams as a model's. */
  answer?: (
    actor: string,
    chain: string,
    turns: Turn[],
    signal: AbortSignal,
    write: (delta: { text?: string; thinking?: string }) => void,
  ) => Promise<Turn>;
  operator?: (
    prompt: { id: string; shape: string; message: string },
    signal: AbortSignal,
  ) => Promise<unknown>;
  /** The extensions the life plays, as the crate resolves them; the builtins when unsaid. */
  extensions?: Extension[];
  /** The parts for a World of the extensions that are no builtins, by name, which `World.load` imports. */
  parts?: Record<string, WorldExtension>;
}

export type { FileChange } from "./changes.js";

/** An actor the World offers: the name of a model, the efforts it takes, and the window it reads. */
type Actor = [name: string, efforts: string[], window: number];
/** What the World keeps beside its record for the next life. */
interface Saved {
  /** The directory, and the model and the effort the host chose last, which a later World takes when its host names
   * none. */
  options: Pick<WorldOptions, "cwd" | "model" | "effort">;
  deadlines?: [string, number][];
  streams?: [string, { chain: string; text: string; thinking: string }][];
}
/** Files, processes, pi-ai models, and durable records. The engine and its order remain native. */
export class World extends EventEmitter {
  readonly directory: string;
  /** The model a prompt goes to when it names none, and none when the World offers the operator alone. */
  readonly model?: string;
  readonly effort: ModelThinkingLevel;
  readonly records: RecordFile;
  readonly models: Models;
  readonly roster: string[];
  readonly prompts = new Map<string, OperatorPrompt>();
  readonly facts: Fact[] = [];
  readonly activity: Activity;
  /** The extensions the life plays, in order. */
  readonly extensions: Extension[];
  /** The parts of the extensions for this World, in the order of their extensions. */
  readonly parts: WorldPart[];
  private readonly images = new ImageCache();
  readonly streams = new Map<string, { chain: string; text: string; thinking: string }>();
  readonly changes: FileChanges;
  /** The acts that the record showed begun and not done when the life opened, and that no pause holds, by their
   * kind: the engine starts none of them until a wake that this life says, which resume says. One that a pause of
   * the operator holds waits for the wake of the operator. */
  readonly pending = new Map<string, string>();
  /** The ears of the life, whose callable carries a show or a filter of the host into it. */
  readonly ears: Ears;
  private life?: Life;
  private readonly adapter: WorldAdapter;
  private readonly controller = new AbortController();
  private readonly asks = new Map<string, AbortController>();
  private readonly options: WorldOptions;
  /** The roster with the efforts and the window of each actor, which the standing of every chain gives. */
  private readonly actors: Actor[];
  /** Keys the conversations of this life at a provider, since the ids of chains repeat in every life. */
  private readonly conversations = randomUUID();
  private stopped = false;
  private readonly deadlines = new Map<string, number>();
  /** How many of the facts the World has given its host. */
  private emitted = 0;

  constructor(options: WorldOptions = {}) {
    super();
    const named = { model: options.model, roster: options.roster ?? [] };
    const companion = options.record ? `${resolve(options.record)}.world.json` : undefined;
    const saved =
      companion && existsSync(companion) ? (JSON.parse(readFileSync(companion, "utf8")) as Saved) : undefined;
    if (saved) {
      options = {
        ...saved.options,
        ...Object.fromEntries(Object.entries(options).filter(([, value]) => value !== undefined)),
      };
      for (const [id, deadline] of saved.deadlines ?? []) this.deadlines.set(id, deadline);
      for (const [id, stream] of saved.streams ?? []) this.streams.set(id, stream);
    }
    this.options = options;
    this.directory = resolve(options.cwd ?? process.cwd());
    this.extensions = options.extensions ?? builtinExtensions();
    const makers = this.extensions.map((one) => {
      const made = one.builtin ? builtinWorldParts[one.name] : options.parts?.[one.name];
      if (!made && !one.builtin && one.world.ts)
        throw new Error(
          `The extension ${one.name} has a World part in ${one.world.ts}, which this World holds not.`,
        );
      return made;
    });
    let records: RecordFile | undefined;
    let changes: FileChanges | undefined;
    try {
      this.models = options.models ?? builtinModels();
      // What the host names must route. The model a saved record names is a preference of the host, which a later
      // World takes only while it holds that model: the record keeps what it was lived on, and a stood tells each
      // chain what it stands on now.
      const preferred = saved?.options.model;
      this.model = named.model ?? (preferred && this.offers(preferred) ? preferred : named.roster[0]);
      this.roster = [...new Set([this.model, ...named.roster])].filter((name) => name !== undefined);
      this.actors = this.roster.map((name): Actor => {
        const model = this.route(name);
        return [name, getSupportedThinkingLevels(model), model.contextWindow];
      });
      const model = this.model ? this.offers(this.model) : undefined;
      this.effort = model ? clampThinkingLevel(model, options.effort ?? "low") : (options.effort ?? "low");
      // A World that offers no model puts every prompt to the operator, so nothing would ever reach `answer`.
      if (options.answer && !this.roster.length)
        throw new Error("An answer replaces the request of a model, and this World offers no model.");
      this.records = records = new RecordFile(
        options.record ? resolve(options.record) : undefined,
        options.readOnly,
      );
      this.changes = changes = new FileChanges(this.records.path, options.readOnly);
    } catch (error) {
      records?.dispose();
      changes?.dispose();
      throw error;
    }
    this.adapter = new WorldAdapter(this.handle, {
      onFacts: this.hear,
      onFault: (error) => this.emit("fault", error),
      onFact: (fact) => {
        if (this.stopped || this.momentary(fact)) return;
        this.facts.push(fact);
        return this.activity.hear(fact);
      },
      readOnly: options.readOnly,
    });
    const context = worldContext(this.adapter, {
      directory: this.directory,
      readOnly: Boolean(options.readOnly),
      signal: this.controller.signal,
      change: (change) => {
        this.changes.append(change);
        // A listener may ask the life, which no ear may do while it speaks, so the host hears of it after the ear.
        queueMicrotask(() => this.emit("change"));
      },
    });
    this.parts = this.adapter.parts = makers.flatMap((made) => (made ? [made(context)] : []));
    this.activity = new Activity(this.parts);
    this.ears = this.adapter.ears;
  }

  /** The model an actor names, with or without its effort, and nothing when the World holds no such model. */
  offers(actor: string): Model<Api> | undefined {
    return modelNamed(this.models.getModels(), actor);
  }
  route(actor: string): Model<Api> {
    const model = this.offers(actor);
    if (!model) throw new Error(`No model ${actor}. Name one as provider:model.`);
    return model;
  }
  /** The actor a prompt goes to when it names none, which the standing of every chain says. */
  get actor(): string {
    return this.model ? `${this.model}/${this.effort}` : "operator";
  }
  get imageDirectory(): string {
    return this.records.path ? `${this.records.path}.images` : join(this.directory, ".furb/images");
  }
  attachImage(path: string): ImageAttachment {
    if (this.options.readOnly) throw new Error("Record inspection cannot attach an image.");
    // With no record, the images stay in the .furb of the directory, which is made with its ignore rule.
    if (!this.records.path) furbDirectory(this.directory);
    return attachImage(this.imageDirectory, resolve(this.directory, path));
  }

  /** A World on the extensions of its directory, as the crate resolves them when the options name none, with the
   * part for a World of each, imported. */
  static async load(options: WorldOptions = {}): Promise<World> {
    const extensions =
      options.extensions ??
      (options.readOnly ? [] : resolveExtensions(resolve(options.cwd ?? process.cwd())));
    return new World({ ...options, extensions, parts: await loadWorldParts(extensions, options.parts) });
  }

  open(): Life {
    if (this.life) throw new Error("This World already owns a life.");
    try {
      // An inspection plays nothing, since it says nothing new.
      const played = this.options.readOnly ? [] : this.extensions;
      this.life = this.adapter.boot(
        this.records.entries,
        played.flatMap((one) => (one.word ? [one.word] : [])),
        played.flatMap((one) => (one.life ? [one.life] : [])),
      );
      // A life that drifted keeps nothing more, so this World refuses to open on it.
      const raised = this.life.raised;
      if (raised) throw new Error(`${raised.is}: ${raised.args.map(String).join(" ")}`);
      this.learnKinds();
      // The journal said the whole record again before boot returned, so every act that is not done now is one the
      // record showed begun and not done.
      for (const act of this.activity.acts.values())
        if ((act.started || act.kind === "prompt" || act.kind === "rung") && !act.done && !act.paused)
          this.pending.set(act.id, act.kind);
      this.save();
      return this.life;
    } catch (error) {
      this.adapter.stopped = true;
      this.life?.dispose();
      this.records.dispose();
      this.changes.dispose();
      this.controller.abort();
      this.stopped = true;
      throw error;
    }
  }

  handle = ({ kind, args }: WorldRequest): unknown => {
    // An inspection says no wake, so the engine starts nothing, and a World that inspects does nothing new.
    if (this.options.readOnly && ["Clock", "Chance", "Ask", "Wait", "Prompt"].includes(kind))
      throw new Error("Record inspection cannot do new work of the World.");
    switch (kind) {
      case "Stand":
        return [[...this.actors, ["operator", [], 200000]], this.directory, this.actor];
      case "Clock":
        return Date.now() / 1000;
      case "Chance":
        return Math.random();
      case "Keep":
        this.records.keep(args[0] as Entry);
        return null;
      case "Ask":
        return this.ask(String(args[0]), String(args[1]), String(args[2]), args[3] as Turn[]);
      case "Wait": {
        const [seconds, id] = [args[0], String(args[1])];
        if (typeof seconds !== "number") throw new Error(`A wait needs a number of seconds, not ${seconds}.`);
        // A wait that an earlier World started ends when it would have ended then.
        const deadline = this.deadlines.get(id) ?? Date.now() + seconds * 1000;
        this.deadlines.set(id, deadline);
        this.save();
        return new Promise((resolve, reject) => {
          const signal = this.controller.signal;
          const abort = () => {
            cancel();
            reject(signal.reason);
          };
          const cancel = at(deadline, () => {
            signal.removeEventListener("abort", abort);
            this.deadlines.delete(id);
            this.save();
            resolve(null);
          });
          if (signal.aborted) abort();
          else signal.addEventListener("abort", abort, { once: true });
        });
      }
      case "Prompt":
        return this.prompt(String(args[0]), String(args[1]), String(args[2]));
    }
  };

  /** Each kind of question the act table does not know, asked of the life once, outside any ear. */
  private learnKinds(): void {
    const life = this.life;
    if (!life) return;
    for (const [kind, id] of this.activity.unknown)
      this.activity.learn(kind, life.held("acts", [id], "in") === true, this.facts, (question) =>
        life.call(question.verb, question.args ?? [], question.kwargs ?? {}),
      );
  }

  /** Whether a fact answers a query the operator asked outside a run, which is named kind@operator.N. Such a query
   * is of the moment: the record keeps none of it and the World keeps none either, so a host that asks the life at
   * each change hears no change of its own asking. */
  private momentary([kind, id]: Fact): boolean {
    return kind === "done" && /^\w+@operator\.\d+$/.test(id);
  }

  private hear = (): void => {
    if (this.stopped) return;
    this.learnKinds();
    const facts = this.facts.slice(this.emitted);
    this.emitted = this.facts.length;
    if (!facts.length) return;
    for (const [kind, id] of facts) {
      if (kind === "done") {
        this.pending.delete(id);
        this.streams.delete(id);
        this.asks.get(id)?.abort();
        this.asks.delete(id);
        const prompt = this.prompts.get(id);
        if (prompt) {
          this.prompts.delete(id);
          prompt.reject(new Error("The prompt ended."));
        }
      }
    }
    this.emit("facts", facts);
    this.emit("change");
  };

  private async ask(id: string, chain: string, actor: string, turns: Turn[]): Promise<Turn> {
    if (this.stopped) throw new Error("The World was disposed.");
    if (this.life?.outcome(id).done) throw new Error("The act is no longer pending.");
    const controller = new AbortController();
    this.asks.set(id, controller);
    const signal = AbortSignal.any([this.controller.signal, controller.signal]);
    this.streams.set(id, { chain, text: "", thinking: "" });
    this.emit("change");
    try {
      if (this.options.answer)
        return await this.options.answer(actor, chain, turns, signal, ({ text = "", thinking = "" }) => {
          const held = this.streams.get(id);
          if (!held) return;
          held.text += text;
          held.thinking += thinking;
          this.emit("change");
        });
      const model = this.route(actor);
      // The engine phrases every turn as python, so the World renders nothing: a user turn goes as the python the
      // engine wrote, and one that holds nothing goes not at all, and an assistant turn as the blocks its provider
      // gave.
      const messages: Message[] = turns.flatMap(([role, python, usage, blocks]): Message[] => {
        if (
          role === "assistant" &&
          blocks &&
          typeof blocks === "object" &&
          "role" in blocks &&
          blocks.role === "assistant"
        )
          return [blocks as AssistantMessage];
        if (role === "user") {
          if (!python) return [];
          const images = turnImages(this.imageDirectory, python, this.images);
          if (images.length && !model.input.includes("image"))
            throw new Error(`${model.name} does not accept images.`);
          return [
            {
              role,
              content: images.length ? [{ type: "text", text: python }, ...images] : python,
              timestamp: 0,
            },
          ];
        }
        return [
          {
            role,
            content: [{ type: "text", text: python }],
            api: model.api,
            provider: model.provider,
            model: model.id,
            usage: usage
              ? {
                  ...zeroUsage(),
                  input: usage[0] - usage[2] - usage[3],
                  output: usage[1],
                  cacheRead: usage[2],
                  cacheWrite: usage[3],
                  cost: { ...zeroUsage().cost, total: usage[4] },
                }
              : zeroUsage(),
            stopReason: "stop",
            timestamp: 0,
          },
        ];
      });
      const stream = this.models.streamSimple(
        model,
        { systemPrompt: system(), messages },
        {
          signal,
          sessionId: `${this.conversations}/${chain}`,
          reasoning:
            actorParts(actor, this.roster).effort === "off"
              ? undefined
              : (actorParts(actor, this.roster).effort as ThinkingLevel),
        },
      );
      for await (const event of stream) {
        const held = this.streams.get(id);
        if (held && event.type === "text_delta") held.text += event.delta;
        if (held && event.type === "thinking_delta") held.thinking += event.delta;
        this.emit("change");
      }
      const reply = await stream.result();
      if (reply.stopReason === "error" || reply.stopReason === "aborted")
        throw new Error(reply.errorMessage ?? reply.stopReason);
      const text = reply.content
        .filter((block) => block.type === "text")
        .map((block) => block.text)
        .join("")
        .trim();
      const usage = reply.usage;
      return [
        "assistant",
        // A model speaks python alone: a fence or prose around the code stays in the word, for the gate to refuse.
        text,
        [
          usage.input + usage.cacheRead + usage.cacheWrite,
          usage.output,
          usage.cacheRead,
          usage.cacheWrite,
          usage.cost.total,
        ],
        JSON.parse(JSON.stringify(reply)),
      ];
    } finally {
      this.asks.delete(id);
      this.streams.delete(id);
      this.emit("change");
    }
  }

  private async prompt(id: string, shape: string, message: string): Promise<unknown> {
    if (this.stopped) throw new Error("The World was disposed.");
    if (this.life?.outcome(id).done) throw new Error("The prompt is no longer pending.");
    if (this.options.operator)
      return shaped(shape, await this.options.operator({ id, shape, message }, this.controller.signal));
    if (!shapes.some((name) => name === shape))
      return Promise.reject(new Error(`The operator cannot answer ${shape}.`));
    return new Promise((resolve, reject) => {
      this.prompts.set(id, { id, shape, message, resolve, reject });
      this.emit("change");
    });
  }

  answer(id: string, input: string): void {
    const prompt = this.prompts.get(id);
    if (!prompt) throw new Error("This prompt is no longer open.");
    let value: unknown = input;
    if (prompt.shape === "None") value = null;
    else if (prompt.shape === "bool") {
      if (!/^(yes|no|true|false|y|n|0|1)$/i.test(input)) throw new Error("Enter yes or no.");
      value = /^(yes|true|y|1)$/i.test(input);
    } else if (prompt.shape === "int" || prompt.shape === "float") {
      value = Number(input);
      if (
        !input.trim() ||
        !Number.isFinite(value) ||
        (prompt.shape === "int" && !Number.isSafeInteger(value))
      )
        throw new Error(`Enter a valid ${prompt.shape}.`);
      value = shaped(prompt.shape, value);
    } else if (prompt.shape === "list" || prompt.shape === "dict") {
      let plain: unknown;
      try {
        plain = JSON.parse(input);
      } catch {
        throw new Error(`Enter a JSON ${prompt.shape}.`);
      }
      if (
        prompt.shape === "list"
          ? !Array.isArray(plain)
          : !plain || typeof plain !== "object" || Array.isArray(plain)
      )
        throw new Error(`Enter a JSON ${prompt.shape}.`);
      // The native reader keeps what JSON.parse loses: it refuses an unsafe integer, and 2.0 stays a float. A map
      // that holds the key is crosses as its pairs.
      value = marked(plain, decodeRecord(input));
    }
    this.prompts.delete(id);
    prompt.resolve(value);
    this.emit("change");
  }

  private save(): void {
    const record = this.records?.path;
    if (!record || this.options.readOnly) return;
    const path = `${record}.world.json`;
    const saved: Saved = {
      options: { cwd: this.directory, model: this.model, effort: this.effort },
      deadlines: [...this.deadlines],
      streams: [...this.streams],
    };
    saveFile(path, JSON.stringify(saved));
  }

  /** Start the pending work: a wake of each chain that holds some, which the engine answers by starting each
   * command, wait and prompt to the operator of it again, and by asking for each pending rung. */
  async resume(): Promise<void> {
    if (this.options.readOnly) throw new Error("Record inspection cannot resume work.");
    const life = this.life;
    if (!life) return;
    const chains = new Set([...this.pending.keys()].map((id) => this.activity.acts.get(id)?.on));
    this.pending.clear();
    for (const chain of chains) if (chain) life.wake(chain);
    this.emit("change");
  }

  /** Save what the next life needs, then end the life whatever the save came to. */
  async dispose(): Promise<void> {
    if (this.stopped) return;
    this.stopped = true;
    try {
      this.save();
    } finally {
      this.adapter.stopped = true;
      this.life?.dispose();
      this.controller.abort();
      for (const request of this.prompts.values()) request.reject(new Error("The World was disposed."));
      this.prompts.clear();
      for (const part of this.parts)
        try {
          await part.dispose?.();
        } catch (error) {
          this.emit("fault", error);
        }
      this.images.clear();
      this.removeAllListeners();
      // The lease of the record goes last, since a directory that refuses its removal throws here.
      this.changes.dispose();
      this.records.dispose();
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
  const world = new World({ record, models, readOnly: true });
  try {
    world.open();
    await Promise.resolve();
    return { pending: [...world.pending] };
  } finally {
    await world.dispose();
  }
}

export interface Session {
  life: Life;
  world?: World;
  /** The ears of the life, whose callable carries a show or a filter of the host into it. */
  ears: Ears;
  dispose(): Promise<void>;
}
/** Use the built-in World, or replace it with a host callback, on the extensions of the directory as the crate
 * resolves them when the options name none. A host callback does what every World does itself, and the parts of the
 * extensions do the rest, as they do in the built-in World. */
export async function boot(
  options: WorldOptions & { world?: WorldHandler; entries?: Entry[]; onFacts?: (facts: Fact[]) => void } = {},
): Promise<Session> {
  if (options.world) {
    const directory = resolve(options.cwd ?? process.cwd());
    const extensions = options.extensions ?? resolveExtensions(directory);
    const made = await loadWorldParts(extensions, { ...builtinWorldParts, ...options.parts });
    const controller = new AbortController();
    const adapter = new WorldAdapter(options.world, { onFacts: options.onFacts });
    const context = worldContext(adapter, {
      directory,
      readOnly: false,
      signal: controller.signal,
      change: () => {},
    });
    adapter.parts = extensions.flatMap((one) => {
      const part = made[one.name];
      if (!part && one.world.ts)
        throw new Error(
          `The extension ${one.name} has a World part in ${one.world.ts}, which this World holds not.`,
        );
      return part ? [part(context)] : [];
    });
    const life = adapter.boot(
      options.entries,
      extensions.flatMap((one) => (one.word ? [one.word] : [])),
      extensions.flatMap((one) => (one.life ? [one.life] : [])),
    );
    return {
      life,
      ears: adapter.ears,
      dispose: async () => {
        adapter.stopped = true;
        controller.abort();
        for (const part of adapter.parts) await part.dispose?.();
        life.dispose();
      },
    };
  }
  const world = await World.load(options);
  if (options.onFacts) world.on("facts", options.onFacts);
  const life = world.open();
  return { life, world, ears: world.ears, dispose: () => world.dispose() };
}
