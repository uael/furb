import { type ChildProcessWithoutNullStreams, spawn } from "node:child_process";
import { EventEmitter } from "node:events";
import { existsSync, mkdirSync, readFileSync, renameSync, statSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { setTimeout as delay } from "node:timers/promises";
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
import type { Life } from "../index.cjs";
import { type FileChange, FileChanges } from "./changes.js";
import { WorldAdapter, type WorldHandler, type WorldRequest } from "./ears.js";
import { type ClaudeOptions, claudeProvider, zeroUsage } from "./providers/claude.js";
import { RecordFile } from "./record.js";
import { actorParts, type Entry, type Fact, type OperatorPrompt, shapes, type Turn } from "./types.js";

export { display, isTag, safeText } from "./types.js";

export interface WorldOptions {
  cwd?: string;
  record?: string;
  model?: string;
  effort?: ModelThinkingLevel;
  models?: Models;
  roster?: string[];
  claude?: ClaudeOptions;
  /** Replace only the model request, for a deterministic test or another host. */
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

interface Command {
  id: string;
  here: string;
  command: string;
  fed: boolean;
  timeout: number;
  merged: boolean;
}
interface Running {
  child: ChildProcessWithoutNullStreams;
  timeout: ReturnType<typeof setTimeout>;
  stop(): void;
}

/** Files, processes, pi-ai models, and durable records. The engine and its order remain native. */
export class World extends EventEmitter {
  readonly directory: string;
  readonly model: string;
  readonly effort: ModelThinkingLevel;
  readonly records: RecordFile;
  readonly models: Models;
  readonly roster: string[];
  readonly prompts = new Map<string, OperatorPrompt>();
  readonly facts: Fact[] = [];
  readonly streams = new Map<string, { chain: string; text: string; thinking: string }>();
  readonly changes: FileChanges;
  readonly held = new Map<string, string>();
  private life?: Life;
  private readonly controller = new AbortController();
  private readonly asks = new Map<string, AbortController>();
  private readonly commands = new Map<string, Running>();
  private readonly cli?: ReturnType<typeof claudeProvider>;
  private readonly options: WorldOptions;
  private delivery: Promise<unknown> = Promise.resolve();
  private stopped = false;
  private adapter?: WorldAdapter;
  private holding = false;
  private release?: () => void;
  private readonly resumeGate: Promise<void>;
  private readonly deferredCommands = new Map<string, Command>();
  private readonly feeds = new Map<string, (string | null)[]>();
  private readonly deadlines = new Map<string, number>();
  private readonly waits = new Set<string>();

  constructor(options: WorldOptions = {}) {
    super();
    let legacyChanges: FileChange[] = [];
    if (options.record && existsSync(`${resolve(options.record)}.world.json`)) {
      const saved = JSON.parse(readFileSync(`${resolve(options.record)}.world.json`, "utf8")) as {
        options: WorldOptions;
        changes?: FileChange[];
        deadlines?: [string, number][];
        streams?: [string, { chain: string; text: string; thinking: string }][];
      };
      options = {
        ...saved.options,
        ...Object.fromEntries(Object.entries(options).filter(([, value]) => value !== undefined)),
      };
      options.roster = [...new Set([...(saved.options.roster ?? []), ...(options.roster ?? [])])];
      legacyChanges = saved.changes ?? [];
      for (const [id, deadline] of saved.deadlines ?? []) this.deadlines.set(id, deadline);
      for (const [id, stream] of saved.streams ?? []) this.streams.set(id, stream);
    }
    this.options = options;
    this.directory = resolve(options.cwd ?? process.cwd());
    this.model = options.model ?? "claude-cli:sonnet";
    let records: RecordFile | undefined;
    let changes: FileChanges | undefined;
    try {
      if (options.models) this.models = options.models;
      else {
        this.cli = claudeProvider(options.claude);
        const models = builtinModels();
        models.setProvider(this.cli.provider);
        this.models = models;
      }
      this.roster = [
        ...new Set([
          this.model,
          ...(options.roster ??
            (options.models ? [] : ["claude-cli:sonnet", "claude-cli:opus", "claude-cli:haiku"])),
        ]),
      ];
      for (const name of this.roster) this.route(name);
      this.effort = clampThinkingLevel(this.route(this.model), options.effort ?? "low");
      this.records = records = new RecordFile(options.record ? resolve(options.record) : undefined);
      this.changes = changes = new FileChanges(this.records.path);
      if (!this.changes.length) for (const change of legacyChanges) this.changes.append(change);
      this.holding = this.records.entries.length > 0;
      this.resumeGate = new Promise((resolve) => {
        this.release = resolve;
      });
      if (!this.holding) this.release?.();
    } catch (error) {
      records?.dispose();
      changes?.dispose();
      this.cli?.dispose();
      throw error;
    }
  }

  route(actor: string): Model<Api> {
    for (const name of [actor, actorParts(actor).model]) {
      const separator = name.indexOf(":");
      const provider = separator < 0 ? "claude-cli" : name.slice(0, separator);
      const id = separator < 0 ? name : name.slice(separator + 1);
      const model = this.models.getModel(provider, id);
      if (model) return model;
    }
    throw new Error(`No model ${actor}. Use provider:model, for example claude-cli:sonnet.`);
  }

  open(): Life {
    if (this.life) throw new Error("This World already owns a life.");
    try {
      this.adapter = new WorldAdapter(this.handle, this.hear, (error) => this.emit("fault", error));
      this.life = this.adapter.boot(this.records.entries);
      if (this.holding) {
        const ids = this.life.held("acts", [], "keys") as string[];
        const chains = new Set<string>();
        for (const id of ids) {
          const act = this.life.get(id);
          if (!["chain", "grant"].includes(act[0]) && !this.life.outcome(id).done) {
            this.held.set(id, act[0]);
            chains.add(act[3]);
          }
        }
        for (const chain of chains) this.pause(chain);
        if (!this.held.size) {
          this.holding = false;
          this.release?.();
        }
      }
      this.save();
      return this.life;
    } catch (error) {
      if (this.adapter) this.adapter.stopped = true;
      this.life?.dispose();
      this.records.dispose();
      this.changes.dispose();
      this.cli?.dispose();
      this.controller.abort();
      this.stopped = true;
      throw error;
    }
  }

  handle = ({ kind, args }: WorldRequest): unknown => {
    switch (kind) {
      case "Stand":
        return [
          [
            ...this.roster.map((name) => [
              name,
              getSupportedThinkingLevels(this.route(name)),
              this.route(name).contextWindow,
            ]),
            ["operator", [], 200000],
          ],
          this.directory,
          `${this.model}/${this.effort}`,
        ];
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
        return { path, content: new TextDecoder("utf-8", { fatal: true }).decode(readFileSync(path)) };
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
        this.emit("change");
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
        const id = String(args[1]);
        this.waits.add(id);
        const deadline = this.deadlines.get(id) ?? Date.now() + Number(args[0]) * 1000;
        this.deadlines.set(id, deadline);
        this.save();
        return this.resumeGate
          .then(() => delay(Math.max(0, deadline - Date.now()), null, { signal: this.controller.signal }))
          .then(() => {
            this.deadlines.delete(id);
            this.waits.delete(id);
            this.save();
            return null;
          });
      }
      case "Run": {
        const command = args[0] as Command;
        if (this.holding) this.deferredCommands.set(command.id, command);
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
        this.deferredCommands.delete(String(args[0]));
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

  private hear = (facts: Fact[]): void => {
    if (this.stopped) return;
    this.facts.push(...facts);
    for (const [kind, id] of facts) {
      if (this.holding && ["prompt", "rung", "bash", "wait"].includes(kind) && !this.life?.outcome(id).done)
        this.held.set(id, kind);
      if (kind === "done") {
        this.held.delete(id);
        this.streams.delete(id);
        this.deferredCommands.delete(id);
        this.feeds.delete(id);
        this.asks.get(id)?.abort();
        this.asks.delete(id);
        const prompt = this.prompts.get(id);
        if (prompt) {
          this.prompts.delete(id);
          prompt.reject(new Error("The prompt ended."));
        }
      }
    }
    if (this.holding && this.held.size === 0) {
      this.holding = false;
      this.release?.();
    }
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
      const messages: Message[] = turns.map(([role, , usage, blocks], index) => {
        if (
          role === "assistant" &&
          blocks &&
          typeof blocks === "object" &&
          "role" in blocks &&
          blocks.role === "assistant"
        )
          return blocks as AssistantMessage;
        if (role === "user") return { role, content: texts[index] ?? "", timestamp: 0 };
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
      const systemPrompt = JSON.parse(
        readFileSync(new URL("../system.json", import.meta.url), "utf8"),
      ) as string;
      const stream = this.models.streamSimple(
        model,
        { systemPrompt, messages },
        {
          signal,
          sessionId: chain,
          reasoning:
            actorParts(actor).effort === "off" ? undefined : (actorParts(actor).effort as ThinkingLevel),
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
      const fences = [...text.matchAll(/```(?:python|py)?\n([\s\S]*?)```/g)];
      const word = fences.length === 1 ? (fences[0]?.[1] ?? text).trim() : text;
      const usage = reply.usage;
      return [
        "assistant",
        [word],
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
    await this.resumeGate;
    if (this.stopped) throw new Error("The World was disposed.");
    if (this.life?.outcome(id).done) throw new Error("The prompt is no longer pending.");
    if (this.options.operator) return this.options.operator({ id, shape, message }, this.controller.signal);
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
      if (prompt.shape === "float") value = { is: "float", args: [String(value)] };
    } else if (prompt.shape === "list" || prompt.shape === "dict") {
      value = JSON.parse(input);
      if (
        prompt.shape === "list"
          ? !Array.isArray(value)
          : !value || typeof value !== "object" || Array.isArray(value)
      )
        throw new Error(`Enter a JSON ${prompt.shape}.`);
    }
    this.prompts.delete(id);
    prompt.resolve(value);
    this.emit("change");
  }

  private save(): void {
    const record = this.records?.path;
    if (!record) return;
    const path = `${record}.world.json`;
    writeFileSync(
      `${path}.tmp`,
      JSON.stringify({
        options: { cwd: this.directory, model: this.model, effort: this.effort, roster: this.roster },
        deadlines: [...this.deadlines],
        streams: [...this.streams],
        held: [...this.held],
      }),
      { mode: 0o600 },
    );
    renameSync(`${path}.tmp`, path);
  }

  /** Confirm the resume. Commands whose process ended at exit are closed as interrupted, never rerun. */
  async resume(): Promise<void> {
    if (!this.life || !this.holding) return;
    const chains = new Set<string>();
    for (const [id, kind] of this.held) {
      const act = await this.life.get(id);
      chains.add(act[3]);
      if (kind === "bash" && !this.deferredCommands.has(id))
        await this.life.close(
          {
            is: "Refused",
            args: [
              "The command process ended when the previous World closed. Its recorded output is available; rerun it as a new act if needed.",
            ],
          },
          id,
        );
      if (kind === "wait" && !this.waits.has(id)) {
        void Promise.resolve(this.handle({ kind: "Wait", args: [act[4], id] }))
          .then(() => {
            if (!this.stopped) this.life?.send("done", id, [null], "world");
          })
          .catch((error) => {
            if (!this.stopped) this.emit("fault", error);
          });
      }
      if (kind === "prompt" && act[6] === "operator" && !this.prompts.has(id)) {
        void this.prompt(id, String(act[4]), String(act[5]))
          .then((value) => this.life?.close(value, id))
          .catch((error) => this.emit("fault", error));
      }
    }
    this.holding = false;
    for (const command of this.deferredCommands.values()) this.run(command);
    this.deferredCommands.clear();
    this.release?.();
    for (const chain of chains) await this.life.wake(chain);
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
    const timeout = setTimeout(() => {
      late = true;
      stop();
    }, command.timeout * 1000);
    this.commands.set(command.id, { child, timeout, stop });
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
      clearTimeout(timeout);
      this.commands.delete(command.id);
      this.delivery = this.delivery.then(async () => {
        if (!this.stopped) await this.life?.close({ is: "Refused", args: [error.message] }, command.id);
      });
    });
    child.on("close", (code) => {
      clearTimeout(timeout);
      this.commands.delete(command.id);
      this.send("exited", command.id, [late ? null : code]);
    });
  }

  async dispose(): Promise<void> {
    if (this.stopped) return;
    if (this.life && this.records.path) {
      const ids = this.life.held("acts", [], "keys") as string[];
      const chains = new Set<string>();
      for (const id of ids) {
        const act = this.life.get(id);
        if (!["chain", "grant"].includes(act[0]) && !this.life.outcome(id).done) {
          chains.add(act[3]);
          this.held.set(id, act[0]);
        }
      }
      for (const chain of chains) this.pause(chain);
    }
    this.stopped = true;
    this.save();
    if (this.adapter) this.adapter.stopped = true;
    await this.life?.dispose();
    this.release?.();
    this.controller.abort();
    for (const request of this.prompts.values()) request.reject(new Error("The World was disposed."));
    this.prompts.clear();
    for (const command of this.commands.values()) {
      clearTimeout(command.timeout);
      command.stop();
    }
    this.commands.clear();
    this.cli?.dispose();
    this.changes.dispose();
    this.records.dispose();
    this.removeAllListeners();
  }

  private pause(chain: string): void {
    const last = this.records.entries
      .map((entry) => entry[1])
      .findLast((fact) => ["pause", "wake"].includes(fact[0]) && fact[1] === chain);
    if (last?.[0] !== "pause") this.life?.pause(chain);
  }
}

export interface Session {
  life: Life;
  world?: World;
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
      dispose: async () => {
        adapter.stopped = true;
        life.dispose();
      },
    };
  }
  const world = new World(options);
  if (options.onFacts) world.on("facts", options.onFacts);
  const life = world.open();
  return { life, world, dispose: () => world.dispose() };
}
