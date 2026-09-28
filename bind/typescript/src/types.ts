import { stripVTControlCharacters } from "node:util";
import { type Engine, levels } from "../index.cjs";

/** The levels of effort, from least to most, which an actor names after its model. */
export const efforts: readonly string[] = levels();
/** The model and the effort an actor names: a model alone, whose effort is `off`, or a model and a level after its
 * last slash, when the models named do not hold the whole name. */
export function actorParts(actor: string, models: readonly string[] = []): { model: string; effort: string } {
  const at = actor.lastIndexOf("/");
  return at < 0 || models.includes(actor) || !efforts.some((effort) => effort === actor.slice(at + 1))
    ? { model: actor, effort: "off" }
    : { model: actor.slice(0, at), effort: actor.slice(at + 1) };
}

export type Turn = ReturnType<Engine["turns"]>[number];
export type Fact = ReturnType<Engine["say"]>;
/** One entry of the record: one fact, an act among them. */
export type Entry = [Fact];
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
/** The lines of a paragraph as their text: a comment without its mark, a quote as the text between its two marks,
 * and python as it stands. The close mark of a quote is the first line after its open mark that ends with it, as the
 * engine reads one. */
export function uncommented(lines: readonly string[]): string {
  const text: string[] = [];
  for (let at = 0; at < lines.length; at++) {
    const line = lines[at] ?? "";
    const name = /^<s:(\w+)>$/.exec(line)?.[1];
    let close = -1;
    for (let end = at + 1; name && end < lines.length && close < 0; end++)
      if (lines[end]?.endsWith(`</s:${name}>`)) close = end;
    if (name && close > at) {
      const quoted = lines.slice(at + 1, close + 1).join("\n");
      text.push(quoted.slice(0, -`</s:${name}>`.length).replace(/\n$/, ""));
      at = close;
    } else text.push(line === "#" ? "" : line.startsWith("# ") ? line.slice(2) : line);
  }
  return text.join("\n");
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
export function display(value: unknown): string {
  if (typeof value === "string") return value;
  return JSON.stringify(unmarked(value), null, 2) ?? "None";
}
export function safeText(value: string): string {
  // Text from files and processes must not become terminal control sequences.
  // biome-ignore lint/suspicious/noControlCharactersInRegex: Remove terminal control bytes from displayed text.
  return stripVTControlCharacters(value).replace(/[\u0000-\u0008\u000b-\u001f\u007f-\u009f]/g, "");
}
