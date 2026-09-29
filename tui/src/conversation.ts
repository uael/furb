import {
  bound,
  OPERATOR,
  opens,
  type Paragraph,
  paragraphs,
  plain,
  questionKind,
  type Turn,
} from "@furb/engine";
import { type ActRow, failed } from "./session.ts";

/** What an act told of itself after its open, or what a query of a run told, by its header: the word of the header
 * after the name, or the kind of the act when the header says nothing more, the rest of the header, and the lines. */
export type Note = { key: string; label: string; detail: string; body: string; act?: ActRow };

/** One thing the conversation shows, read off the python of the turns of a chain.
 *
 * - `word`: a word, which an assistant turn holds, or which the open of a rung its caller wrote binds as rungN_word,
 *   with the rung it is the word of, and what came of it in the order the chain told it: each act that the word made,
 *   at its first paragraph, and what those acts and the queries of its run told.
 * - `thread`: the open of a thread, which tells its markdown.
 * - `result`: the close of a thread, and whether other threads closed in the same turn.
 * - `act`: an act that no word made, as a command of the operator, with what it told of itself.
 * - `note`: any other paragraph that no word holds.
 */
export type Item =
  | { type: "word"; key: string; code: string; rung?: ActRow; told: (ActRow | Note)[] }
  | { type: "thread"; key: string; act: ActRow }
  | { type: "result"; key: string; act: ActRow; parallel: boolean }
  | { type: "act"; key: string; act: ActRow; notes: Note[] }
  | ({ type: "note" } & Note);

type Word = Extract<Item, { type: "word" }>;
/** The acts that a word made, in the order it made them. */
export const actsOf = (word: Word): ActRow[] => word.told.filter((one): one is ActRow => "id" in one);
/** What a word and its acts told, in order. */
export const notesOf = (word: Word): Note[] => word.told.filter((one): one is Note => !("id" in one));

/** The headers that tell how an act ended, which the card of the act shows as its state. */
const ENDS = ["closed", "exited", "cancelled"];

/** Whether an act is a question to the operator: a thread whose actor is the operator. */
export const asksOperator = (act: ActRow) => act.kind === "thread" && act.words[2] === OPERATOR;

/** The comments of a word, in order, each with the index of its line: each line that is `#`, a space and text, which
 * says a step as one line of markdown. A quote is text, from `<s:name>` at the start of a line to `</s:name>` at the
 * end of a line, so a heading of markdown that it holds is no step. */
function stepLines(word: string): [line: number, text: string][] {
  const said: [number, string][] = [];
  let quote = "";
  for (const [index, line] of word.split("\n").entries()) {
    const opened = quote ? undefined : /^<s:(\w+)>/.exec(line)?.[1];
    if (opened) quote = opened;
    else if (!quote)
      said.push(
        ...(/^\s*# (.*\S.*)$/.exec(line)?.slice(1) ?? []).map((text): [number, string] => [index, text]),
      );
    if (quote && line.endsWith(`</s:${quote}>`)) quote = "";
  }
  return said;
}

/** The steps of a word, in order. */
export const steps = (word: string): string[] => stepLines(word).map(([, text]) => text);

/** The step under which each command of a word stands, in the order the word ran them: the last step above the line
 * that holds the command, from the line of the command before it on. A line holds a command that it writes as a
 * string, or that it builds, when it calls bash with anything but a plain string, as a loop or an f-string does. */
export function stepsOf(word: string, commands: readonly string[]): number[] {
  const lines = word.split("\n");
  const at = stepLines(word).map(([line]) => line);
  const built = (text: string) => /\bbash\((?!\s*["'])/.test(text);
  let from = 0;
  return commands.map((command) => {
    const written = [command, JSON.stringify(command).slice(1, -1)].flatMap((one) => [`"${one}`, `'${one}`]);
    const holds = (text: string) => written.some((one) => text.includes(one));
    const line = [holds, built]
      .map((rule) => lines.findIndex((text, index) => index >= from && rule(text)))
      .find((one) => one >= 0);
    if (line !== undefined) from = line;
    return Math.max(
      0,
      at.findLastIndex((step) => step <= from),
    );
  });
}

/** Whether a word is a note: it holds a comment, and no line of it is anything but a comment or blank. */
export function isNote(word: string): boolean {
  const lines = word.split("\n").filter((line) => line.trim());
  return lines.length > 0 && lines.every((line) => /^\s*#( |$)/.test(line)) && steps(word).length > 0;
}

/** Whether an act is a note of the operator: a rung of the operator whose word is only comments. */
export const operatorNote = (act: ActRow) =>
  act.kind === "rung" && act.by === OPERATOR && isNote(String(act.words[0] ?? ""));

/** What the conversation of a chain shows, in the order of its turns, with the act that asked each step, such as a
 * read, which is no act of its own. */
export function conversation(
  turns: readonly Turn[],
  acts: readonly ActRow[],
  asked: ReadonlyMap<string, string>,
): Item[] {
  const rows = new Map(acts.map((act) => [act.id, act]));
  const items: Item[] = [];
  const seen = new Set<string>();
  /** The item of each word by its rung, and of each act that no word made, which take what their acts tell. */
  const words = new Map<string, Word>();
  const owned = new Map<string, Extract<Item, { type: "act" }>>();
  let last: Word | undefined;
  let rung: ActRow | undefined;
  const word = (key: string, code: string, of?: ActRow) => {
    const item: Word = { type: "word", key, code, rung: of, told: [] };
    items.push(item);
    if (of) words.set(of.id, item);
    last = item;
  };
  for (const [index, [role, python]] of turns.entries()) {
    if (role === "assistant") {
      if (python) word(`turn-${index}`, python, rung);
      if (rung) seen.add(rung.id);
      continue;
    }
    const told = paragraphs(python);
    const closes = told.filter(
      (paragraph) => rows.get(paragraph.name)?.kind === "thread" && paragraph.words.startsWith("closed"),
    ).length;
    for (const [part, paragraph] of told.entries()) {
      const key = `turn-${index}-${part}`;
      const act = rows.get(paragraph.name);
      const asker = rows.get(asked.get(paragraph.name) ?? "");
      const source = act ?? asker;
      if (source && !fromOperator(source, rows)) continue;
      const [head = "", ...rest] = paragraph.words.split(" ");
      // A note goes to the act that no word made, to the word of its rung, to the word that made its act or asked its
      // step, or, for a paragraph whose maker the life does not say, to the word that ran last.
      const note = () => {
        const one = noted(key, paragraph, act, head, rest.join(" "));
        const holder = act
          ? (owned.get(act.id) ?? words.get(act.id) ?? words.get(act.by))
          : asked.has(paragraph.name)
            ? words.get(asker?.id ?? "")
            : last;
        if (holder?.type === "word") holder.told.push(one);
        else if (holder) holder.notes.push(one);
        else items.push({ type: "note", ...one });
      };
      if (head === "ledger") continue;
      if (act?.kind === "rung") {
        if (writes(act, paragraph)) {
          rung = act;
          seen.add(act.id);
          word(key, String(act.words[0]), act);
        } else if (head === "advance") rung = act;
        else if (head === "closed") {
          if (failed(act) && !seen.has(act.id)) {
            const item: Extract<Item, { type: "act" }> = { type: "act", key: act.id, act, notes: [] };
            items.push(item);
            owned.set(act.id, item);
          }
          seen.add(act.id);
        } else if (!(["raised", "refused"].includes(head) && words.has(act.id))) note();
      } else if (act?.kind === "thread") {
        if (opens(paragraph)) items.push({ type: "thread", key, act });
        else if (head === "closed") items.push({ type: "result", key, act, parallel: closes > 1 });
        else note();
      } else if (act && ["chain", "grant"].includes(act.kind)) {
        // The inspector shows what the chain stands on, so its standing, headed by its roster, is no card of the
        // conversation.
        if (!opens(paragraph) && !ENDS.includes(head) && head !== "standing") note();
      } else if (act && (opens(paragraph) || ENDS.includes(head))) {
        if (!seen.has(act.id)) {
          const maker = words.get(act.by);
          if (maker) maker.told.push(act);
          else {
            const item: Extract<Item, { type: "act" }> = { type: "act", key: act.id, act, notes: [] };
            items.push(item);
            owned.set(act.id, item);
          }
        }
        seen.add(act.id);
        // What an act told as it ended, as the output of a command, is a note of the word that made it.
        if (ENDS.includes(head)) note();
      } else note();
    }
  }
  return items;
}

/** Whether an act comes from the operator: the conversation is what the operator started and what came of it, and an
 * act whose makers lead to another ear of the outside, as the word of an extension and each act that word makes, is
 * none of it. */
export function fromOperator(act: ActRow, rows: ReadonlyMap<string, ActRow>): boolean {
  let at = act;
  for (let maker = rows.get(at.by); maker; maker = rows.get(at.by)) at = maker;
  return at.by === OPERATOR;
}

/** Whether a paragraph is the open of a rung its caller wrote, which binds that word as rungN_word under a header
 * that says nothing more. */
function writes(act: ActRow, paragraph: Paragraph): boolean {
  return (
    Boolean(act.words[0]) && !paragraph.words && bound(paragraph.lines, `${act.id}_word`) === act.words[0]
  );
}

/** A paragraph as a note: the word of its header after the name of its act, or the kind of that act when its header
 * says nothing more, as its label. The feed names no act, so each value that the paragraph binds as a statement under
 * the name of its act stands as the value alone: the path of a read says what the read was about, and the others say
 * what the act told, beside the texts that it quotes. */
function noted(key: string, paragraph: Paragraph, act: ActRow | undefined, head: string, rest: string): Note {
  const { name } = paragraph;
  const statement = new RegExp(`^${name}_(\\w+) = (.*)$`);
  const lines = paragraph.lines.flatMap((line) => {
    const [, word, value] = statement.exec(line) ?? [];
    if (word === undefined) return [line];
    return word === "path" ? [] : [bound([line], `${name}_${word}`) ?? value ?? ""];
  });
  return {
    key,
    label: head || (act?.kind ?? questionKind(name) ?? name),
    detail: rest || bound(paragraph.lines, `${name}_path`) || "",
    body: plain(lines),
    ...(act ? { act } : {}),
  };
}

/** The findings that refused the word of a rung, one for each line that the paragraph its chain told of it binds. */
export function refusal(turns: readonly Turn[], rung: string): string[] {
  const told = turns
    .flatMap(([role, python]) => (role === "user" ? paragraphs(python) : []))
    .find((paragraph) => paragraph.name === rung && paragraph.words === "refused");
  return told ? (bound(told.lines, `${rung}_findings`) ?? "").split("\n") : [];
}

/** The threads of a life and what stands under each. A thread stands in the list of its chain, but a question to the
 * operator that a thread of its chain holds, which stands in that thread. An act stands under the nearest listed
 * thread of its chain among the acts that made it. A note of the operator, a rung of the operator whose word is only
 * comments, stands under the thread whose model reads it: the thread of the first rung of a model on its chain after
 * it, or while no such rung stands, the last thread of its chain that a model works. */
export class Threads {
  private readonly rows: Map<string, ActRow>;
  private readonly order: Map<string, number>;
  private readonly held = new Map<string, string | undefined>();
  constructor(readonly acts: readonly ActRow[]) {
    this.rows = new Map(acts.map((act) => [act.id, act]));
    this.order = new Map(acts.map((act, index) => [act.id, index]));
  }
  /** The nearest listed thread of the chain of an act among the acts that made it, or the act when it is one. */
  of(act?: ActRow): string | undefined {
    if (!act) return undefined;
    if (this.held.has(act.id)) return this.held.get(act.id);
    let holder: string | undefined;
    if (operatorNote(act)) holder = this.holderOfNote(act);
    else
      for (let at: ActRow | undefined = act; at && holder === undefined; at = this.rows.get(at.by))
        if (at.kind === "thread" && at.on === act.on && this.listed(at)) holder = at.id;
    this.held.set(act.id, holder);
    return holder;
  }
  /** Whether a thread stands in the list of its chain. */
  listed(act: ActRow): boolean {
    if (act.kind !== "thread") return false;
    if (!asksOperator(act)) return true;
    for (let at = this.rows.get(act.by); at; at = this.rows.get(at.by))
      if (at.kind === "thread" && at.on === act.on && this.listed(at)) return false;
    return true;
  }
  /** The listed threads of a chain, in the order of their first message. */
  on(chain: string): ActRow[] {
    return this.acts.filter((act) => act.on === chain && this.listed(act));
  }
  /** Who speaks in a word: the actor of its rung, or of the nearest rung or thread among the acts that made it, or the
   * operator. A word that a model wrote into the text of its thread, as the fix of a word the gate refused, runs as a
   * rung of that thread with no actor of its own, and it is the word of the actor of the thread. */
  speaker(rung: ActRow): string {
    for (let at: ActRow | undefined = rung; at; at = this.rows.get(at.by))
      if (["rung", "thread"].includes(at.kind) && at.words[2]) return String(at.words[2]);
    return OPERATOR;
  }
  private holderOfNote(note: ActRow): string | undefined {
    const at = this.order.get(note.id) ?? 0;
    const model = (act: ActRow) =>
      act.kind === "rung" && act.on === note.on && this.speaker(act) !== OPERATOR;
    const next = this.acts.slice(at + 1).find(model);
    if (next) return this.of(next);
    return this.acts
      .slice(0, at)
      .findLast((act) => act.kind === "thread" && act.on === note.on && !act.done && !asksOperator(act))?.id;
  }
}
