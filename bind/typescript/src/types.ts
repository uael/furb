import { stripVTControlCharacters } from "node:util";
import type { ModelThinkingLevel, Usage as ModelUsage } from "@earendil-works/pi-ai";
import type { Life } from "../index.cjs";

export const efforts = [
  "off",
  "minimal",
  "low",
  "medium",
  "high",
  "xhigh",
  "max",
] as const satisfies readonly ModelThinkingLevel[];
export function actorParts(actor: string, models: readonly string[] = []): { model: string; effort: string } {
  const at = actor.lastIndexOf("/");
  return at < 0 || models.includes(actor) || !efforts.some((effort) => effort === actor.slice(at + 1))
    ? { model: actor, effort: "off" }
    : { model: actor.slice(0, at), effort: actor.slice(at + 1) };
}
/** The one model among models that an actor names, with or without its effort: its provider and id as
 * `provider:id`, or its id alone. */
export function modelNamed<M extends { provider: string; id: string }>(
  models: readonly M[],
  actor: string,
): M | undefined {
  for (const name of [actor, actorParts(actor).model]) {
    const found = models.filter((model) => `${model.provider}:${model.id}` === name || model.id === name);
    if (found.length === 1) return found[0];
  }
  return undefined;
}
export const shapes = ["str", "None", "bool", "int", "float", "list", "dict"] as const;

export type Turn = Awaited<ReturnType<Life["turns"]>>[number];
export type Tag = Exclude<Turn[1][number], string>;
export type Fact = Awaited<ReturnType<Life["send"]>>;
export type Entry = [string, Fact, unknown?];
export type Usage = NonNullable<Turn[2]>;
export interface TextValue {
  is?: "Text";
  path: string;
  content: string;
}
export interface OperatorPrompt {
  id: string;
  shape: string;
  message: string;
  resolve(value: unknown): void;
  reject(error: Error): void;
}

export function isTag(value: unknown): value is Tag {
  return (
    Array.isArray(value) && value.length === 3 && typeof value[0] === "string" && Array.isArray(value[1])
  );
}
export function display(value: unknown): string {
  if (typeof value === "string") return value;
  return JSON.stringify(value, null, 2) ?? "None";
}
export function safeText(value: string): string {
  // Text from files and processes must not become terminal control sequences.
  // biome-ignore lint/suspicious/noControlCharactersInRegex: Remove terminal control bytes from displayed text.
  return stripVTControlCharacters(value).replace(/[\u0000-\u0008\u000b-\u001f\u007f-\u009f]/g, "");
}
export const zeroUsage = (): ModelUsage => ({
  input: 0,
  output: 0,
  cacheRead: 0,
  cacheWrite: 0,
  totalTokens: 0,
  cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, total: 0 },
});
