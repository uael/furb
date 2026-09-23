import { display, type Fact, isTag } from "./types.js";

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
  paused: boolean;
  run?: RunState;
}
const exception = (value: unknown): value is { is: string; args: unknown[] } =>
  Boolean(value && typeof value === "object" && "is" in value && "args" in value);
const lineage = (id: string) => id.split("://").at(-1) ?? "";
const under = (id: string, parent: string) =>
  Boolean(parent) && `${lineage(id)}.`.startsWith(`${lineage(parent)}.`);

/** The observable state of acts, updated once as facts enter the life. No sandbox query is needed but one per
 * kind of question the file does not make: a kind is made by one verb, an act or a query, and the last ear hears
 * every act but only a query nobody answered, so a fact alone cannot say which its kind is. */
export class Activity {
  readonly acts = new Map<string, LiveAct>();
  completed = 0;
  cost = 0;
  /** Each kind of question and whether its verb makes acts: the file's own acts, then the kinds the life said. */
  private readonly kinds = new Map(
    ["chain", "prompt", "rung", "bash", "wait", "grant"].map((kind) => [kind, true]),
  );
  /** A question of each kind not known yet, which the owner of the life asks it about outside any ear. */
  readonly unknown = new Map<string, string>();
  /** How many times the table was derived again, which a reader of it compares to know that. */
  generation = 0;
  private controls = new Map<string, { paused: boolean; order: number }>();
  private children = new Map<string, Set<string>>();
  private ran = new Map<string, unknown>();
  private refused = new Map<string, string>();
  private merged = new Map<string, boolean>();
  private order = 0;

  /** What the life said of a kind: an act kind derives the table again from the facts, which now hold its acts. */
  learn(kind: string, act: boolean, facts: readonly Fact[]): void {
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
    this.merged = new Map();
    this.order = 0;
    this.generation++;
    for (const fact of facts) this.hear(fact);
  }

  hear(fact: Fact): void {
    const [kind, id, by] = fact;
    const question = id.startsWith(`${kind}://`);
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
        paused: false,
        ...(kind === "rung" ? { run: { status: "running" as const, reason: "" } } : {}),
      };
      let order = 0;
      for (const [target, control] of this.controls)
        if (control.order > order && (target === act.on || under(id, target))) {
          act.paused = control.paused;
          order = control.order;
        }
      if (kind === "bash")
        act.value = {
          is: "Exit",
          code: null,
          stdout: { is: "Text", path: `${id}/stdout`, content: "" },
          stderr: { is: "Text", path: `${id}/stderr`, content: "" },
        };
      this.acts.set(id, act);
      if (!this.children.has(by)) this.children.set(by, new Set());
      this.children.get(by)?.add(id);
      return;
    }
    const act = this.acts.get(id);
    if (kind === "answer" && Array.isArray(fact[3]) && Array.isArray(fact[3][2]))
      this.cost += Number(fact[3][2][4] ?? 0);
    if (kind === "pause" || kind === "wake") {
      const paused = kind === "pause";
      this.controls.set(id, { paused, order: ++this.order });
      for (const row of this.acts.values()) if (row.on === id || under(row.id, id)) row.paused = paused;
    } else if (kind === "done") {
      if (id.startsWith("merged://") && this.acts.get(by)?.kind === "bash")
        this.merged.set(by, Boolean(fact[3]));
      if (act) {
        if (!act.done && ["prompt", "rung", "bash", "wait"].includes(act.kind)) this.completed++;
        act.done = true;
        act.value = fact[3];
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
      for (const tag of fact[3])
        if (isTag(tag) && tag[0] === "refused") {
          this.refused.set(id, typeof tag[2] === "string" ? tag[2] : display(tag[2]));
          this.updateRun(act);
        }
    } else if (kind === "out" && act?.kind === "bash" && !act.done) {
      const value = act.value as { stdout: { content: string }; stderr: { content: string } };
      const stream = fact[4] === "stderr" && this.merged.get(id) === false ? value.stderr : value.stdout;
      stream.content += String(fact[3]);
    }
  }
  private updateRun(act: LiveAct): void {
    if (act.kind !== "rung") return;
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
