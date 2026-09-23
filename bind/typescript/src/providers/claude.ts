// The session and delta design comes from uael/dirt's MIT-licensed Claude CLI provider.
// This provider uses pi-ai's normalized transcripts and runs pure completions with no MCP bridge.
import { type ChildProcessWithoutNullStreams, spawn } from "node:child_process";
import { randomUUID } from "node:crypto";
import { accessSync, constants, readdirSync } from "node:fs";
import { homedir } from "node:os";
import { delimiter, join } from "node:path";
import {
  type Api,
  type AssistantMessage,
  type AssistantMessageEventStream,
  createAssistantMessageEventStream,
  createProvider,
  getCurrentSystemPrompt,
  getCurrentTools,
  type Message,
  type Model,
  type Provider,
  type SimpleStreamOptions,
  type TranscriptContext,
} from "@earendil-works/pi-ai";
import { zeroUsage } from "../types.js";

export const CLAUDE = "claude-cli";

export function claudeBinary(): string {
  if (process.env.FURB_CLAUDE_BIN) return process.env.FURB_CLAUDE_BIN;
  if (process.env.DIRT_CLI_BIN) return process.env.DIRT_CLI_BIN;
  const candidates = (process.env.PATH ?? "")
    .split(delimiter)
    .filter(Boolean)
    .map((path) => join(path, "claude"));
  candidates.push(join(homedir(), ".local/bin/claude"));
  if (process.platform === "darwin") {
    const desktop = join(homedir(), "Library/Application Support/Claude/claude-code");
    try {
      for (const version of readdirSync(desktop).sort((a, b) =>
        b.localeCompare(a, undefined, { numeric: true }),
      )) {
        candidates.push(join(desktop, version, "claude.app/Contents/MacOS/claude"));
      }
    } catch {
      /* The standalone CLI also works without Claude Desktop. */
    }
  }
  return (
    candidates.find((path) => {
      try {
        accessSync(path, constants.X_OK);
        return true;
      } catch {
        return false;
      }
    }) ?? "claude"
  );
}

export function cliModel(id: string): Model<Api> {
  return {
    id,
    name: id,
    api: CLAUDE,
    provider: CLAUDE,
    baseUrl: "",
    reasoning: true,
    thinkingLevelMap: {
      off: null,
      minimal: null,
      low: "low",
      medium: "medium",
      high: "high",
      xhigh: "xhigh",
      max: "max",
    },
    input: ["text", "image"],
    cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
    contextWindow: id.includes("haiku") ? 200000 : 1000000,
    maxTokens: id.includes("sonnet") || id.includes("haiku") ? 64000 : 128000,
  };
}

export function canonical(message: Message): string {
  if (typeof message.content === "string") return JSON.stringify([message.role, message.content]);
  return JSON.stringify([
    message.role,
    message.content.map((block) => {
      if (block.type === "thinking") return ["thinking"];
      if (block.type === "text") return ["text", block.text];
      if (block.type === "image") return ["image", block.mimeType, block.data];
      return ["tool", block.name, block.arguments];
    }),
  ]);
}

export function inputLine(messages: Message[]): string {
  const content: Record<string, unknown>[] = [];
  for (const [index, message] of messages.entries()) {
    if (message.role === "system") continue;
    if (message.role === "toolResult")
      throw new Error("The Claude completion provider takes no tool results.");
    const prefix =
      index === messages.length - 1 && message.role === "user"
        ? ""
        : `[earlier turn: ${message.role === "assistant" ? "you" : "user"}]\n`;
    if (prefix) content.push({ type: "text", text: prefix });
    const blocks =
      typeof message.content === "string"
        ? [{ type: "text" as const, text: message.content }]
        : message.content;
    for (const block of blocks) {
      if (block.type === "text") content.push({ type: "text", text: block.text });
      else if (block.type === "image")
        content.push({
          type: "image",
          source: { type: "base64", media_type: block.mimeType, data: block.data },
        });
      else if (block.type === "toolCall")
        throw new Error("The Claude completion provider takes no tool calls.");
    }
  }
  if (!content.length) throw new Error("There is no new message for Claude.");
  return `${JSON.stringify({ type: "user", message: { role: "user", content } })}\n`;
}

interface CliEvent {
  type?: string;
  result?: string;
  is_error?: boolean;
  total_cost_usd?: number;
  session_id?: string;
  event?: {
    type?: string;
    index?: number;
    content_block?: { type: string; text?: string; thinking?: string };
    delta?: { type?: string; text?: string; thinking?: string; signature?: string };
  };
  message?: { content?: { type: string; text?: string; thinking?: string; signature?: string }[] };
  usage?: {
    input_tokens?: number;
    output_tokens?: number;
    cache_read_input_tokens?: number;
    cache_creation_input_tokens?: number;
  };
}
interface Flight {
  stream: AssistantMessageEventStream;
  message: AssistantMessage;
  resolve(): void;
  reject(error: Error): void;
}

class Session {
  id: string = randomUUID();
  chain: string[] = [];
  requested: string[] = [];
  child?: ChildProcessWithoutNullStreams;
  flight?: Flight;
  used = Date.now();
  private buffer = "";
  private stderr = "";
  private cost = 0;
  private resumed = false;
  /** The place in the reply of each block the current message streams, by its index in the stream: a block of a
   * kind this provider does not read has none. Nothing streamed means the CLI sends each message whole. */
  private streamed?: Map<number, number>;
  /** How many blocks of the current message claude has settled, which is the stream index of the next one. */
  private settled = 0;
  private again = false;
  private heartbeat?: () => void;
  constructor(
    readonly model: Model<Api>,
    readonly system: string,
    readonly effort: string | undefined,
    readonly bin: string,
    private parent?: string,
    private readonly stallMs = 900000,
  ) {}

  async run(
    messages: Message[],
    options: SimpleStreamOptions,
    stream: AssistantMessageEventStream,
  ): Promise<void> {
    if (this.flight) throw new Error("This Claude conversation already has a request in flight.");
    if (options.signal?.aborted) throw new Error("The model request was cancelled.");
    const incoming = messages.map(canonical);
    const prefix = (held: string[]) =>
      held.length <= incoming.length && held.every((value, index) => value === incoming[index]);
    let start = prefix(this.chain) ? this.chain.length : prefix(this.requested) ? this.requested.length : -1;
    if (start < 0 || (start > 0 && start === incoming.length && this.again)) {
      this.stop();
      this.id = randomUUID();
      this.resumed = false;
      this.parent = undefined;
      this.chain = [];
      this.requested = [];
      this.again = false;
      start = 0;
    } else if (start > 0 && start === incoming.length) {
      this.again = true;
      start = incoming.length - 1;
    }
    const message: AssistantMessage = {
      role: "assistant",
      api: CLAUDE,
      provider: CLAUDE,
      model: this.model.id,
      content: [],
      usage: zeroUsage(),
      stopReason: "stop",
      timestamp: Date.now(),
    };
    this.streamed = undefined;
    this.settled = 0;
    let timer: ReturnType<typeof setTimeout> | undefined;
    let watchdog: ReturnType<typeof setTimeout> | undefined;
    let completed = false;
    this.heartbeat = () => {
      clearTimeout(watchdog);
      watchdog = setTimeout(
        () => this.stop(new Error(`Claude made no progress for ${this.stallMs / 1000}s.`)),
        this.stallMs,
      );
    };
    const abort = () => this.stop(new Error("The model request was cancelled."));
    try {
      await new Promise<void>((resolve, reject) => {
        this.flight = { stream, message, resolve, reject };
        this.heartbeat?.();
        stream.push({ type: "start", partial: message });
        options.signal?.addEventListener("abort", abort, { once: true });
        if (options.timeoutMs)
          timer = setTimeout(() => this.stop(new Error("The model request timed out.")), options.timeoutMs);
        try {
          if (!this.child) this.start();
          this.child?.stdin.write(inputLine(messages.slice(start)), (error) => {
            if (error) this.stop(error);
          });
        } catch (error) {
          this.stop(error instanceof Error ? error : new Error(String(error)));
        }
      });
      completed = true;
      stream.push({ type: "done", reason: "stop", message });
      stream.end();
    } finally {
      if (timer) clearTimeout(timer);
      clearTimeout(watchdog);
      this.heartbeat = undefined;
      this.requested = incoming;
      this.chain = completed ? [...incoming, canonical(message)] : incoming;
      if (completed) this.again = false;
      options.signal?.removeEventListener("abort", abort);
      this.flight = undefined;
      this.used = Date.now();
    }
  }

  private start(): void {
    const args = [
      "-p",
      "--input-format",
      "stream-json",
      "--output-format",
      "stream-json",
      "--verbose",
      "--include-partial-messages",
      "--model",
      this.model.id,
      "--system-prompt",
      this.system,
      "--setting-sources",
      "",
      "--strict-mcp-config",
      "--tools",
      "",
      "--disable-slash-commands",
    ];
    if (this.resumed) args.push("--resume", this.id);
    else if (this.parent) args.push("--resume", this.parent, "--fork-session");
    else args.push("--session-id", this.id);
    if (this.effort) args.push("--effort", this.effort);
    this.cost = 0;
    this.buffer = "";
    this.stderr = "";
    const child = spawn(this.bin, args, {
      stdio: "pipe",
      env: {
        ...process.env,
        CLAUDE_CODE_DISABLE_BUNDLED_SKILLS: "1",
        CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC: "1",
      },
    });
    this.child = child;
    this.resumed = true;
    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stderr.on("data", (text: string) => {
      this.stderr = (this.stderr + text).slice(-2000);
    });
    child.stdout.on("data", (chunk: string) => {
      if (child !== this.child) return;
      this.buffer += chunk;
      if (this.buffer.length > 32 * 1024 * 1024)
        return this.stop(new Error("Claude returned a stream line over 32 MiB."));
      for (;;) {
        const end = this.buffer.indexOf("\n");
        if (end < 0) break;
        const line = this.buffer.slice(0, end);
        this.buffer = this.buffer.slice(end + 1);
        if (!line.trim()) continue;
        try {
          this.event(JSON.parse(line) as CliEvent);
        } catch (error) {
          this.stop(error instanceof Error ? error : new Error(String(error)));
          break;
        }
      }
    });
    child.on("error", (error) => {
      if (child === this.child) this.stop(error);
    });
    child.on("close", (code) => {
      if (child === this.child) this.stop(new Error(`Claude exited (${code}): ${this.stderr.trim()}`));
    });
  }

  private event(event: CliEvent): void {
    const flight = this.flight;
    if (!flight) return;
    if (["stream_event", "assistant", "result"].includes(event.type ?? "")) this.heartbeat?.();
    if (event.type === "stream_event" && event.event) {
      const update = event.event;
      if (update.type === "message_start") {
        this.streamed = new Map();
        this.settled = 0;
      }
      this.streamed ??= new Map();
      const streamed = this.streamed;
      if (update.type === "content_block_start") {
        const block = update.content_block;
        const at = flight.message.content.length;
        if (block?.type === "text") {
          streamed.set(update.index ?? 0, at);
          flight.message.content.push({ type: "text", text: block.text ?? "" });
          flight.stream.push({ type: "text_start", contentIndex: at, partial: flight.message });
        } else if (block?.type === "thinking") {
          streamed.set(update.index ?? 0, at);
          flight.message.content.push({ type: "thinking", thinking: block.thinking ?? "" });
          flight.stream.push({ type: "thinking_start", contentIndex: at, partial: flight.message });
        }
      }
      const contentIndex = streamed.get(update.index ?? 0) ?? -1;
      const block = flight.message.content[contentIndex];
      if (update.type === "content_block_delta") {
        if (block?.type === "text" && update.delta?.type === "text_delta") {
          const delta = update.delta.text ?? "";
          block.text += delta;
          flight.stream.push({ type: "text_delta", contentIndex, delta, partial: flight.message });
        } else if (block?.type === "thinking" && update.delta?.type === "thinking_delta") {
          const delta = update.delta.thinking ?? "";
          block.thinking += delta;
          flight.stream.push({ type: "thinking_delta", contentIndex, delta, partial: flight.message });
        } else if (block?.type === "thinking" && update.delta?.type === "signature_delta") {
          block.thinkingSignature = (block.thinkingSignature ?? "") + (update.delta.signature ?? "");
        }
      } else if (update.type === "content_block_stop") {
        if (block?.type === "text")
          flight.stream.push({
            type: "text_end",
            contentIndex,
            content: block.text,
            partial: flight.message,
          });
        else if (block?.type === "thinking")
          flight.stream.push({
            type: "thinking_end",
            contentIndex,
            content: block.thinking,
            partial: flight.message,
          });
      }
    } else if (event.type === "assistant") {
      if (this.streamed) {
        // claude says each block of a streamed message again once it is whole, in an assistant event of its own and in
        // the order it streamed them: the whole block settles over the one streamed at its place, signature and all,
        // and is never appended a second time.
        for (const block of event.message?.content ?? []) {
          const contentIndex = this.streamed.get(this.settled++);
          if (contentIndex === undefined) continue;
          if (block.type === "text")
            flight.message.content[contentIndex] = { type: "text", text: block.text ?? "" };
          else if (block.type === "thinking")
            flight.message.content[contentIndex] = {
              type: "thinking",
              thinking: block.thinking ?? "",
              thinkingSignature: block.signature,
            };
        }
        return;
      }
      for (const block of event.message?.content ?? []) {
        const contentIndex = flight.message.content.length;
        if (block.type === "text") {
          flight.message.content.push({ type: "text", text: block.text ?? "" });
          flight.stream.push({ type: "text_start", contentIndex, partial: flight.message });
          flight.stream.push({
            type: "text_delta",
            contentIndex,
            delta: block.text ?? "",
            partial: flight.message,
          });
          flight.stream.push({
            type: "text_end",
            contentIndex,
            content: block.text ?? "",
            partial: flight.message,
          });
        } else if (block.type === "thinking") {
          flight.message.content.push({
            type: "thinking",
            thinking: block.thinking ?? "",
            thinkingSignature: block.signature,
          });
          flight.stream.push({ type: "thinking_start", contentIndex, partial: flight.message });
          flight.stream.push({
            type: "thinking_delta",
            contentIndex,
            delta: block.thinking ?? "",
            partial: flight.message,
          });
          flight.stream.push({
            type: "thinking_end",
            contentIndex,
            content: block.thinking ?? "",
            partial: flight.message,
          });
        }
      }
    } else if (event.type === "result") {
      if (event.session_id) this.id = event.session_id;
      if (event.is_error) {
        this.stop(new Error(event.result || "Claude reported an error."));
        return;
      }
      const usage = event.usage ?? {};
      const paid = event.total_cost_usd ?? this.cost;
      flight.message.usage = {
        input: usage.input_tokens ?? 0,
        output: usage.output_tokens ?? 0,
        cacheRead: usage.cache_read_input_tokens ?? 0,
        cacheWrite: usage.cache_creation_input_tokens ?? 0,
        totalTokens:
          (usage.input_tokens ?? 0) +
          (usage.output_tokens ?? 0) +
          (usage.cache_read_input_tokens ?? 0) +
          (usage.cache_creation_input_tokens ?? 0),
        cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, total: Math.max(0, paid - this.cost) },
      };
      this.cost = paid;
      if (!flight.message.content.length && event.result)
        flight.message.content.push({ type: "text", text: event.result });
      flight.resolve();
    }
  }

  stop(error = new Error("The Claude session was stopped.")): void {
    const child = this.child;
    this.child = undefined;
    child?.kill("SIGKILL");
    this.flight?.reject(error);
  }
}

export interface ClaudeOptions {
  bin?: string;
  maxWarm?: number;
  maxSessions?: number;
  idleMs?: number;
  stallMs?: number;
}
export function claudeProvider(options: ClaudeOptions = {}): { provider: Provider; dispose(): void } {
  const sessions = new Map<string, Session>();
  const sweep = () => {
    const idle = [...sessions.entries()]
      .filter(([, session]) => !session.flight)
      .sort((a, b) => a[1].used - b[1].used);
    let warm = [...sessions.values()].filter((session) => session.child).length;
    for (const [key, session] of idle) {
      if (
        session.child &&
        (warm > (options.maxWarm ?? 8) || Date.now() - session.used > (options.idleMs ?? 300000))
      ) {
        session.stop();
        warm--;
      }
      if (!session.child && sessions.size > (options.maxSessions ?? 64)) sessions.delete(key);
    }
  };
  const reaper = setInterval(sweep, options.idleMs ?? 300000);
  reaper.unref();
  const stream = (model: Model<Api>, context: TranscriptContext, settings: SimpleStreamOptions = {}) => {
    const output = createAssistantMessageEventStream();
    void (async () => {
      try {
        if (getCurrentTools(context.messages).length)
          throw new Error("furb's Claude provider accepts pure completions only.");
        const system = getCurrentSystemPrompt(context.messages);
        const mapped = settings.reasoning ? model.thinkingLevelMap?.[settings.reasoning] : undefined;
        if (mapped === null) throw new Error(`${model.id} does not offer ${settings.reasoning}.`);
        const effort = mapped ?? settings.reasoning;
        const key = JSON.stringify([model.id, effort, system, settings.sessionId ?? randomUUID()]);
        let session = sessions.get(key);
        if (!session) {
          const incoming = context.messages.filter((message) => message.role !== "system");
          const chain = incoming.map(canonical);
          const donor = [...sessions.values()]
            .filter(
              (held) =>
                !held.flight &&
                held.chain.length > 0 &&
                held.model.id === model.id &&
                held.system === system &&
                held.chain.length < chain.length &&
                held.chain.every((message, index) => message === chain[index]) &&
                incoming.slice(held.chain.length).every((message) => message.role !== "assistant"),
            )
            .sort((a, b) => b.chain.length - a.chain.length)[0];
          session = new Session(
            model,
            system,
            effort,
            options.bin ?? claudeBinary(),
            donor?.id,
            options.stallMs,
          );
          if (donor) session.chain = [...donor.chain];
          sessions.set(key, session);
        }
        await session.run(
          context.messages.filter((message) => message.role !== "system"),
          settings,
          output,
        );
      } catch (error) {
        const message: AssistantMessage = {
          role: "assistant",
          api: CLAUDE,
          provider: CLAUDE,
          model: model.id,
          content: [],
          usage: zeroUsage(),
          stopReason: settings.signal?.aborted ? "aborted" : "error",
          errorMessage: error instanceof Error ? error.message : String(error),
          timestamp: Date.now(),
        };
        output.push({ type: "error", reason: message.stopReason as "error" | "aborted", error: message });
        output.end();
      } finally {
        sweep();
      }
    })();
    return output;
  };
  return {
    provider: createProvider({
      id: CLAUDE,
      name: "Claude CLI",
      models: ["sonnet", "opus", "haiku", "fable"].map(cliModel),
      auth: { apiKey: { name: "Claude subscription", resolve: async () => ({ auth: {} }) } },
      api: { stream, streamSimple: stream },
    }),
    dispose() {
      clearInterval(reaper);
      for (const session of sessions.values()) session.stop();
      sessions.clear();
    },
  };
}
