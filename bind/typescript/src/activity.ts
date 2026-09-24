import type { Call, WorldPart } from "./extension.js";
import { display, type Fact, isQuestion, uncommented } from "./types.js";

export interface RunState {
  status: "running" | "failed" | "done";
  reason: string;
}
export interface LiveAct {
  id: string;
  kind: string;
  by: string;
  on: string;
  words: unknown[];
  done: boolean;
  value: unknown;
  /** Whether a pause stands over the act while it lives. */
  paused: boolean;
  /** Whether the World does the work of the act and was started on it: a start said so, or the record showed the
   * act begun. */
  started: boolean;
  run?: RunState;
}
/** The questions the table asks the engine while it hears a fact. */
type Hearing<T = void> = Generator<Call, T, unknown>;
const exception = (value: unknown): value is { is: string; args: unknown[] } =>
  Boolean(value && typeof value === "object" && "is" in value && "args" in value);
const covers = (control: Fact, id: string): Call => ({ verb: "covers", args: [control, id] });

/** The observable state of acts, updated once as facts enter the life. It asks the engine what a fact alone cannot
 * say. One question per kind of question the file does not make: a kind is made by one verb, an act or a query,
 * and the last ear hears every act but only a query nobody answered. And covers, for a pause or a wake and each
 * live act whose state it would change, and for a new act and the controls that could decide its state. */
export class Activity {
  readonly acts = new Map<string, LiveAct>();
  /** How many acts of work are done: a prompt, a rung, or an act the World was started on, which the World made
   * not itself. */
  completed = 0;
  cost = 0;
  /** Each kind of question and whether its verb makes acts: the acts of the engine and of the parts of the World,
   * then the kinds the life said. */
  private readonly kinds: Map<string, boolean>;
  /** What each part of the World adds to the table of its acts. */
  private readonly views: WorldPart[];
  /** A question of each kind not known yet, which the owner of the life asks it about outside any ear. */
  readonly unknown = new Map<string, string>();
  /** How many times the table was derived again, which a reader of it compares to know that. */
  generation = 0;
  /** How many changes of a row the table has made, and the count at the last change of each row, so that a reader
   * takes only the rows that changed since it last read. */
  private changes = 0;
  private changed = new Map<string, number>();
  /** The count of changes when the table was last derived again: a reader behind it takes the whole table. */
  private derived = 0;
  /** The last pause or wake of each act it names, oldest first. A pause and a wake are over acts by the act they
   * name alone, so an earlier control of the same name decides nothing more. */
  private controls = new Map<string, Fact>();
  private children = new Map<string, Set<string>>();
  private ran = new Map<string, unknown>();
  private refused = new Map<string, string>();

  constructor(parts: readonly WorldPart[] = []) {
    const kinds = ["chain", "prompt", "rung", "wait", ...parts.flatMap((part) => part.kinds ?? [])];
    this.kinds = new Map(kinds.map((kind) => [kind, true]));
    this.views = parts.filter((part) => part.live);
  }

  /** What the life said of a kind: an act kind derives the table again from the facts, which now hold its acts, and
   * asks the engine through the call it is given. */
  learn(kind: string, act: boolean, facts: readonly Fact[], call: (question: Call) => unknown): void {
    this.kinds.set(kind, act);
    this.unknown.delete(kind);
    if (!act) return;
    this.acts.clear();
    this.completed = 0;
    this.cost = 0;
    this.controls = new Map();
    this.children = new Map();
    this.ran = new Map();
    this.refused = new Map();
    this.generation++;
    this.changed = new Map();
    this.derived = this.changes;
    for (const fact of facts) {
      const hearing = this.hear(fact);
      for (let step = hearing.next(); !step.done; step = hearing.next(call(step.value)));
    }
  }

  /** The rows that changed after a count of changes, all of them when the table was derived again since, and the
   * count now. */
  since(count: number): { count: number; whole: boolean; acts: LiveAct[] } {
    const whole = count < this.derived;
    const acts = [...this.acts.values()].filter((act) => whole || (this.changed.get(act.id) ?? 0) > count);
    return { count: this.changes, whole, acts };
  }
  private mark(act: LiveAct): void {
    this.changed.set(act.id, ++this.changes);
  }

  *hear(fact: Fact): Hearing {
    const [kind, id, by] = fact;
    const question = isQuestion(kind, id);
    const known = this.kinds.get(kind);
    if (question && known === undefined && !this.unknown.has(kind)) this.unknown.set(kind, id);
    if (question && known) {
      const act: LiveAct = {
        id,
        kind,
        by,
        on: kind === "chain" ? id : String(fact[3]),
        words: fact.slice(4),
        done: false,
        value: null,
        paused: yield* this.pausedAtBirth(id),
        started: false,
        ...(kind === "rung" ? { run: { status: "running" as const, reason: "" } } : {}),
      };
      for (const part of this.views)
        if (part.kinds?.includes(kind) && part.live?.born) act.value = part.live.born(act);
      this.acts.set(id, act);
      this.mark(act);
      if (!this.children.has(by)) this.children.set(by, new Set());
      this.children.get(by)?.add(id);
      return;
    }
    const act = this.acts.get(id);
    if (kind === "answer" && Array.isArray(fact[3]) && Array.isArray(fact[3][2]))
      this.cost += Number(fact[3][2][4] ?? 0);
    if (kind === "pause" || kind === "wake") {
      const paused = kind === "pause";
      this.controls.delete(id);
      this.controls.set(id, fact);
      for (const row of [...this.acts.values()])
        if (!row.done && row.paused !== paused && (yield covers(fact, row.id))) {
          row.paused = paused;
          this.mark(row);
        }
    } else if (kind === "start" && act) {
      act.started = true;
      this.mark(act);
    } else if (kind === "done") {
      // An act that the World does the work of asks the record at its birth whether it holds the act, under a query
      // that the act made: a record that holds it shows it begun, so the life starts it again only at a wake.
      const asker = this.acts.get(/^holds@(\w+)\.\d+$/.exec(id)?.[1] ?? "");
      if (asker && asker.kind !== "chain" && Array.isArray(fact[3]) && fact[3].length && !asker.started) {
        asker.started = true;
        this.mark(asker);
      }
      if (act) {
        if (!act.done && act.by !== "world" && (act.started || act.kind === "prompt" || act.kind === "rung"))
          this.completed++;
        act.done = true;
        // An act of a part that gives its value keeps that value, which the part made of the facts of the World, so
        // no value of the engine crosses for it; a fault it came to stands in its place.
        const owned = this.views.some((part) => part.kinds?.includes(act.kind) && part.live?.born);
        if (!owned || exception(fact[3])) act.value = fact[3];
        this.mark(act);
        this.updateRun(act);
        for (const child of this.children.get(id) ?? []) {
          const row = this.acts.get(child);
          if (row) this.updateRun(row);
        }
      }
    } else if (kind === "ran") {
      this.ran.set(id, fact[3]);
      if (act) this.updateRun(act);
    } else if (kind === "tell" && act?.kind === "rung" && Array.isArray(fact[3])) {
      // The chain tells the findings that refused a word under the header refused, one comment for each.
      const [header, ...findings] = fact[3];
      if (header === `#${id} refused`) {
        this.refused.set(id, uncommented(findings.map(String).join("\n").split("\n")));
        this.updateRun(act);
      }
    }
    for (const part of this.views) {
      const changed = part.live?.hears?.(fact, this.acts);
      if (changed) this.mark(changed);
    }
  }
  /** Whether the last control over a new act is a pause. Only a control no older than the oldest pause can make
   * it one. */
  private *pausedAtBirth(id: string): Hearing<boolean> {
    const controls = [...this.controls.values()];
    const oldest = controls.findIndex(([kind]) => kind === "pause");
    for (let at = controls.length - 1; oldest >= 0 && at >= oldest; at--) {
      const control = controls[at] as Fact;
      if (yield covers(control, id)) return control[0] === "pause";
    }
    return false;
  }
  private updateRun(act: LiveAct): void {
    if (act.kind !== "rung") return;
    this.mark(act);
    const refused = this.refused.get(act.id);
    if (refused !== undefined) {
      act.run = { status: "failed", reason: refused };
      return;
    }
    const ran = this.ran.has(act.id);
    const value = ran ? this.ran.get(act.id) : act.value;
    if (exception(value)) {
      const parent = this.acts.get(act.by);
      if (
        value.is === "CancelledError" &&
        (act.words[1] ||
          (act.done && !exception(act.value)) ||
          (parent?.kind === "prompt" && parent.done && !exception(parent.value)))
      )
        act.run = { status: "done", reason: "" };
      else act.run = { status: "failed", reason: `${value.is}: ${value.args.map(display).join(", ")}` };
    } else act.run = { status: ran ? "done" : "running", reason: "" };
  }
}
