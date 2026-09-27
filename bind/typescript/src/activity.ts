import { type Call, isFault } from "./ears.js";
import { display, type Fact, isQuestion, uncommented } from "./types.js";

/** The kinds of act that are work which ends later: the table counts each one that is done, and the record may show
 * one begun and not done, which waits for a wake. */
export const WORK = ["prompt", "rung", "bash", "wait"];

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
  run?: RunState;
}
/** The questions the table asks the engine while it hears a fact. */
type Hearing<T = void> = Generator<Call, T, unknown>;
const covers = (control: Fact, id: string): Call => ({ verb: "covers", args: [control, id] });
/** The questions the engine asks on the way to what an act does, which are no acts a person follows: each is
 * answered for the act that asked it, which shows what it came to. */
const STEPS = new Set([
  "stand",
  "read",
  "write",
  "clock",
  "chance",
  "gate",
  "cd",
  "merged",
  "run",
  "wants",
  "reply",
]);

/** The observable state of the acts a person follows, updated once as facts enter the life. It asks the engine
 * what a fact alone cannot say: covers, for a pause or a wake and each live act whose state it would change, and
 * for a new act and the controls that could decide its state. */
export class Activity {
  readonly acts = new Map<string, LiveAct>();
  completed = 0;
  cost = 0;
  /** How many changes of a row the table has made, and the count at the last change of each row, so that a reader
   * takes only the rows that changed since it last read. */
  private changes = 0;
  private readonly changed = new Map<string, number>();
  /** The last pause or wake of each act it names, oldest first. A pause and a wake are over acts by the act they
   * name alone, so an earlier control of the same name decides nothing more. */
  private readonly controls = new Map<string, Fact>();
  private readonly children = new Map<string, Set<string>>();
  /** The chain each question stands on, which is the chain itself for a chain, so a fact about any question finds
   * the transcript it stands in. */
  private readonly scopes = new Map<string, string>();
  /** The rung each run runs the word of, and what the word of each rung gave when its run is done. */
  private readonly runs = new Map<string, string>();
  private readonly ran = new Map<string, unknown>();
  private readonly refused = new Map<string, string>();
  private readonly merged = new Map<string, boolean>();

  /** The chain whose transcript holds a fact about a name: the chain itself for a chain, the chain a question stands
   * on for any other question, and nothing for a name of no question. */
  scope(id: string): string {
    return this.scopes.get(id) ?? "";
  }

  /** The rows that changed after a count of changes, and the count now. */
  since(count: number): { count: number; acts: LiveAct[] } {
    const acts = [...this.acts.values()].filter((act) => (this.changed.get(act.id) ?? 0) > count);
    return { count: this.changes, acts };
  }
  private mark(act: LiveAct): void {
    this.changed.set(act.id, ++this.changes);
  }

  *hear(fact: Fact): Hearing {
    const [kind, id, by] = fact;
    const question = isQuestion(kind, id);
    if (question) this.scopes.set(id, kind === "chain" ? id : String(fact[3]));
    if (question && kind === "run") this.runs.set(id, String(fact[4]));
    if (question && !STEPS.has(kind)) {
      const act: LiveAct = {
        id,
        kind,
        by,
        on: kind === "chain" ? id : String(fact[3]),
        words: fact.slice(4),
        done: false,
        value: null,
        paused: yield* this.pausedAtBirth(id),
        ...(kind === "rung" ? { run: { status: "running" as const, reason: "" } } : {}),
      };
      if (kind === "bash")
        act.value = {
          is: "Exit",
          code: null,
          stdout: { is: "Text", path: `${id}/stdout`, content: "" },
          stderr: { is: "Text", path: `${id}/stderr`, content: "" },
        };
      this.acts.set(id, act);
      this.mark(act);
      if (!this.children.has(by)) this.children.set(by, new Set());
      this.children.get(by)?.add(id);
      return;
    }
    const act = this.acts.get(id);
    if (kind === "pause" || kind === "wake") {
      const paused = kind === "pause";
      this.controls.delete(id);
      this.controls.set(id, fact);
      for (const row of [...this.acts.values()])
        if (!row.done && row.paused !== paused && (yield covers(fact, row.id))) {
          row.paused = paused;
          this.mark(row);
        }
    } else if (kind === "done") {
      if (isQuestion("merged", id) && this.acts.get(by)?.kind === "bash")
        this.merged.set(by, Boolean(fact[3]));
      if (isQuestion("reply", id) && Array.isArray(fact[3]) && Array.isArray(fact[3][2]))
        this.cost += Number(fact[3][2][4] ?? 0);
      const rung = this.runs.get(id);
      if (rung !== undefined) {
        this.ran.set(rung, fact[3]);
        const row = this.acts.get(rung);
        if (row) this.updateRun(row);
      }
      if (act) {
        if (!act.done && WORK.includes(act.kind)) this.completed++;
        act.done = true;
        act.value = fact[3];
        this.mark(act);
        this.updateRun(act);
        for (const child of this.children.get(id) ?? []) {
          const row = this.acts.get(child);
          if (row) this.updateRun(row);
        }
      }
    } else if (kind === "tell" && act?.kind === "rung" && Array.isArray(fact[3])) {
      // The chain tells the findings that refused a word under the header refused, one comment for each.
      const [header, ...findings] = fact[3];
      if (header === `#${id} refused`) {
        this.refused.set(id, uncommented(findings.map(String).join("\n").split("\n")));
        this.updateRun(act);
      }
    } else if (kind === "out" && act?.kind === "bash" && !act.done) {
      const value = act.value as { stdout: { content: string }; stderr: { content: string } };
      const stream = fact[4] === "stderr" && this.merged.get(id) === false ? value.stderr : value.stdout;
      stream.content += String(fact[3]);
      this.mark(act);
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
