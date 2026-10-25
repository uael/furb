import { closeSync, existsSync, fstatSync, openSync, readSync } from "node:fs";
import {
  Activity,
  type Actor,
  type Call,
  type Entry,
  type Fact,
  isQuestion,
  type LiveAct,
  questionKind,
  type Turn,
  WORK,
} from "furb";
import { row } from "./snapshots.ts";

/** Why a record opens to read: the engine and the version of the build that made it, as the record names them, the
 * engine and the version of this build, and what the life that opened on it raised. */
export interface Reading {
  made: string | null;
  engine: string;
  version: number | null;
  current: number;
  why: string;
  /** Whether the facts are those of the log, which holds every fact that the life said, or those that the record
   * kept, with the acts that they name. */
  logged: boolean;
}

/** Whether a record is a recorded life: a log of a life whose first line is a header that says so, which a host reads
 * and runs not. tui/script/fixture.ts makes one from the log of a real life. */
export function isRecorded(record: string): boolean {
  if (!existsSync(record)) return false;
  // Only the head of the record is read, since a record of a long life holds gigabytes.
  const head = Buffer.alloc(4096);
  const file = openSync(record, "r");
  const size = (() => {
    try {
      return readSync(file, head, 0, head.length, 0);
    } finally {
      closeSync(file);
    }
  })();
  const first = head.subarray(0, size).toString("utf8").split("\n")[0] ?? "";
  try {
    return (JSON.parse(first) as { recorded?: unknown }).recorded === true;
  } catch {
    return false;
  }
}

/** How many bytes of the file of a recorded life the reader reads at a time. */
const PART = 16 * 2 ** 20;

/** How many characters of a text the reader holds of a fact whose text no view reads whole, and the kinds of fact that
 * it holds whole, since the turns, the program and the rows read their texts. The file holds each fact whole. */
const HELD = 2000;
const WHOLE = new Set(["tell", "run", "rung", "prompt", "prefix", "module", "chain", "stand", "cd"]);

/** The place of a line in a file, and its length, in bytes. */
type Line = [at: number, length: number];

/** A value with each text longer than HELD cut to its start, or the value itself when no text is so long. */
function shortened<T>(value: T): T {
  if (typeof value === "string") return (value.length > HELD ? `${value.slice(0, HELD)}…` : value) as T;
  if (!value || typeof value !== "object") return value;
  let changed = false;
  const each = (one: unknown) => {
    const short = shortened(one);
    changed ||= short !== one;
    return short;
  };
  const made = Array.isArray(value)
    ? value.map(each)
    : Object.fromEntries(Object.entries(value).map(([key, one]) => [key, each(one)]));
  return (changed ? made : value) as T;
}

/** What a session that only reads does not do. */
const READS = "This record is open to read, so nothing runs on it. Start a new life from it to go on.";

/** A record open to read, with no life: the facts that the crate retold of it, in the file that it names, as an engine
 * and a session give a snapshot of a life, so the TUI draws it as it draws a life. No word runs again, no model is
 * asked, no command runs, and nothing is kept. */
export class Reader {
  readonly root = "chain1";
  readonly activity = new Activity();
  readonly facts: Fact[] = [];
  readonly entries: Entry[] = [];
  readonly pending = new Map<string, string>();
  readonly streams = new Map<string, never>();
  readonly console = { prompts: new Map<string, never>(), notices: new Map<string, never>() };
  readonly changes = { length: 0, read: () => [], readable: () => [] };
  /** The facts of each chain, as the transcript of the chain holds them. */
  private readonly held = new Map<string, Fact[]>();
  /** The lines of the file that hold whole the question and the done of each act whose texts the reader cut. */
  private readonly whole = new Map<string, Line[]>();
  constructor(
    readonly directory: string,
    readonly record: string,
    readonly reading?: Reading,
    /** The file of the facts: the record of a recorded life, or the file that the crate retold the record in. */
    private readonly file = record,
  ) {
    if (reading) this.read();
    // The life stopped where the record ends, and nothing runs on a record open to read, so the work that is not done
    // waits, as paused work does. A chain is no work, and stays as it is. A recorded life shows a moment of a life,
    // so its work stands as it stood at that moment.
    if (reading)
      for (const act of this.activity.acts.values())
        if (!act.done && WORK.includes(act.kind)) act.paused = true;
  }
  /** A recorded life, read from its file: a log of a life whose first line is a header that says so, which the
   * reader shows at the moment where the log ends. */
  static recorded(path: string, directory: string): Reader {
    const reader = new Reader(directory, path);
    reader.follow();
    return reader;
  }
  /** The bytes of the file of a recorded life that the reader heard, up to the end of its last whole line. */
  private heard = 0;
  /** The reader of a recorded life hears each fact that a line added to its file since it read the file last, as the
   * life said it. A line that is not whole yet waits for the next read. It gives whether it heard a fact. */
  follow(): boolean {
    return this.recorded && this.read();
  }
  /** Each fact that a line added to the file since the reader read it last, as the life said it. A line that holds no
   * fact, as a head or a line of keep of a log, is passed over. It gives whether it heard a fact. */
  private read(): boolean {
    const file = openSync(this.file, "r");
    try {
      const size = fstatSync(file).size;
      let heard = false;
      // The file is read a part at a time, so that a long life takes no copy of its whole file to open.
      const part = Buffer.alloc(Math.min(PART, Math.max(0, size - this.heard)));
      while (this.heard < size) {
        const got = readSync(file, part, 0, Math.min(part.length, size - this.heard), this.heard);
        let end = part.subarray(0, got).lastIndexOf(10) + 1;
        // A line longer than a part is read whole.
        const line = !end && got === part.length ? Buffer.alloc(size - this.heard) : undefined;
        if (line) end = line.subarray(0, readSync(file, line, 0, line.length, this.heard)).indexOf(10) + 1;
        if (!end) break;
        const base = this.heard;
        this.heard += end;
        // Each line is read alone, so that no text of a whole part stays in memory while its facts are heard.
        const bytes = line ?? part;
        for (let start = 0; start < end; ) {
          const stop = bytes.indexOf(10, start);
          if (bytes[start] === 91) {
            const fact = JSON.parse(bytes.toString("utf8", start, stop)) as Fact;
            if (fact[0] !== "keep" || fact[1] !== "") {
              this.hear(fact, [base + start, stop - start]);
              heard = true;
            }
          }
          start = stop + 1;
        }
      }
      return heard;
    } finally {
      closeSync(file);
    }
  }
  /** Whether the reader shows a recorded life, which keeps nothing beside its file. */
  get recorded(): boolean {
    return !this.reading;
  }
  get imageDirectory(): string {
    return `${this.record}.images`;
  }
  get actor(): string {
    return String(this.standing()[2] ?? "operator");
  }

  /** One fact: the activity hears it, and the transcript of its chain holds it. A fact of a line of the file is held
   * with its long texts cut, and an act that it ends keeps of them what its row shows, since the file holds them whole. */
  private hear(whole: Fact, line?: Line): void {
    const ear = this.activity.hear(whole);
    for (let step = ear.next(); !step.done; ) {
      const asked = step.value as Call;
      step = ear.next(
        asked.call === "covers" ? this.covers(asked.args[0] as Fact, String(asked.args[1])) : null,
      );
    }
    const fact = line ? this.cut(whole, line) : whole;
    this.facts.push(fact);
    const [kind, about] = fact;
    // A question stands in the transcript of the chain that it is on, and any other fact in that of what it is about.
    const scope =
      this.activity.scope(isQuestion(kind, about) ? String(fact[3]) : about) || String(fact[3] ?? "");
    const facts = this.held.get(scope) ?? [];
    facts.push(fact);
    this.held.set(scope, facts);
  }

  /** A fact of a line of the file with its long texts cut, and the fact itself when it holds none or when a view reads
   * its texts. The act that it names keeps the line, and once it is done it keeps of its texts what its row shows. */
  private cut(fact: Fact, line: Line): Fact {
    const [kind, about] = fact;
    if (
      WHOLE.has(kind) ||
      (kind === "done" && (isQuestion("reply", about) || WHOLE.has(questionKind(about) ?? "")))
    )
      return fact;
    const short = shortened(fact);
    const act = this.activity.acts.get(about);
    if (short !== fact && act && (kind === "done" || isQuestion(kind, about)))
      this.whole.set(about, [...(this.whole.get(about) ?? []), line]);
    if (act?.done && this.whole.has(about)) {
      const slim = ["read", "write", "bash"].includes(act.kind)
        ? row(act)
        : { ...act, words: shortened(act.words), value: shortened(act.value) };
      // A part of a text holds the whole text in memory, so the parts that the act keeps are copies.
      Object.assign(act, slim, { words: structuredClone(slim.words), value: structuredClone(slim.value) });
    }
    return short === fact ? fact : structuredClone(short);
  }

  /** An act whole, as the file holds it: the words of its question and the value of its done, which the reader cut. */
  act(id: string): LiveAct | undefined {
    const act = this.activity.acts.get(id);
    const lines = this.whole.get(id);
    if (!act || !lines) return act;
    const whole: LiveAct = { ...act };
    const file = openSync(this.file, "r");
    try {
      for (const [at, length] of lines) {
        const bytes = Buffer.alloc(length);
        readSync(file, bytes, 0, length, at);
        const fact = JSON.parse(bytes.toString("utf8")) as Fact;
        if (isQuestion(fact[0], fact[1])) whole.words = fact.slice(4);
        else if (fact[0] === "done") whole.value = fact[3];
      }
    } finally {
      closeSync(file);
    }
    return whole;
  }

  /** Whether a pause or a wake covers an act: it names the act, the chain of the act, or an act that made it. */
  private covers([, name]: Fact, id: string): boolean {
    if (this.activity.scope(id) === name && name.startsWith("chain")) return true;
    const seen = new Set<string>();
    for (let at: string | undefined = id; at && !seen.has(at); at = this.activity.acts.get(at)?.by) {
      if (at === name) return true;
      seen.add(at);
    }
    return false;
  }

  /** The facts that the transcript of a chain holds. */
  transcript({ on = this.root }: { on?: string } = {}): Fact[] {
    return [...(this.held.get(this.activity.scope(on) || on) ?? [])];
  }

  /** What the root stands on: its roster, its directory and its actor, as the last stand said. */
  standing(): [Actor[], string, string] {
    const stood = this.facts.findLast(([kind, about]) => kind === "done" && isQuestion("stand", about));
    const [roster = [], directory = this.directory, actor = "operator"] = (stood?.[3] ?? []) as unknown[];
    // A recorded life names no path of this machine, so it stands where the host opens it.
    return [roster as Actor[], this.recorded ? this.directory : String(directory), String(actor)];
  }

  /** The word of each rung of a chain, by the rung, as each run of the chain ran it since its last module. */
  program({ on = this.root }: { on?: string } = {}): Record<string, string> {
    let words: Record<string, string> = {};
    for (const fact of this.transcript({ on })) {
      if (fact[0] === "module") words = {};
      if (fact[0] === "run" && isQuestion("run", fact[1]))
        words[String(fact[6] || fact[4])] = String(fact[5]);
    }
    return words;
  }

  /** The turns of each chain as far as the reader read its facts: the turns before its last answer, the paragraphs told
   * since, and how many of them stand before the last reply that it asked. */
  private readonly turned = new Map<string, { read: number; turns: Turn[]; told: string[]; cut: number }>();
  /** The turns of a chain: the paragraphs told on it between two answers, and each answer of a model. Each read takes
   * only the facts that the chain heard since the last. */
  turns({ on = this.root }: { on?: string } = {}): Turn[] {
    const chain = this.activity.scope(on) || on;
    const facts = this.held.get(chain) ?? [];
    const kept = this.turned.get(chain) ?? { read: 0, turns: [], told: [], cut: 0 };
    this.turned.set(chain, kept);
    for (const fact of facts.slice(kept.read)) {
      const [kind, about] = fact;
      if (kind === "reply" && isQuestion("reply", about)) kept.cut = kept.told.length;
      else if (
        kind === "done" &&
        isQuestion("reply", about) &&
        Array.isArray(fact[3]) &&
        fact[3].length === 4
      ) {
        kept.turns.push(["user", kept.told.slice(0, kept.cut).join("\n\n"), null, null], fact[3] as Turn);
        kept.told = kept.told.slice(kept.cut);
        kept.cut = 0;
      } else if (kind === "tell" && Array.isArray(fact[3])) kept.told.push(fact[3].join("\n"));
    }
    kept.read = facts.length;
    return [...kept.turns, ["user", kept.told.join("\n\n"), null, null]];
  }

  /** Where a chain stands: where it went last, or the directory of the life. */
  cwd({ on = this.root }: { on?: string } = {}): string {
    const went = this.transcript({ on }).findLast(
      ([kind, about]) => kind === "done" && isQuestion("cd", about),
    );
    return went ? String(went[3]) : this.standing()[1];
  }

  inspect(name: string): { value: unknown } {
    if (name !== "actor") throw new Error(READS);
    return { value: this.actor };
  }

  verb(): undefined {
    return undefined;
  }

  outcome(id: string): { done: boolean; value: unknown } {
    const act = this.activity.acts.get(id);
    return { done: act?.done ?? false, value: act?.value ?? null };
  }

  isPaused(id: string): boolean {
    return this.activity.acts.get(id)?.paused ?? false;
  }

  dispose(): void {}

  /** Each other verb, which would make an act or change the life. */
  refuse(): never {
    throw new Error(READS);
  }
}
