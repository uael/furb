import { type ChildProcessWithoutNullStreams, spawn } from "node:child_process";
import { randomUUID } from "node:crypto";
import { EventEmitter } from "node:events";
import { existsSync, mkdirSync, readFileSync, renameSync, statSync, writeFileSync } from "node:fs";
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
import { RecordFile } from "./record.js";
import {
  actorParts,
  type Entry,
  type Fact,
  modelNamed,
  type OperatorPrompt,
  shapes,
  type Turn,
  zeroUsage,
} from "./types.js";

export { display, isTag, safeText } from "./types.js";

let prompt: string | undefined;
/** The system prompt of every model: the engine minified in layout alone, which `bun run build` writes beside the
 * package, read once. */
function system(): string {
  prompt ??= JSON.parse(readFileSync(new URL("../system.json", import.meta.url), "utf8")) as string;
  return prompt;
}

/** The kinds of act whose outside work the World holds until the host resumes it: an ask for a prompt or a rung, a
 * command, and a wait. */
const HELD = ["prompt", "rung", "bash", "wait"];
const INTERRUPTED =
  "The command process ended when the previous World closed. Its recorded output is available; rerun it as a new act if needed.";
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

/** Whether plain data holds a map with the key `is`, which the wire reads as a value of the engine. */
function tagged(value: unknown): boolean {
  if (Array.isArray(value)) return value.some(tagged);
  return Boolean(value && typeof value === "object" && ("is" in value || Object.values(value).some(tagged)));
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
  /** Replace only the model request, for a deterministic test or another host; the host still names the models. */
  answer?: (
    actor: string,
    chain: string,
    turns: Turn[],
    signal: AbortSignal,
    rendered: readonly string[],
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
  options: WorldOptions;
  actors?: Actor[];
  deadlines?: [string, number][];
  streams?: [string, { chain: string; text: string; thinking: string }][];
  spawned?: string[];
  prompted?: string[];
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
  /** The acts whose outside work the World holds until the host calls resume, by their kind. */
  readonly held = new Map<string, string>();
  /** The ears of the life, whose callable carries a show or a filter of the host into it. */
  readonly ears: Ears;
  private life?: Life;
  private readonly adapter: WorldAdapter;
  private readonly controller = new AbortController();
  private readonly asks = new Map<string, AbortController>();
  private readonly commands = new Map<string, Running>();
  private readonly options: WorldOptions;
  /** The roster with the efforts and the window of each actor, which the standing of every chain gives. */
  private readonly actors: Actor[];
  /** Keys the conversations of this life at a provider, since the ids of chains repeat in every life. */
  private readonly conversations = randomUUID();
  private delivery: Promise<unknown> = Promise.resolve();
  private stopped = false;
  private holding: boolean;
  private release?: () => void;
  private readonly resumeGate: Promise<void>;
  private readonly deferred = new Map<string, Command>();
  private readonly feeds = new Map<string, (string | null)[]>();
  private readonly deadlines = new Map<string, number>();
  /** The commands this World or an earlier one spawned and that are not done, kept on the disk so that no life runs
   * one of them again. */
  private readonly spawned = new Set<string>();
  /** The prompts that this World or an earlier one put to the operator and that are not done. */
  private readonly prompted = new Set<string>();
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
      for (const id of saved.spawned ?? []) this.spawned.add(id);
      for (const id of saved.prompted ?? []) this.prompted.add(id);
    }
    this.options = options;
    this.directory = resolve(options.cwd ?? process.cwd());
    let records: RecordFile | undefined;
    let changes: FileChanges | undefined;
    try {
      this.models = options.models ?? builtinModels();
      // What the host names must route. What a saved record names stays, whether it routes or not: the record was
      // lived on it, so a later life stands on it too, and an ask of a model gone since fails as the ask it is.
      for (const name of named.model ? [named.model, ...named.roster] : named.roster) this.route(name);
      const kept = [saved?.options.model ?? "", ...(saved?.options.roster ?? [])].filter(Boolean);
      this.model = named.model ?? kept[0] ?? named.roster[0];
      this.roster = [...new Set([this.model, ...kept, ...named.roster])].filter((name) => name !== undefined);
      const standing = new Map((saved?.actors ?? []).map(([name, ...rest]) => [name, rest] as const));
      this.actors = this.roster.map((name): Actor => {
        const model = this.offers(name);
        if (model) return [name, getSupportedThinkingLevels(model), model.contextWindow];
        const actor = standing.get(name);
        if (!actor) throw new Error(`No model ${name}, and the record keeps no standing of it.`);
        return [name, ...actor];
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
      // A later life holds its outside work until the host resumes it, and an inspection holds it for good.
      this.holding = Boolean(options.readOnly) || this.records.entries.length > 0;
      this.resumeGate = new Promise((resolve) => {
        this.release = resolve;
      });
      if (!this.holding) this.release?.();
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
        if (this.stopped || this.momentary(fact)) return;
        this.facts.push(fact);
        this.activity.hear(fact);
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
    return attachImage(this.imageDirectory, resolve(this.directory, path));
  }

  open(): Life {
    if (this.life) throw new Error("This World already owns a life.");
    try {
      this.life = this.adapter.boot(this.records.entries);
      // A life that drifted keeps nothing more, so this World refuses to open on it.
      const raised = this.life.raised;
      if (raised) throw new Error(`${raised.is}: ${raised.args.map(String).join(" ")}`);
      this.learnKinds();
      if (this.holding)
        for (const act of this.activity.acts.values())
          if (HELD.includes(act.kind) && !act.done) this.held.set(act.id, act.kind);
      this.endHoldWhenEmpty();
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
    if (this.options.readOnly && ["Read", "Write", "Clock", "Chance"].includes(kind))
      throw new Error("Record inspection cannot perform a new World query.");
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
        const info = statSync(path);
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
      case "Ask":
        return this.ask(
          String(args[0]),
          String(args[1]),
          String(args[2]),
          args[3] as Turn[],
          args[4] as string[],
        );
      case "Wait": {
        const [seconds, id] = [args[0], String(args[1])];
        if (typeof seconds !== "number") throw new Error(`A wait needs a number of seconds, not ${seconds}.`);
        const deadline = this.deadlines.get(id) ?? Date.now() + seconds * 1000;
        this.deadlines.set(id, deadline);
        this.save();
        return this.resumeGate.then(
          () =>
            new Promise((resolve, reject) => {
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
            }),
        );
      }
      case "Run": {
        const command = args[0] as Command;
        // A command an earlier World spawned has no process now, and resume ends it as interrupted. Its done is not
        // in the record, so it is held work, and a World that holds nothing refuses it at once.
        if (this.spawned.has(command.id)) {
          if (!this.holding) throw new Error(INTERRUPTED);
        } else if (this.holding) this.deferred.set(command.id, command);
        else this.run(command);
        return null;
      }
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
        this.deferred.delete(String(args[0]));
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

  /** Each kind of question the act table does not know, asked of the life once, outside any ear. */
  private learnKinds(): void {
    for (const [kind, id] of this.activity.unknown)
      this.activity.learn(kind, this.life?.held("acts", [id], "in") === true, this.facts);
  }

  /** Whether a fact answers a query the operator asked outside a run. Such a query is of the moment: the record
   * keeps none of it and the World keeps none either, so a host that asks the life at each change hears no change of
   * its own asking. */
  private momentary([kind, id]: Fact): boolean {
    const [scheme = "", lineage = ""] = id.split("://");
    return (
      kind === "done" &&
      /^operator\.\d+$/.test(lineage) &&
      !this.activity.acts.has(id) &&
      !this.activity.unknown.has(scheme)
    );
  }

  /** The World lets its outside work go once it holds nothing, but an inspection holds for good. */
  private endHoldWhenEmpty(): void {
    if (!this.holding || this.held.size || this.options.readOnly) return;
    this.holding = false;
    this.release?.();
  }

  private hear = (): void => {
    if (this.stopped) return;
    this.learnKinds();
    const facts = this.facts.slice(this.emitted);
    this.emitted = this.facts.length;
    if (!facts.length) return;
    for (const [kind, id] of facts) {
      if (this.holding && HELD.includes(kind) && !this.activity.acts.get(id)?.done) this.held.set(id, kind);
      if (kind === "done") {
        this.held.delete(id);
        this.streams.delete(id);
        this.deferred.delete(id);
        this.feeds.delete(id);
        this.asks.get(id)?.abort();
        this.asks.delete(id);
        const prompt = this.prompts.get(id);
        if (prompt) {
          this.prompts.delete(id);
          prompt.reject(new Error("The prompt ended."));
        }
        if (this.spawned.delete(id) || this.prompted.delete(id)) this.save();
      }
    }
    this.endHoldWhenEmpty();
    this.emit("facts", facts);
    this.emit("change");
  };

  private async ask(id: string, chain: string, actor: string, turns: Turn[], texts: string[]): Promise<Turn> {
    await this.resumeGate;
    if (this.stopped) throw new Error("The World was disposed.");
    if (this.life?.outcome(id).done) throw new Error("The act is no longer pending.");
    const controller = new AbortController();
    this.asks.set(id, controller);
    const signal = AbortSignal.any([this.controller.signal, controller.signal]);
    this.streams.set(id, { chain, text: "", thinking: "" });
    this.emit("change");
    try {
      if (this.options.answer) return await this.options.answer(actor, chain, turns, signal, texts);
      const model = this.route(actor);
      const messages: Message[] = turns.map(([role, parts, usage, blocks], index) => {
        if (
          role === "assistant" &&
          blocks &&
          typeof blocks === "object" &&
          "role" in blocks &&
          blocks.role === "assistant"
        )
          return blocks as AssistantMessage;
        if (role === "user") {
          const images = turnImages(this.imageDirectory, parts, this.images);
          if (images.length && !model.input.includes("image"))
            throw new Error(`${model.name} does not accept images.`);
          return {
            role,
            content: images.length
              ? [{ type: "text", text: texts[index] ?? "" }, ...images]
              : (texts[index] ?? ""),
            timestamp: 0,
          };
        }
        return {
          role,
          content: [{ type: "text", text: texts[index] ?? "" }],
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
        };
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
        [text],
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
    this.prompted.add(id);
    this.save();
    await this.resumeGate;
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
      if (tagged(plain)) throw new Error("A map in an answer cannot hold the key is.");
      // The native reader keeps what JSON.parse loses: it refuses an unsafe integer, and 2.0 stays a float.
      value = decodeRecord(input);
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
      options: { cwd: this.directory, model: this.model, effort: this.effort, roster: this.roster },
      actors: this.actors,
      deadlines: [...this.deadlines],
      streams: [...this.streams],
      spawned: [...this.spawned],
      prompted: [...this.prompted],
    };
    writeFileSync(`${path}.tmp`, JSON.stringify(saved), { mode: 0o600 });
    renameSync(`${path}.tmp`, path);
  }

  /** Let the held work go. A command an earlier World spawned ends with a refusal, and is never run again. */
  async resume(): Promise<void> {
    if (this.options.readOnly) throw new Error("Record inspection cannot resume work.");
    const life = this.life;
    if (!life || !this.holding) return;
    this.holding = false;
    this.release?.();
    for (const [id, kind] of this.held) {
      if (life.outcome(id).done) continue;
      if (kind === "bash" && !this.deferred.has(id)) life.close({ is: "Refused", args: [INTERRUPTED] }, id);
      else if (kind === "wait" || this.prompted.has(id)) {
        // The engine starts a wait or a prompt to the operator only when the record holds nothing of it, so the
        // one that the record holds the World starts again here, and every other one has its start already.
        const [, kept] = life.call<[unknown, unknown[]]>("ask", ["holds", "", id], {});
        if (kept.length) this.adapter.start(life.get(id));
      }
    }
    for (const command of this.deferred.values()) {
      try {
        this.run(command);
      } catch (error) {
        life.close(
          { is: "Refused", args: [error instanceof Error ? error.message : String(error)] },
          command.id,
        );
      }
    }
    this.deferred.clear();
    this.held.clear();
    this.save();
    this.emit("change");
  }

  private send(kind: string, id: string, words: unknown[]): void {
    this.delivery = this.delivery
      .then(async () => {
        if (!this.stopped) await this.life?.send(kind, id, words, "world");
      })
      .catch((error) => {
        this.emit("fault", error);
      });
  }

  private run(command: Command): void {
    this.spawned.add(command.id);
    // The note stands on the disk before the process does, so no later life runs the command again.
    this.save();
    const child = spawn(
      "/bin/sh",
      ["-c", command.merged ? `exec 2>&1\n${command.command}` : command.command],
      {
        cwd: resolve(this.directory, command.here),
        stdio: "pipe",
        detached: process.platform !== "win32",
      },
    );
    let late = false;
    const stop = () => {
      try {
        if (process.platform !== "win32" && child.pid) process.kill(-child.pid, "SIGKILL");
        else child.kill("SIGKILL");
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
      child[name].on("data", (text: string) => this.send("out", command.id, [text, name]));
    }
    child.on("error", (error) => {
      cancel();
      this.commands.delete(command.id);
      this.delivery = this.delivery.then(async () => {
        if (!this.stopped) await this.life?.close({ is: "Refused", args: [error.message] }, command.id);
      });
    });
    child.on("close", (code) => {
      cancel();
      this.commands.delete(command.id);
      this.send("exited", command.id, [late ? null : code]);
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
      this.release?.();
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
export async function inspectRecord(record: string, models?: Models): Promise<{ held: [string, string][] }> {
  const world = new World({ record, models, readOnly: true });
  try {
    world.open();
    await Promise.resolve();
    return { held: [...world.held] };
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
/** Use the built-in World, or replace it with a host callback. */
export function boot(
  options: WorldOptions & { world?: WorldHandler; entries?: Entry[]; onFacts?: (facts: Fact[]) => void } = {},
): Session {
  if (options.world) {
    const adapter = new WorldAdapter(options.world, options.onFacts);
    const life = adapter.boot(options.entries);
    return {
      life,
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
