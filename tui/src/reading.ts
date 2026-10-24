import { closeSync, existsSync, fstatSync, openSync, readSync } from "node:fs";
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
    // waits, as paused work does. A chain is no work, and stays as it is. A recorded life shows a moment of a life,
    // so its work stands as it stood at that moment.
    if (reading)
      for (const act of this.activity.acts.values())
        if (!act.done && WORK.includes(act.kind)) act.paused = true;
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
    const turns: Turn[] = [];
    let told: string[] = [];
    let cut = 0;
    for (const fact of this.transcript({ on })) {
      const [kind, about] = fact;
      if (kind === "reply" && isQuestion("reply", about)) cut = told.length;
      else if (
        kind === "done" &&
        isQuestion("reply", about) &&
        Array.isArray(fact[3]) &&
        fact[3].length === 4
      ) {
        turns.push(["user", told.slice(0, cut).join("\n\n"), null, null], fact[3] as Turn);
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
