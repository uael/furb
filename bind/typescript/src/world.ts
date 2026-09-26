import { type ChildProcessWithoutNullStreams, spawn, spawnSync } from "node:child_process";
import { randomUUID } from "node:crypto";
import { EventEmitter } from "node:events";
import { existsSync, mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
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
import { decodeRecord, type Life } from "../index.cjs";
import { Activity, type RunState } from "./activity.js";
import { FileChanges } from "./changes.js";
import { type Ears, WorldAdapter, type WorldHandler, type WorldRequest } from "./ears.js";
import { attachImage, type ImageAttachment, ImageCache, turnImages } from "./images.js";
import { furbDirectory, saveFile } from "./project.js";
import { RecordFile } from "./record.js";
import { shell } from "./shell.js";
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

/** The kinds of act whose work the record may show begun and not done, which waits for a wake. */
const PENDING = ["prompt", "rung", "bash", "wait"];
/** The longest delay that one timer holds, in milliseconds. */
const LONGEST = 2 ** 31 - 1;

/** Calls the action at a time, through timers of LONGEST at most, and never for a time that is not finite; gives the
 * cancel. */
function at(time: number, action: () => void): () => void {
  let timer: ReturnType<typeof setTimeout> | undefined;
  const wait = () => {
    timer = setTimeout(step, Math.min(Math.max(time - Date.now(), 0), LONGEST));
  };
  const step = () => (Date.now() < time ? wait() : action());
  if (Number.isFinite(time)) wait();
  return () => clearTimeout(timer);
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
interface Command {
  id: string;
  here: string;
  command: string;
  fed: boolean;
  timeout: number | null;
  merged: boolean;
}
interface Running {
  child: ChildProcessWithoutNullStreams;
  cancel(): void;
  stop(): void;
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
  readonly activity = new Activity();
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
  /** The model request of each rung that a reply of the World asks for, which a done of that rung ends. */
  private readonly replies = new Map<string, AbortController>();
  private readonly commands = new Map<string, Running>();
  private readonly options: WorldOptions;
  /** The roster with the efforts and the window of each actor, which the standing of every chain gives. */
  private readonly actors: Actor[];
  /** Keys the conversations of this life at a provider, since the ids of chains repeat in every life. */
  private readonly conversations = randomUUID();
  private delivery: Promise<unknown> = Promise.resolve();
  private stopped = false;
  private readonly feeds = new Map<string, (string | null)[]>();
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
    let records: RecordFile | undefined;
    let changes: FileChanges | undefined;
    try {
      this.models = options.models ?? builtinModels();
      // What the host names must route. The model a saved record names is a preference of the host, which a later
      // World takes only while it holds that model: the record keeps what it was lived on, and the stand of the
      // later life tells each chain what it stands on now.
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
    this.adapter = new WorldAdapter(
      this.handle,
      this.hear,
      (error) => this.emit("fault", error),
      (fact) => {
        if (this.stopped) return;
        this.facts.push(fact);
        return this.activity.hear(fact);
      },
    );
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

  open(): Life {
    if (this.life) throw new Error("This World already owns a life.");
    try {
      this.life = this.adapter.boot(this.records.entries);
      // A life that drifted keeps nothing more, so this World refuses to open on it.
      const raised = this.life.raised;
      if (raised) throw new Error(`${raised.is}: ${raised.args.map(String).join(" ")}`);
      // The journal said the whole record again before boot returned, so every act that is not done now is one the
      // record showed begun and not done.
      for (const act of this.activity.acts.values())
        if (PENDING.includes(act.kind) && !act.done && !act.paused) this.pending.set(act.id, act.kind);
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
    if (
      this.options.readOnly &&
      ["Read", "Write", "Clock", "Chance", "Reply", "Run", "Wait", "Prompt"].includes(kind)
    )
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
      case "Read": {
        const path = this.path(String(args[0]), String(args[1]));
        let info: ReturnType<typeof statSync>;
        try {
          info = statSync(path);
        } catch (error) {
          // A path that names nothing is said in plain words, which the model and the operator both read.
          if ((error as NodeJS.ErrnoException).code === "ENOENT")
            throw new Error(`There is no file at ${path}.`);
          throw error;
        }
        if (!info.isFile() || info.size > 524288)
          throw new Error(`Read needs a text file at most 524288 bytes: ${path}`);
        return {
          path,
          content: new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(readFileSync(path)),
        };
      }
      case "Write": {
        const path = this.path(String(args[0]), String(args[1]));
        let before = "";
        try {
          before = readFileSync(path, "utf8");
        } catch (error) {
          if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
        }
        mkdirSync(dirname(path), { recursive: true });
        writeFileSync(path, String(args[2]));
        this.changes.append({ path, before, after: String(args[2]) });
        // A listener may ask the life, which no ear may do while it speaks, so the host hears of it after the ear.
        queueMicrotask(() => this.emit("change"));
        return { path, content: readFileSync(path, "utf8") };
      }
      case "Reply":
        return this.reply(
          String(args[0]),
          String(args[1]),
          String(args[2]),
          String(args[3]),
          args[4] as Turn[],
        );
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
      case "Run":
        this.run(args[0] as Command);
        return null;
      case "Feed": {
        const id = String(args[0]);
        const child = this.commands.get(id)?.child;
        if (!child)
          this.feeds.set(id, [...(this.feeds.get(id) ?? []), args[1] === null ? null : String(args[1])]);
        else if (args[1] === null) child.stdin.end();
        else child.stdin.write(String(args[1]));
        return null;
      }
      case "Slay":
        this.commands.get(String(args[0]))?.stop();
        this.feeds.delete(String(args[0]));
        return null;
      case "Prompt":
        return this.prompt(String(args[0]), String(args[1]), String(args[2]));
    }
  };

  private path(here: string, path: string): string {
    if (path.includes("://")) throw new Error(`No file at ${path}.`);
    return resolve(this.directory, here, path);
  }

  private hear = (): void => {
    if (this.stopped) return;
    const facts = this.facts.slice(this.emitted);
    this.emitted = this.facts.length;
    if (!facts.length) return;
    for (const [kind, id] of facts) {
      if (kind === "done") {
        this.pending.delete(id);
        this.streams.delete(id);
        this.feeds.delete(id);
        this.replies.get(id)?.abort();
        this.replies.delete(id);
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

  /** One turn of a model for a reply, streamed under the rung the reply asks for, whose done ends the request. */
  private async reply(id: string, rung: string, chain: string, actor: string, turns: Turn[]): Promise<Turn> {
    if (this.stopped) throw new Error("The World was disposed.");
    if (this.life?.outcome(id).done) throw new Error("The act is no longer pending.");
    const controller = new AbortController();
    this.replies.set(rung, controller);
    const signal = AbortSignal.any([this.controller.signal, controller.signal]);
    this.streams.set(rung, { chain, text: "", thinking: "" });
    this.emit("change");
    try {
      if (this.options.answer)
        return await this.options.answer(actor, chain, turns, signal, ({ text = "", thinking = "" }) => {
          const held = this.streams.get(rung);
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
        const held = this.streams.get(rung);
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
      this.replies.delete(rung);
      this.streams.delete(rung);
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

  /** What a command came to, said into the life in the order it came, and never while an ear speaks. */
  private deliver(action: () => void): void {
    this.delivery = this.delivery
      .then(() => {
        if (!this.stopped) action();
      })
      .catch((error) => {
        this.emit("fault", error);
      });
  }

  private run(command: Command): void {
    const child = spawn(shell, ["-c", command.merged ? `exec 2>&1\n${command.command}` : command.command], {
      cwd: resolve(this.directory, command.here),
      stdio: "pipe",
      detached: process.platform !== "win32",
      windowsHide: true,
    });
    let late = false;
    // A command ends with every process it started: on Unix its process group, which it leads, and on Windows,
    // which has no process group that a program can signal, its tree of processes.
    const stop = () => {
      if (!child.pid) return;
      try {
        if (process.platform === "win32")
          spawnSync("taskkill", ["/pid", String(child.pid), "/t", "/f"], {
            stdio: "ignore",
            windowsHide: true,
          });
        else process.kill(-child.pid, "SIGKILL");
      } catch {}
    };
    // A command with no timeout runs until it ends.
    const cancel = at(
      command.timeout === null ? Number.POSITIVE_INFINITY : Date.now() + command.timeout * 1000,
      () => {
        late = true;
        stop();
      },
    );
    this.commands.set(command.id, { child, cancel, stop });
    if (!command.fed) child.stdin.end();
    child.stdin.on("error", () => {});
    for (const text of this.feeds.get(command.id) ?? []) {
      if (text === null) child.stdin.end();
      else child.stdin.write(text);
    }
    this.feeds.delete(command.id);
    for (const name of ["stdout", "stderr"] as const) {
      child[name].setEncoding("utf8");
      child[name].on("data", (text: string) =>
        this.deliver(() => this.adapter.say("out", command.id, [text, name])),
      );
    }
    child.on("error", (error) => {
      cancel();
      this.commands.delete(command.id);
      this.deliver(() => this.adapter.say("done", command.id, [{ is: "Refused", args: [error.message] }]));
    });
    child.on("close", (code) => {
      cancel();
      this.commands.delete(command.id);
      this.deliver(() => this.adapter.exited(command.id, late ? null : code));
    });
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
      for (const command of this.commands.values()) {
        command.cancel();
        command.stop();
      }
      this.commands.clear();
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
  /** The adapter a World that replaces the supplied one speaks through: what a command writes and how it ends. */
  adapter?: WorldAdapter;
  /** The ears of the life, whose callable carries a show or a filter of the host into it. */
  ears: Ears;
  dispose(): Promise<void>;
}
/** Use the built-in World, or replace it with a host callback. */
export function boot(
  options: WorldOptions & { world?: WorldHandler; entries?: Entry[]; onFacts?: (facts: Fact[]) => void } = {},
): Session {
  if (options.world) {
    const adapter = new WorldAdapter(options.world, options.onFacts);
    const life = adapter.boot(options.entries);
    return {
      life,
      adapter,
      ears: adapter.ears,
      dispose: async () => {
        adapter.stopped = true;
        life.dispose();
      },
    };
  }
  const world = new World(options);
  if (options.onFacts) world.on("facts", options.onFacts);
  const life = world.open();
  return { life, world, ears: world.ears, dispose: () => world.dispose() };
}
