import { type Engine, levels, leveled as read } from "../index.cjs";
import { unmarked } from "./life/marks.js";

export { unmarked };

/** The levels of effort, from least to most, which an actor names after its model. */
export const efforts: readonly string[] = levels();
/** The model and the effort an actor names, as the crate reads an actor: a model alone, whose effort is `off`, or a
 * model and a level after its last slash, when the models named do not hold the whole name. */
export const leveled = (actor: string, models: readonly string[] = []): [model: string, effort: string] => {
  const [model = actor, effort = "off"] = read(actor, [...models]);
  return [model, effort];
};

/** One actor of the roster, as the World offers it: its name, its efforts and its window. A record of a build before
 * the doctrine was read holds the doctrine after the window. */
export type Actor = [name: string, efforts: string[], window: number, doctrine?: string];
/** What the chains stand on, as standing gives it: the roster, the directory, the default actor and the path of the
 * doctrine that each chain reads, which a record of a build before it leaves unsaid, or nothing before the first
 * stand. */
export type Standing = [roster: Actor[], directory: string, actor: string, doctrine?: string] | [];
/** The standing of an engine, in the one shape that every reader of it takes. */
export function standingOf(engine: Pick<Engine, "standing">): Standing {
  return engine.standing() as Standing;
}
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

/** What one fact that tells stands as in a user turn: its header, `#` and the name of the act it is of, then the
 * words that say what happened to the act; and the lines after the header, which are python: a binding of each value
 * that the act tells, and the binding of the act on its open. */
export interface Paragraph {
  name: string;
  words: string;
  lines: string[];
  text: string;
}
/** Whether a paragraph is the open of the act it is of: an act tells its binding as the last line of its open,
 * and no other paragraph ends with it. */
export function opens(paragraph: Paragraph): boolean {
  return paragraph.lines.at(-1)?.startsWith(`${paragraph.name}: Act[`) ?? false;
}
/** The lines of a paragraph as a reader sees them: a quote as the text between its two marks, and any other line as
 * it stands. The close mark of a quote is the first line after its open mark that ends with it, as the engine reads
 * one. */
export function plain(lines: readonly string[]): string {
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
    } else text.push(line);
  }
  return text.join("\n");
}
/** Whether a name is the name of a question of that kind: the kind and a number, as every act is named. */
export function isQuestion(kind: string, id: string): boolean {
  return id.startsWith(kind) && /^\d+$/.test(id.slice(kind.length));
}
/** The usage of an answer: the tokens that it read, the tokens that it wrote, the tokens read from and written to the
 * cache, and its dollars. A float that the life marked, as 0.0, is the number it stands for, and a part that no record
 * gave, or that is no finite number, counts as nothing, so that no sum of usages is ever NaN. */
export function usageOf(usage: readonly unknown[]): [number, number, number, number, number] {
  const part = (index: number) => {
    const one = Number(unmarked(usage[index]));
    return Number.isFinite(one) ? one : 0;
  };
  return [part(0), part(1), part(2), part(3), part(4)];
}
export function display(value: unknown): string {
  if (typeof value === "string") return value;
  return JSON.stringify(unmarked(value), null, 2) ?? "None";
}
