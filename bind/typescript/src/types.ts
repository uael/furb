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
export type Fact = Awaited<ReturnType<Life["say"]>>;
/** One entry of the record: one fact, an act among them. */
export type Entry = [Fact];
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

/** What one fact that tells stands as in a user turn: its header, `#` and the name of the act it is of, or the kind
 * of a query, then its words; and the lines after the header. */
export interface Paragraph {
  name: string;
  words: string;
  lines: string[];
  text: string;
}
/** The paragraphs of the python of a user turn, in order. A blank line that a header follows is where one paragraph
 * ends, since a word its caller wrote may hold a blank line of its own. */
export function paragraphs(python: string): Paragraph[] {
  if (!python) return [];
  return python.split(/\n\n(?=#\S)/).map((text) => {
    const [header = "", ...lines] = text.split("\n");
    const space = header.indexOf(" ");
    return {
      name: header.slice(1, space < 0 ? undefined : space),
      words: space < 0 ? "" : header.slice(space + 1),
      lines,
      text,
    };
  });
}
/** Whether a paragraph is the open of the act it is of: an act tells its binding as the last line of its open,
 * and no other paragraph ends with it. */
export function opens(paragraph: Paragraph): boolean {
  return paragraph.lines.at(-1)?.startsWith(`${paragraph.name}: Act[`) ?? false;
}
/** The lines of a paragraph as their text: a comment without its mark, and python as it stands. */
export function uncommented(lines: readonly string[]): string {
  return lines.map((line) => (line === "#" ? "" : line.startsWith("# ") ? line.slice(2) : line)).join("\n");
}
/** Whether a name is the name of a question of that kind: the kind and a number, as every act is named. */
export function isQuestion(kind: string, id: string): boolean {
  return id.startsWith(kind) && /^\d+$/.test(id.slice(kind.length));
}
/** The kind of a question, from its name, which is its kind and a number. */
export function questionKind(id: string): string | undefined {
  return id.match(/^(\w+?)\d+$/)?.[1];
}
/** A map as JavaScript holds it again: a map the life marked as its pairs, since it holds the key `is`, whose keys
 * are all strings becomes that map, and its entries are read the same way. */
export function unmarked(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(unmarked);
  if (!value || typeof value !== "object") return value;
  const held = value as Record<string, unknown>;
  const pairs = held.is === "dict" && Array.isArray(held.args) ? held.args[0] : undefined;
  if (Array.isArray(pairs) && pairs.every((pair) => Array.isArray(pair) && typeof pair[0] === "string"))
    return Object.fromEntries(pairs.map(([key, one]) => [key, unmarked(one)]));
  return Object.fromEntries(Object.entries(held).map(([key, one]) => [key, unmarked(one)]));
}
/** A plain value of the host as the life takes it: each map that holds the key `is` crosses as its pairs, under the
 * mark dict, so no map of the host reads as a mark. `decoded` is the same value as the native reader gave it, whose
 * numbers it keeps. */
export function marked(plain: unknown, decoded: unknown = plain): unknown {
  if (Array.isArray(plain)) return plain.map((one, index) => marked(one, (decoded as unknown[])[index]));
  if (!plain || typeof plain !== "object") return decoded;
  const pairs = Object.entries(plain).map(
    ([key, one]) => [key, marked(one, (decoded as Record<string, unknown>)[key])] as const,
  );
  return "is" in plain ? { is: "dict", args: [pairs] } : Object.fromEntries(pairs);
}
export function display(value: unknown): string {
  if (typeof value === "string") return value;
  return JSON.stringify(unmarked(value), null, 2) ?? "None";
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
