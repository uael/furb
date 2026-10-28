import {
  closeSync,
  existsSync,
  fstatSync,
  linkSync,
  openSync,
  readdirSync,
  readSync,
  renameSync,
  rmSync,
  statSync,
  truncateSync,
  utimesSync,
} from "node:fs";
import { basename, dirname, join } from "node:path";
import {
  Activity,
  type Actor,
  type Call,
  type Entry,
  type Fact,
  FileChanges,
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
  /** What a new life from the record carries: the turns of its root chain and the files that its acts wrote. */
  carries?: { turns: number; files: number; prompts: number };
}

/** What a new life from a record open to read carries: the turns of the root chain and the files that the record
 * wrote, as the first word of the new life binds them. */
export interface Carry {
  turns: number;
  files: string[];
  word: string;
  /** The word that resumes the record instead: it goes on from the turns of the root chain and asks again each prompt
   * of a model that the record left open, as the engine reads them from the record of any engine. */
  resume: string;
  /** How many open prompts of a model the resume asks again. */
  prompts: number;
}

/** The build that made a record and this build, as the dialog and the footer name them: an engine by the first twelve
 * characters of its hash, or a version with its engine, or an earlier engine that the record does not name. */
export function engines(reading: Reading): { made: string; build: string } {
  const ours = reading.engine.slice(0, 12);
  const theirs = reading.made?.slice(0, 12);
  return reading.version !== null
    ? {
        made: `version ${reading.version}${theirs ? ` (${theirs})` : ""}`,
        build: `version ${reading.current} (${ours})`,
      }
    : { made: theirs ?? "an earlier engine, not named", build: ours };
}

/** Whether this build made the record, so that its replay drifts on its own. */
export function sameBuild(reading: Reading): boolean {
  return reading.version === null ? reading.made === reading.engine : reading.version === reading.current;
}

/** The sentence that the footer and the composer of a record open to read say. */
export function readOnly(reading: Reading): string {
  return `Read only: ${sameBuild(reading) ? "its replay drifts" : "made by another build"}`;
}

/** The lines of the dialog of a record open to read: each engine on a line of its own, why it opens to read, and what
 * a new life carries. */
export function readingNote(reading: Reading): string[] {
  const { made, build } = engines(reading);
  const { turns = 0, files = 0, prompts = 0 } = reading.carries ?? {};
  const count = (n: number, one: string) => `${n} ${one}${n === 1 ? "" : "s"}`;
  return [
    `Made by: ${made}`,
    `This build: ${build}`,
    sameBuild(reading)
      ? `This build made it, and its replay drifts: ${reading.why}`
      : "A replay would drift, so it opens to read. No word runs again.",
    ...(reading.carries
      ? [
          `A new life carries ${count(turns, "turn")} of its root chain and the ${count(files, "file")} that it wrote.`,
          ...(prompts
            ? [`Its ${count(prompts, "open prompt")} of a model stay open, and a resume asks them again.`]
            : []),
        ]
      : []),
  ];
}

/** What a read of a record leaves as it was. A life that opens on a record writes beside it before its replay drifts:
 * it adds the entries that it says, replaces the log, and makes files of its own. The guard holds the sizes of the
 * record and its siblings and a link to the log, and `undo` gives back what the open changed, so a record that opens
 * to read is the record that it was. `keep` lets the guard go when the life opened. */
export function guard(record: string): { undo(): void; keep(): void } {
  const folder = dirname(record);
  const name = basename(record);
  const log = `${record}.log`;
  const held = `${record}.log.held`;
  // A host that was killed in the middle of an open leaves its hold behind: the same rule that `undo` uses frees it,
  // so the next open goes on. A log is derived from its record and each life writes it again, so the log that goes
  // back is the one that the cut open found, and nothing of the record is lost.
  if (existsSync(log) && existsSync(held)) {
    if (statSync(log).ino === statSync(held).ino) rmSync(held);
    else renameSync(held, log);
  }
  const siblings = () => readdirSync(folder).filter((one) => one === name || one.startsWith(`${name}.`));
  const before = new Map(
    siblings().map((one) => {
      const info = statSync(join(folder, one));
      return [one, { size: info.size, atime: info.atimeMs, mtime: info.mtimeMs }] as const;
    }),
  );
  const logged = before.has(`${name}.log`);
  if (logged) linkSync(log, held);
  const size = before.get(`${name}.log`)?.size ?? 0;
  const release = () => {
    if (logged) rmSync(held, { force: true });
  };
  return {
    keep: release,
    undo() {
      for (const one of siblings())
        if (!before.has(one) && one !== `${name}.log.held`)
          rmSync(join(folder, one), { recursive: true, force: true });
      const length = before.get(name)?.size;
      if (length !== undefined && statSync(record).size > length) truncateSync(record, length);
      if (logged) {
        // A log that the life wrote on in place is the file that the link holds, and one it replaced is not.
        if (existsSync(log) && statSync(log).ino === statSync(held).ino) rmSync(held);
        else renameSync(held, log);
        if (statSync(log).size > size) truncateSync(log, size);
      }
      // A file that the open wrote and gave back keeps the time it had, since the times tell when a record was used.
      for (const [one, was] of before) {
        const path = join(folder, one);
        if (existsSync(path) && statSync(path).mtimeMs !== was.mtime)
          utimesSync(path, was.atime / 1000, was.mtime / 1000);
      }
    },
  };
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

/** The items of a tuple as a record keeps it, which is a list of its items or the map of the tuple that wraps them. */
function tupled(value: unknown): unknown[] | undefined {
  if (Array.isArray(value)) return value;
  const held = value as { is?: unknown; args?: unknown } | null;
  if (held?.is !== "tuple" || !Array.isArray(held.args)) return undefined;
  return Array.isArray(held.args[0]) ? held.args[0] : held.args;
}

/** The tuple value of a recorded model answer, as the values that Activity reads. */
function answerValue(value: unknown): unknown[] | undefined {
  const items = tupled(value);
  if (!items || items.length !== 4) return undefined;
  return [items[0], items[1], tupled(items[2]) ?? items[2], items[3]];
}

/** How many characters of a turn the first word of a new life keeps, and how many it keeps of all turns. */
const TURN = 1500;
const BUDGET = 30000;

/** How many characters of a text the reader holds of a fact whose text no view reads whole, and the kinds of fact that
 * it holds whole, since the turns, the program and the rows read their texts. The file holds each fact whole. */
const HELD = 2000;
const WHOLE = new Set(["tell", "run", "rung", "prompt", "prefix", "module", "chain", "stand", "cd"]);

/** The place of a line in a file, and its length, in bytes. */
type Line = [at: number, length: number];

/** What a fold of the facts of a chain holds: how many facts it read, its turns, the paragraphs told since the last
 * answer, and how many of them stand before the last reply that it asked. */
type Folded = { read: number; turns: Turn[]; told: string[]; cut: number };

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
  readonly changes: FileChanges;
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
    this.changes = new FileChanges(record, true);
    if (reading) this.read();
    // The life stopped where the record ends, and nothing runs on a record open to read, so the work that is not done
    // stays so, with no end to come: it neither runs nor waits for a wake, so it counts as no work at all. A chain is no work, and stays as it is. A
    // rung whose run ended in the facts that the record kept ended with it. A recorded life shows a moment of a life,
    // so its work stands as it stood at that moment.
    if (reading) this.settle();
    if (reading) reading.carries = this.carries();
    if (reading)
      for (const act of this.activity.acts.values()) {
        // A rung that the record ended, with no run in its facts, has no run left to go on.
        if (act.kind === "rung" && act.done && act.run?.status === "running")
          act.run = { status: "done", reason: "" };
        if (!act.done && WORK.includes(act.kind) && !(act.kind === "rung" && act.run?.status !== "running"))
          act.unfinished = true;
      }
  }
  /** A model answer whose tuple values the activity reads from the live engine. */
  private normalize(fact: Fact): Fact {
    if (fact[0] !== "done" || !isQuestion("reply", fact[1])) return fact;
    const value = answerValue(fact[3]);
    return value ? ([fact[0], fact[1], fact[2], value, ...fact.slice(4)] as Fact) : fact;
  }
  /** Ends work that a recorded close or cancel already ended in the life. */
  private settle(): void {
    for (const [at, fact] of this.facts.entries()) {
      const [kind, about, , value] = fact;
      if (kind !== "close" && kind !== "cancel") continue;
      for (const act of [...this.activity.acts.values()]) {
        if (act.done || act.began > at || !WORK.includes(act.kind) || !this.covers(fact, act.id)) continue;
        const ended = kind === "close" && act.id === about ? value : { is: "CancelledError", args: [] };
        this.feed(["done", act.id, "", ended] as Fact);
      }
    }
  }
  /** Feeds a derived end fact through the activity. */
  private feed(fact: Fact): void {
    const ear = this.activity.hear(fact);
    for (let step = ear.next(); !step.done; ) {
      const asked = step.value as Call;
      step = ear.next(
        asked.call === "covers" ? this.covers(asked.args[0] as Fact, String(asked.args[1])) : null,
      );
    }
  }
  /** How many turns of the root chain say something, and how many files the acts of the record wrote. */
  private carries(): { turns: number; files: number; prompts: number } {
    // The prompts that a resume asks again are those of a model on the root chain that no end came to, as the carry of
    // the engine counts them.
    const chain = this.activity.scope(this.root) || this.root;
    let prompts = 0;
    for (const act of this.activity.acts.values())
      if (act.kind === "prompt" && !act.done && act.on === chain && act.words[2] !== "operator") prompts++;
    return { turns: this.said().length, files: this.written().length, prompts };
  }
  private said(): [string, string][] {
    const chain = this.activity.scope(this.root) || this.root;
    const folded = this.fold(
      { read: 0, turns: [], told: [], cut: 0 },
      this.held.get(chain) ?? [],
      Boolean(this.reading && !this.reading.logged),
    );
    return folded.flatMap(([role, text]) =>
      String(text ?? "").trim() ? [[String(role), String(text)] as [string, string]] : [],
    );
  }
  private written(): string[] {
    const paths = new Set<string>();
    for (const act of this.activity.acts.values()) {
      const path = (act.words[0] as { path?: unknown } | null)?.path;
      if (act.kind === "write" && typeof path === "string" && !path.includes("://")) paths.add(path);
    }
    return [...paths];
  }
  /** What a new life from the record carries, and the first word that binds it: the last turns of the root chain, each
   * cut to a length, up to a budget, and the files that the record wrote. */
  carry(): Carry {
    const said = this.said();
    const files = this.written();
    const pieces = said.map(([role, text]) => {
      // The end of a message to the model says what it asked, and the start of an answer says what it did.
      const user = role === "user";
      const cut =
        text.length <= TURN
          ? text
          : user
            ? `[the start is cut at ${TURN} characters]\n${text.slice(-TURN)}`
            : `${text.slice(0, TURN)}\n[cut at ${TURN} characters]`;
      return `${user ? "User" : "Assistant"}:\n${cut}`;
    });
    let used = 0;
    let first = pieces.length;
    while (first > 0 && used + (pieces[first - 1]?.length ?? 0) <= BUDGET)
      used += pieces[--first]?.length ?? 0;
    const left = first ? `[${first} earlier turns of the record are left out]\n\n` : "";
    const body = left + pieces.slice(first).join("\n\n");
    let name = "earlier";
    while (body.includes(`</s:${name}>`)) name += "_";
    const word = [
      `# The earlier work of the record ${JSON.stringify(basename(this.record))}, which this life carries.`,
      `<s:${name}>\n${body}\n</s:${name}>`,
      `earlier_files = ${JSON.stringify(files.slice(0, 200))}`,
    ].join("\n");
    const resume = `await inherit(${JSON.stringify(this.record)}, ${JSON.stringify(this.root)})`;
    return { turns: said.length, files, word, resume, prompts: this.carries().prompts };
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
  private hear(told: Fact, line?: Line): void {
    const whole = this.normalize(told);
    this.feed(whole);
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
  private readonly turned = new Map<string, Folded>();
  /** The turns of a chain: the paragraphs told on it between two answers, and each answer of a model. Each read takes
   * only the facts that the chain heard since the last. */
  turns({ on = this.root }: { on?: string } = {}): Turn[] {
    const chain = this.activity.scope(on) || on;
    const kept = this.turned.get(chain) ?? { read: 0, turns: [], told: [], cut: 0 };
    this.turned.set(chain, kept);
    return this.fold(kept, this.held.get(chain) ?? [], false);
  }
  /** The turns of the facts of a chain that the fold has not read yet, with the message of each prompt told as a
   * paragraph when the facts hold no tell, since the facts that a record kept hold the prompts and not the paragraphs
   * that the engine told. */
  private fold(kept: Folded, facts: Fact[], prompts: boolean): Turn[] {
    for (const fact of facts.slice(kept.read)) {
      const [kind, about] = fact;
      if (
        prompts &&
        kind === "prompt" &&
        isQuestion("prompt", about) &&
        typeof fact[5] === "string" &&
        fact[5]
      )
        kept.told.push(fact[5]);
      else if (kind === "reply" && isQuestion("reply", about)) kept.cut = kept.told.length;
      else if (kind === "done" && isQuestion("reply", about) && tupled(fact[3])?.length === 4) {
        kept.turns.push(
          ["user", kept.told.slice(0, kept.cut).join("\n\n"), null, null],
          tupled(fact[3]) as Turn,
        );
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

  dispose(): void {
    this.changes.dispose();
  }

  /** Each other verb, which would make an act or change the life. */
  refuse(): never {
    throw new Error(READS);
  }
}
