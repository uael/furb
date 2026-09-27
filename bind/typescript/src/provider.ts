import { randomUUID } from "node:crypto";
import { readFileSync } from "node:fs";
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
import type { Engine } from "../index.cjs";
import { type Ear, isFault, over, speaking } from "./ears.js";
import { ImageCache, turnImages } from "./images.js";
import { actorParts, type Fact, float, isQuestion, modelNamed, type Turn, zeroUsage } from "./types.js";

let prompt: string | undefined;
/** The system prompt of every model: the engine minified in layout alone, which `bun run build` writes beside the
 * package, read once. */
function system(): string {
  prompt ??= JSON.parse(readFileSync(new URL("../system.json", import.meta.url), "utf8")) as string;
  return prompt;
}

/** An actor the provider offers: the name of a model, the efforts it takes, and the window it reads. */
type Actor = [name: string, efforts: string[], window: number];

/** The model request a test or another host puts in the place of a real one. It may tell the text and the thinking
 * that it writes as it writes them, which the provider streams as a model's. */
export type Answer = (
  actor: string,
  chain: string,
  turns: Turn[],
  signal: AbortSignal,
  write: (delta: { text?: string; thinking?: string }) => void,
) => Promise<Turn>;

export interface ProviderOptions {
  /** The directory the life stands on, which each chain stands in until it goes elsewhere. */
  directory: string;
  models: Models;
  /** The model a prompt goes to when it names none; none puts every prompt to the operator. */
  model?: string;
  effort?: ModelThinkingLevel;
  roster?: string[];
  answer?: Answer;
  /** The directory of the images a turn names. */
  imageDirectory: string;
  /** Whether it may ask no model, as an inspection of a record may not. */
  readOnly?: boolean;
  /** Told when what a model writes changes. */
  changed?: () => void;
  /** Told what the life refused when the provider said what a reply came to. */
  fault?: (error: unknown) => void;
}

/** The provider of models, as an ear: it answers what a chain stands on and takes a reply, which it answers with the
 * turn of a model of pi-ai. A reply that fails is answered with a refusal, and a second in a row on a chain of the
 * same actor pauses the chain. */
export class Provider {
  readonly directory: string;
  readonly models: Models;
  /** The model a prompt goes to when it names none, and none when the provider offers the operator alone. */
  readonly model?: string;
  readonly effort: ModelThinkingLevel;
  readonly roster: string[];
  /** What each rung that asked a model streams while the model writes, by that rung. */
  readonly streams = new Map<string, { chain: string; text: string; thinking: string }>();
  /** The engine whose replies it answers, which its later work speaks into. */
  engine?: Engine;
  private readonly actors: Actor[];
  private readonly options: ProviderOptions;
  /** The model request of each reply the provider takes, which the done of that reply ends, whoever says it. */
  private readonly replies = new Map<string, AbortController>();
  /** The actor whose last reply on each chain answered nothing, so a second in a row pauses the chain. */
  private readonly mute = new Map<string, string>();
  private readonly controller = new AbortController();
  /** Keys the conversations of this life at a provider, since the ids of chains repeat in every life. */
  private readonly conversations = randomUUID();
  /** The content of each image a turn names, read once while its file stays the same. */
  private readonly images = new ImageCache();

  constructor(options: ProviderOptions) {
    this.options = options;
    this.directory = options.directory;
    const roster = options.roster ?? [];
    this.models = options.models;
    this.model = options.model;
    this.roster = [...new Set([this.model, ...roster])].filter((name) => name !== undefined);
    this.actors = this.roster.map((name): Actor => {
      const model = this.route(name);
      return [name, getSupportedThinkingLevels(model), model.contextWindow];
    });
    const model = this.model ? this.offers(this.model) : undefined;
    this.effort = model ? clampThinkingLevel(model, options.effort ?? "low") : (options.effort ?? "low");
    // A provider that offers no model puts every prompt to the operator, so nothing would ever reach `answer`.
    if (options.answer && !this.roster.length)
      throw new Error("An answer replaces the request of a model, and this provider offers no model.");
  }

  /** The model an actor names, with or without its effort, and nothing when the provider holds no such model. */
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

  *ear(): Ear {
    for (;;) {
      const fact = (yield null) as Fact | undefined;
      if (!fact) continue;
      const [kind, id, by, ...words] = fact;
      if (kind === "stand" && isQuestion(kind, id))
        yield ["done", id, [[...this.actors, ["operator", [], 200000]], this.directory, this.actor]];
      else if (kind === "reply" && isQuestion(kind, id)) {
        yield ["started", id];
        const [chain, actor] = [String(words[0]), String(words[1])];
        const turns = (yield { verb: "turns", kwargs: { on: chain } }) as Turn[];
        queueMicrotask(() =>
          this.answered(id, by, chain, actor, turns).catch((error) => this.options.fault?.(error)),
        );
      } else if (kind === "done") {
        this.replies.get(id)?.abort();
        this.replies.delete(id);
      }
    }
  }

  /** What a reply came to, said later as the provider: the turn of the model, or the refusal of a model that answered
   * nothing, whose second in a row on a chain pauses the chain. */
  private async answered(
    id: string,
    rung: string,
    chain: string,
    actor: string,
    turns: Turn[],
  ): Promise<void> {
    const engine = this.engine;
    if (!engine || over(engine, id)) return;
    let said: unknown;
    try {
      const turn = await this.reply(id, rung, chain, actor, turns);
      this.mute.delete(chain);
      // A cost that is whole stays a float for python.
      said = Array.isArray(turn[2])
        ? [turn[0], turn[1], turn[2].map((part, at) => (at === 4 ? float(part) : part)), turn[3]]
        : turn;
    } catch (error) {
      if (over(engine, id)) return;
      if (this.mute.get(chain) === actor) speaking(engine, "provider", () => engine.pause(chain));
      this.mute.set(chain, actor);
      const why =
        error instanceof Error
          ? `${error.name}: ${error.message}`
          : isFault(error)
            ? `${error.is}: ${error.args.map(String).join(", ")}`
            : `Refused: ${String(error)}`;
      said = { is: "Refused", args: [`${actor} answered nothing: ${why}`] };
    }
    if (!over(engine, id)) speaking(engine, "provider", () => engine.say("done", id, [said]));
  }

  /** One turn of a model for a reply, streamed under the rung the reply asks for; the done of the reply ends the
   * request. */
  private async reply(id: string, rung: string, chain: string, actor: string, turns: Turn[]): Promise<Turn> {
    if (this.options.readOnly) throw new Error("Record inspection cannot ask a model.");
    const controller = new AbortController();
    this.replies.set(id, controller);
    const signal = AbortSignal.any([this.controller.signal, controller.signal]);
    this.streams.set(rung, { chain, text: "", thinking: "" });
    this.options.changed?.();
    try {
      if (this.options.answer)
        return await this.options.answer(actor, chain, turns, signal, ({ text = "", thinking = "" }) => {
          const held = this.streams.get(rung);
          if (!held) return;
          held.text += text;
          held.thinking += thinking;
          this.options.changed?.();
        });
      const model = this.route(actor);
      // The engine phrases every turn as python, so the provider renders nothing: a user turn goes as the python the
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
          const found = turnImages(this.options.imageDirectory, python, this.images);
          if (found.length && !model.input.includes("image"))
            throw new Error(`${model.name} does not accept images.`);
          return [
            {
              role,
              content: found.length ? [{ type: "text", text: python }, ...found] : python,
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
      const effort = actorParts(actor, this.roster).effort;
      const stream = this.models.streamSimple(
        model,
        { systemPrompt: system(), messages },
        {
          signal,
          sessionId: `${this.conversations}/${chain}`,
          reasoning: effort === "off" ? undefined : (effort as ThinkingLevel),
        },
      );
      for await (const event of stream) {
        const held = this.streams.get(rung);
        if (held && event.type === "text_delta") held.text += event.delta;
        if (held && event.type === "thinking_delta") held.thinking += event.delta;
        this.options.changed?.();
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
      this.replies.delete(id);
      this.streams.delete(rung);
      this.options.changed?.();
    }
  }

  /** Every request it makes ends, and the images it read go. */
  dispose(): void {
    this.controller.abort();
    this.images.clear();
  }
}
