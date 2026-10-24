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
import { Activity, type Actor, type Call, type Entry, type Fact, isQuestion, type Turn, WORK } from "furb";

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
  carries?: { turns: number; files: number };
}

/** What a new life from a record open to read carries: the turns of the root chain and the files that the record
 * wrote, as the first word of the new life binds them. */
export interface Carry {
  turns: number;
  files: string[];
  word: string;
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
  const { turns = 0, files = 0 } = reading.carries ?? {};
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
  const siblings = () => readdirSync(folder).filter((one) => one === name || one.startsWith(`${name}.`));
  const before = new Map(
    siblings().map((one) => {
      const info = statSync(join(folder, one));
      return [one, { size: info.size, atime: info.atimeMs, mtime: info.mtimeMs }] as const;
    }),
  );
  const log = `${record}.log`;
  const held = `${record}.log.held`;
  const logged = before.has(`${name}.log`);
  if (logged) linkSync(log, held);
  const size = before.get(`${name}.log`)?.size ?? 0;
  const release = () => rmSync(held, { force: true });
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

/** How many characters of a turn the first word of a new life keeps, and how many it keeps of all turns. */
const TURN = 1500;
const BUDGET = 30000;

/** What a session that only reads does not do. */
const READS = "This record is open to read, so nothing runs on it. Start a new life from it to go on.";

/** A record open to read, with no life: the facts that the crate retold of it, as an engine and a session give a
 * snapshot of a life, so the TUI draws it as it draws a life. No word runs again, no model is asked, no command
 * runs, and nothing is kept. */
export class Reader {
  readonly root = "chain1";
  readonly activity = new Activity();
  readonly facts: Fact[] = [];
  readonly entries: Entry[] = [];
  readonly pending = new Map<string, string>();
  readonly streams = new Map<string, never>();
  readonly console = { prompts: new Map<string, never>(), notices: new Map<string, never>() };
  readonly changes = { length: 0, read: () => [] };
  /** The facts of each chain, as the transcript of the chain holds them. */
  private readonly held = new Map<string, Fact[]>();
  constructor(
    facts: unknown[],
    readonly directory: string,
    readonly record: string,
    readonly reading?: Reading,
  ) {
    for (const fact of facts as Fact[]) this.hear(fact);
    // The life stopped where the record ends, and nothing runs on a record open to read, so the work that is not done
    // stays so, with no end to come: it neither runs nor waits for a wake, so it counts as no work at all. A chain is no work, and stays as it is. A
    // rung whose run ended in the facts that the record kept ended with it. A recorded life shows a moment of a life,
    // so its work stands as it stood at that moment.
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
  /** How many turns of the root chain say something, and how many files the acts of the record wrote. */
  private carries(): { turns: number; files: number } {
    return { turns: this.said().length, files: this.written().length };
  }
  private said(): [string, string][] {
    return this.collect(this.root, Boolean(this.reading && !this.reading.logged)).flatMap(([role, text]) =>
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
      `# The earlier work of the record ${basename(this.record)}, which this life carries.`,
      `<s:${name}>\n${body}\n</s:${name}>`,
      `earlier_files = ${JSON.stringify(files.slice(0, 200))}`,
    ].join("\n");
    return { turns: said.length, files, word };
  }
  /** A recorded life, read from its file: a log of a life whose first line is a header that says so, which the
   * reader shows at the moment where the log ends. */
  static recorded(path: string, directory: string): Reader {
    const reader = new Reader([], directory, path);
    reader.follow();
    return reader;
  }
  /** The bytes of the file of a recorded life that the reader heard, up to the end of its last whole line. */
  private heard = 0;
  /** The reader of a recorded life hears each fact that a line added to its file since it read the file last, as the
   * life said it. A line that is not whole yet waits for the next read. It gives whether it heard a fact. */
  follow(): boolean {
    if (!this.recorded) return false;
    const file = openSync(this.record, "r");
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
        this.heard += end;
        // Each line is read alone, so that no text of a whole part stays in memory while its facts are heard.
        const bytes = line ?? part;
        for (let start = 0; start < end; ) {
          const stop = bytes.indexOf(10, start);
          if (bytes[start] === 91) {
            this.hear(JSON.parse(bytes.toString("utf8", start, stop)) as Fact);
            heard = true;
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

  /** One fact: the activity hears it, and the transcript of its chain holds it. */
  private hear(fact: Fact): void {
    this.facts.push(fact);
    const ear = this.activity.hear(fact);
    for (let step = ear.next(); !step.done; ) {
      const asked = step.value as Call;
      step = ear.next(
        asked.call === "covers" ? this.covers(asked.args[0] as Fact, String(asked.args[1])) : null,
      );
    }
    const [kind, about] = fact;
    // A question stands in the transcript of the chain that it is on, and any other fact in that of what it is about.
    const scope =
      this.activity.scope(isQuestion(kind, about) ? String(fact[3]) : about) || String(fact[3] ?? "");
    const facts = this.held.get(scope) ?? [];
    facts.push(fact);
    this.held.set(scope, facts);
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

  /** The turns of a chain: the paragraphs told on it between two answers, and each answer of a model. */
  turns({ on = this.root }: { on?: string } = {}): Turn[] {
    return this.collect(on, false);
  }
  /** The turns of a chain, with the message of each prompt told as a paragraph when the facts hold no tell, since the
   * facts that a record kept hold the prompts and not the paragraphs that the engine told. */
  private collect(on: string, prompts: boolean): Turn[] {
    const turns: Turn[] = [];
    let told: string[] = [];
    let cut = 0;
    for (const fact of this.transcript({ on })) {
      const [kind, about] = fact;
      if (
        prompts &&
        kind === "prompt" &&
        isQuestion("prompt", about) &&
        typeof fact[5] === "string" &&
        fact[5]
      )
        told.push(fact[5]);
      else if (kind === "reply" && isQuestion("reply", about)) cut = told.length;
      else if (kind === "done" && isQuestion("reply", about) && tupled(fact[3])?.length === 4) {
        turns.push(["user", told.slice(0, cut).join("\n\n"), null, null], tupled(fact[3]) as Turn);
        told = told.slice(cut);
        cut = 0;
      } else if (kind === "tell" && Array.isArray(fact[3])) told.push(fact[3].join("\n"));
    }
    return [...turns, ["user", told.join("\n\n"), null, null]];
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
