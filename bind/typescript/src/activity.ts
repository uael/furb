import { restated } from "../index.cjs";
import { type Call, call, isFault } from "./ears.js";
import {
  type Act,
  type Compaction,
  followed,
  operated,
  type PauseCause,
  type Spend,
  WORK,
} from "./life/acts.js";
import { bound, display, type Fact, isQuestion, usageOf } from "./types.js";

/** One answer of a model on a chain: its place in time, the actor whose reply it answered, and its usage, which is
 * the tokens that it read and wrote, of which the tokens that it read again and kept to read again, and its dollars. */
export interface Answered {
  at: number;
  actor: string;
  usage: [number, number, number, number, number];
}
/** How the word of a rung ran: it runs, it ran, or it failed, with what a person reads. */
export interface LiveRun {
  status: "running" | "failed" | "done";
  reason: string;
  /** Whether the gate refused the word, so that the reason holds its findings, one for each line. */
  refused?: boolean;
}
/** An act as the activity follows it. A host with a daemon reads the acts of the daemon instead, and a host with none
 * reads them as the daemon gives them through [`Activity.hosted`]. */
export interface LiveAct {
  id: string;
  kind: string;
  by: string;
  /** The chain that the act stands on, which is the chain of its thread for a prompt on a thread. */
  on: string;
  /** The thread that a prompt on a thread is on: the prompt that its on names. */
  thread?: string;
  words: unknown[];
  /** The word of a rung, which its caller or its model wrote, once its chain took it to run or to refuse. */
  word?: string;
  done: boolean;
  value: unknown;
  /** Whether a pause stands over the act while it lives. */
  paused: boolean;
  /** The cause of the pause that stands over the act, while one does. */
  pause?: PauseCause;
  /** Whether the act had no end when the record ended, in a record open to read, where nothing runs to end it. */
  unfinished?: boolean;
  run?: LiveRun;
  /** Whether a reply of the rung runs, while its model thinks or writes its word. */
  asking?: boolean;
  /** The place of the act in time: how many facts the life had said when the act was made, and when it was done. It
   * is the order of the record, so a later life gives the same places. */
  began: number;
  ended?: number;
  /** The dollars of the answers of the models that wrote the word of a rung, once one is done. */
  cost?: number;
}
/** Whether a pause or a wake covers an act, which the engine answers the ear that yields the question. */
function* covers(control: Fact, id: string): Generator<Call, boolean, unknown> {
  return Boolean(yield call("covers", [control, id]));
}

/** The observable state of the acts a person follows, updated once as facts enter the life. It asks the engine
 * what a fact alone cannot say: covers, for a pause or a wake and each live act whose state it would change, and
 * for a new act and the controls that could decide its state. */
export class Activity {
  readonly acts = new Map<string, LiveAct>();
  completed = 0;
  cost = 0;
  /** Each compaction of a chain, in the order of the record. */
  readonly compactions: Compaction[] = [];
  /** The answers of the models on each chain, in order, by the chain that the reply stands on, so that a host reads
   * the dollars of a chain and the share of the window that its last answer read. */
  readonly answers = new Map<string, Answered[]>();
  /** The place up to which the operator read each chain or thread, by its name: the place of the last fact seen that
   * he said of it, which covers each change that the record holds before it. An act whose place is past it, as it began
   * or ended, is new to him. */
  readonly seen = new Map<string, number>();
  /** How many facts the activity has heard, which is the place in time of the fact that it hears. */
  private clock = 0;
  /** The chain, the actor and the maker of each reply that is not done. The maker of a reply is the rung that waits
   * for the word that the reply writes. */
  private readonly replies = new Map<string, [string, string, string]>();
  /** How many changes of a row the table has made, and the count at the last change of each row, so that a reader
   * takes only the rows that changed since it last read. */
  private changes = 0;
  private readonly changed = new Map<string, number>();
  /** Each change of a row in order, as the count of the change and the name of the row, so that a reader finds the
   * rows that changed after a count with no pass over every row. A row that changed again leaves its earlier entries,
   * which the log drops once it holds twice as many entries as rows. */
  private log: { count: number[]; name: string[] } = { count: [], name: [] };
  /** The place of each row in the table. */
  private readonly places = new Map<string, number>();
  /** The rows that are not done, in the order of the table, which a pause or a wake can change. */
  private readonly open = new Set<string>();
  /** The last pause or wake of each act it names, oldest first. A pause and a wake are over acts by the act they
   * name alone, so an earlier control of the same name decides nothing more. */
  private readonly controls = new Map<string, Fact>();
  private readonly children = new Map<string, Set<string>>();
  /** The chain each question stands on, which is the chain itself for a chain and the chain of its thread for a prompt
   * on a thread, so a fact about any question finds the transcript it stands in. */
  private readonly scopes = new Map<string, string>();
  /** The rung each run runs the word of, and what the word of each rung gave when its run is done. */
  private readonly runs = new Map<string, string>();
  private readonly ran = new Map<string, unknown>();
  private readonly refused = new Map<string, string>();
  /** The acts that stand in each chain but its chains, in the order they began. */
  private readonly inChain = new Map<string, LiveAct[]>();
  /** How many times the spend of a chain or of a rung changed. */
  spendChanges = 0;

  /** The chain whose transcript holds a fact about a name: the chain itself for a chain, the chain a question stands
   * on for any other question, and nothing for a name of no question. */
  scope(id: string): string {
    return this.scopes.get(id) ?? "";
  }

  /** The rows that changed after a count of changes, and the count now. */
  /** What the answers of the models cost each chain and each rung whose word they wrote: their dollars, how many they
   * are, and the actor of the last. */
  readonly spends = new Map<string, Spend>();

  /** Acts as the daemon gives them, each with its meaning: its phase, its pause, its spend and its state, and each
   * chain that they stand in, in the state that the acts of the chain give it. A host with no daemon reads them, as a
   * record open to read, whose console owes the prompts that `owed` names. */
  hosted(acts: readonly LiveAct[], owed: ReadonlySet<string> = new Set()): Act[] {
    const out = new Map(acts.map((act) => [act.id, this.shape(act, owed)]));
    const shaped = (act: LiveAct) => out.get(act.id) ?? this.shape(act, owed);
    // An act that had no end when the record ended is no work of its chain, unless the operator owes it.
    const open = [...this.open].flatMap((id) => {
      const act = this.acts.get(id);
      return act && (!act.unfinished || owed.has(id)) ? [act] : [];
    });
    // The crate gives each act its state, and each chain the state that the acts that stand in it give.
    const states = new Map(restated([...out.values()]).map((act) => [act.id, act.state]));
    const chains = new Set([
      ...acts.map((act) => act.on),
      ...acts.flatMap((act) => (act.kind === "chain" ? [act.id] : [])),
    ]);
    for (const chain of chains) {
      const act = this.acts.get(chain);
      if (act?.kind !== "chain") continue;
      const remote = open.filter(
        (one) =>
          one.kind === "prompt" &&
          one.on !== chain &&
          this.acts.get(this.acts.get(one.by)?.kind === "rung" ? one.by : "")?.on === chain,
      );
      const read = [act, ...this.stating(chain, open, owed), ...remote].map(shaped);
      states.set(chain, restated(read).find((one) => one.id === chain)?.state ?? "idle");
      out.set(chain, shaped(act));
    }
    for (const [id, act] of out) act.state = states.get(id) ?? act.state;
    return [...out.values()];
  }
  /** The acts of a chain that its state reads, in the order they began: the acts that the operator owes, the acts that
   * are not done, its last act of work, and its first prompt. Its other acts decide nothing, so a chain with a long
   * life reads as few acts as a short one. */
  private stating(chain: string, open: readonly LiveAct[], owed: ReadonlySet<string>): LiveAct[] {
    const mine = this.inChain.get(chain) ?? [];
    const work = (act: LiveAct) => WORK.includes(act.kind) && !act.unfinished;
    const asked = [...owed].flatMap((id) => this.acts.get(id) ?? []);
    const read = new Set(
      [...open, ...asked, mine.findLast(work), mine.find((act) => act.kind === "prompt" && work(act))].filter(
        (act): act is LiveAct => act !== undefined && act.kind !== "chain" && act.on === chain,
      ),
    );
    return [...read].sort((one, other) => (this.places.get(one.id) ?? 0) - (this.places.get(other.id) ?? 0));
  }
  /** One act as the daemon gives it, in the state that it holds alone. */
  private shape(act: LiveAct, owed: ReadonlySet<string>): Act {
    const run = act.run;
    const reason = run?.reason ?? "";
    return {
      kind: act.kind,
      id: act.id,
      by: act.by,
      on: act.thread ?? act.on,
      chain: act.on,
      words: act.words,
      phase: act.done ? { phase: "done", value: act.value } : { phase: "asked" },
      operator: owed.has(act.id),
      // An act that had no end when the record ended stays held, since nothing runs to end it.
      pause: act.paused || act.unfinished ? (act.pause ?? { by: "" }) : null,
      run: !run
        ? null
        : run.status !== "failed"
          ? { run: run.status }
          : run.refused
            ? { run: "refused", reason }
            : /^CancelledError\b/.test(reason)
              ? { run: "cancelled", reason }
              : { run: "failed", reason },
      word: act.word ?? null,
      asking: Boolean(act.asking),
      began: act.began,
      ended: act.ended ?? null,
      spend: this.spends.get(act.id) ?? null,
      // The crate gives the state, which hosted reads.
      state: "idle",
    };
  }

  since(count: number): { count: number; acts: LiveAct[] } {
    const { log } = this;
    let low = 0;
    for (let high = log.count.length; low < high; ) {
      const middle = (low + high) >> 1;
      if ((log.count[middle] ?? 0) <= count) low = middle + 1;
      else high = middle;
    }
    const acts: LiveAct[] = [];
    for (let at = low; at < log.count.length; at++) {
      const name = log.name[at] ?? "";
      const act = this.acts.get(name);
      if (act && this.changed.get(name) === log.count[at]) acts.push(act);
    }
    acts.sort((one, other) => (this.places.get(one.id) ?? 0) - (this.places.get(other.id) ?? 0));
    return { count: this.changes, acts };
  }
  private mark(act: LiveAct): void {
    this.changed.set(act.id, ++this.changes);
    this.log.count.push(this.changes);
    this.log.name.push(act.id);
    if (this.log.count.length <= 2 * this.acts.size + 64) return;
    const kept: { count: number[]; name: string[] } = { count: [], name: [] };
    for (const [at, name] of this.log.name.entries())
      if (this.changed.get(name) === this.log.count[at]) {
        kept.count.push(this.log.count[at] ?? 0);
        kept.name.push(name);
      }
    this.log = kept;
  }

  *hear(fact: Fact): Generator<Call, void, unknown> {
    const [kind, id, by] = fact;
    const at = ++this.clock;
    const question = isQuestion(kind, id);
    if (question) this.scopes.set(id, kind === "chain" ? id : this.scope(String(fact[3])) || String(fact[3]));
    if (question && kind === "run") {
      this.runs.set(id, String(fact[4]));
      // A record that keeps no ready holds the word of a rung in the run of that word.
      const rung = this.acts.get(String(fact[4]));
      if (rung?.kind === "rung" && rung.word === undefined) this.worded(rung, fact[5]);
    }
    if (question && kind === "reply") {
      this.replies.set(id, [String(fact[3]), String(fact[4]), by]);
      const rung = this.acts.get(by);
      if (rung?.kind === "rung") {
        rung.asking = true;
        this.mark(rung);
      }
    }
    if (question && followed(kind)) {
      const act: LiveAct = {
        id,
        kind,
        by,
        on: this.scope(id),
        ...(kind === "prompt" && fact[3] !== this.scope(id) ? { thread: String(fact[3]) } : {}),
        words: fact.slice(4),
        done: false,
        value: null,
        paused: yield* this.pausedAtBirth(id),
        // A question said again, as a later life asks it anew, keeps the place where it first began.
        began: this.acts.get(id)?.began ?? at,
        ...(kind === "rung" ? { run: { status: "running" as const, reason: "" } } : {}),
      };
      if (kind === "bash")
        act.value = {
          is: "Exit",
          code: null,
          stdout: { is: "Text", path: `${id}/stdout`, content: "" },
          stderr: { is: "Text", path: `${id}/stderr`, content: "" },
        };
      this.places.set(id, this.acts.size);
      this.acts.set(id, act);
      if (kind !== "chain") (this.inChain.get(act.on) ?? this.inChain.set(act.on, []).get(act.on))?.push(act);
      this.open.add(id);
      this.mark(act);
      if (!this.children.has(by)) this.children.set(by, new Set());
      this.children.get(by)?.add(id);
      return;
    }
    const act = this.acts.get(id);
    // A later life says each fact of its record again as the journal, which names no speaker, and only the operator
    // says seen.
    if (kind === "seen" && (operated(by) || by === "journal")) {
      this.seen.set(id, at);
    } else if (kind === "prefix" && by !== id) {
      // A chain says its own prefix at its birth, as a chain with a source does, and that prefix cuts nothing.
      this.compactions.push({ chain: id, by, at });
    } else if (kind === "pause" || kind === "wake") {
      const paused = kind === "pause";
      const cause: PauseCause | undefined = paused
        ? { by, ...(fact[3] === null || fact[3] === undefined ? {} : { why: fact[3] }) }
        : undefined;
      this.controls.delete(id);
      this.controls.set(id, fact);
      for (const name of [...this.open]) {
        const row = this.acts.get(name);
        if (row && row.paused !== paused && (yield* covers(fact, row.id))) {
          row.paused = paused;
          if (cause) row.pause = cause;
          else delete row.pause;
          this.mark(row);
        }
      }
    } else if (kind === "done") {
      const reply = this.replies.get(id);
      this.replies.delete(id);
      const asked = reply && this.acts.get(reply[2]);
      if (asked?.asking) {
        delete asked.asking;
        this.mark(asked);
      }
      if (reply && Array.isArray(fact[3]) && Array.isArray(fact[3][2])) {
        const usage: Answered["usage"] = usageOf(fact[3][2]);
        this.cost += usage[4];
        const [chain, actor, maker] = reply;
        const answers = this.answers.get(chain) ?? [];
        answers.push({ at, actor, usage });
        this.answers.set(chain, answers);
        // A rung costs the dollars of the answers that wrote its word.
        const wrote = this.acts.get(maker);
        if (wrote?.kind === "rung") {
          wrote.cost = (wrote.cost ?? 0) + usage[4];
          this.mark(wrote);
        }
        for (const one of [chain, ...(wrote?.kind === "rung" ? [maker] : [])]) {
          const spend = this.spends.get(one) ?? { dollars: 0, answers: 0 };
          this.spends.set(one, { dollars: spend.dollars + usage[4], answers: spend.answers + 1, actor });
          this.spendChanges++;
        }
      }
      const rung = this.runs.get(id);
      if (rung !== undefined) {
        this.ran.set(rung, fact[3]);
        const row = this.acts.get(rung);
        if (row) this.updateRun(row);
      }
      if (act) {
        if (!act.done && WORK.includes(act.kind)) this.completed++;
        act.done = true;
        this.open.delete(id);
        act.ended ??= at;
        act.value = fact[3];
        this.mark(act);
        this.updateRun(act);
        for (const child of this.children.get(id) ?? []) {
          const row = this.acts.get(child);
          if (row) this.updateRun(row);
        }
      }
    } else if (kind === "ready" && act?.kind === "rung" && typeof fact[3] === "string") {
      this.worded(act, fact[3]);
    } else if (kind === "tell" && act?.kind === "rung" && Array.isArray(fact[3])) {
      // The chain tells the findings that refused a word under the header refused, and binds them one line for each.
      const [header, ...lines] = String(fact[3][0] ?? "").split("\n");
      if (header === `#${id} refused`) {
        this.refused.set(id, bound(lines, `${id}_findings`) ?? "");
        this.updateRun(act);
      }
    } else if (kind === "started" && act?.kind === "bash" && !act.done) {
      // A command that a wake runs again starts its streams again, as its doors do.
      const value = act.value as { stdout: { content: string }; stderr: { content: string } };
      value.stdout.content = value.stderr.content = "";
      this.mark(act);
    } else if (kind === "out" && act?.kind === "bash" && !act.done) {
      const value = act.value as { stdout: { content: string }; stderr: { content: string } };
      const stream = fact[4] === "stderr" ? value.stderr : value.stdout;
      stream.content += String(fact[3]);
      this.mark(act);
    }
  }
  /** Whether the last control over a new act is a pause. Only a control no older than the oldest pause can make
   * it one. */
  private *pausedAtBirth(id: string): Generator<Call, boolean, unknown> {
    const controls = [...this.controls.values()];
    const oldest = controls.findIndex(([kind]) => kind === "pause");
    for (let at = controls.length - 1; oldest >= 0 && at >= oldest; at--) {
      const control = controls[at] as Fact;
      if (yield* covers(control, id)) return control[0] === "pause";
    }
    return false;
  }
  /** The word of a rung, as the chain took it to run or to refuse. */
  private worded(act: LiveAct, word: unknown): void {
    act.word = String(word ?? "");
    this.mark(act);
  }
  private updateRun(act: LiveAct): void {
    if (act.kind !== "rung") return;
    this.mark(act);
    const refused = this.refused.get(act.id);
    if (refused !== undefined) {
      act.run = { status: "failed", reason: refused, refused: true };
      return;
    }
    const ran = this.ran.has(act.id);
    const value = ran ? this.ran.get(act.id) : act.value;
    if (isFault(value)) {
      const parent = this.acts.get(act.by);
      if (
        value.is === "CancelledError" &&
        (act.words[1] ||
          (act.done && !isFault(act.value)) ||
          (parent?.kind === "prompt" && parent.done && !isFault(parent.value)))
      )
        act.run = { status: "done", reason: "" };
      else act.run = { status: "failed", reason: `${value.is}: ${value.args.map(display).join(", ")}` };
    } else act.run = { status: ran ? "done" : "running", reason: "" };
  }
}
