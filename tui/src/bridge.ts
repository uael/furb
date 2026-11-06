import { randomUUID } from "node:crypto";
import { EventEmitter } from "node:events";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { createTwoFilesPatch } from "diff";
import {
  actorParts,
  attachImage,
  codexAuth,
  ENGINE,
  type Fact,
  type FileChange,
  FileChanges,
  type ImageAttachment,
  isQuestion,
  model,
  models,
  questionKind,
  type Repaired,
  retold,
  type SetupInfo,
  type Stream,
  saveFile,
  setup,
  type Turn,
  usageOf,
  WINDOW,
} from "furb";
import { type Act, type Compaction, type Exit, isPaused, type Measure, type Spend } from "furb/life";
import type { Config, FollowUp, ThreadNode, Threads } from "furb/protocol";
import { type Choices, daemon, type Listener, Refused, RemoteLife } from "./client.ts";
import { clock } from "./clock.ts";
import type { Setting } from "./configs.ts";
import { demoModels } from "./demo/answer.ts";
import { expandHome } from "./files.ts";
import { lineCounts } from "./format.ts";
import { type EngineOptions, roster } from "./models.ts";
import type { ProviderChoice } from "./providers.ts";
import { isRecorded, Reader, type Reading, row } from "./reading.ts";
import { branchChanges, net } from "./review.ts";
import type { ActRow } from "./session.ts";

/** What one answer of a model read and wrote: its whole prompt, its output, the reads and the writes of the cache,
 * and its dollars. */
export type Usage = NonNullable<Turn[2]>;

/** The models of the roster, each as its name, its efforts and its window. */
export type Roster = [string, string[], number][];
/** The items of a list from a place on, which take the place of the items that the view holds from that place. */
export interface Tail<T> {
  from: number;
  list: T[];
}

/** A view of a chain. A view that gives the mark of the last snapshot it took takes only what changed since then: the
 * roster when it changed, the words that the program changed, and the tails of the turns and of the answers. */
export interface Snapshot {
  /** The mark of this snapshot, which the session gives with its next request once it holds this snapshot. */
  mark: number;
  paused: boolean;
  /** The cause that the pause over the chain gave, null when none stands or none was given. */
  cause: import("furb/life").PauseCause | null;
  roster?: Roster;
  /** The acts that changed after the count the session asked with. */
  acts: ActRow[];
  /** The count of changes of the act table that the acts are read at. */
  count: number;
  selected: string;
  turns: Tail<Turn>;
  /** The usage of each answer of the chain, in order. */
  answers: Tail<Usage>;
  /** The share of the window that the last answer of the chain filled. Nothing before the first answer. */
  filled?: number;
  /** The word of each rung of the program, whole, or the words that changed since the snapshot of the mark given. */
  program?: { whole: boolean; words: Record<string, string> };
  actor: string;
  directory: string;
}

/** What a chain stands on: its roster, its directory and its actor. */
type Standing = [Roster, string, string];

/** The named verbs that a window says to a life: to the daemon that holds it, or to a record open to read, which
 * refuses each verb that would change it. */
export type Engine = Pick<
  RemoteLife,
  | "root"
  | "disposed"
  | "prompt"
  | "rung"
  | "chain"
  | "bash"
  | "grant"
  | "stand"
  | "result"
  | "outcome"
  | "peek"
  | "get"
  | "pause"
  | "wake"
  | "cancel"
  | "close"
  | "read"
  | "write"
  | "transcript"
  | "turns"
  | "program"
  | "cwd"
  | "names"
  | "inspect"
>;

/** How long a recorded life waits before it reads its file again, in milliseconds. */
const FOLLOW = 100;

/** Why a record open to read refuses a verb. */
const READS = "This record is open to read: no word runs, no model is asked, and nothing is kept.";

/** A record open to read, with no life, as the named verbs of a life read it: each view of the record, and a refusal
 * of each verb that would change it. A recorded life follows its file. */
class ReadLife extends EventEmitter {
  readonly root: string;
  readonly disposed = false;
  readonly lost = "";
  readonly daemon = undefined;
  readonly streams = new Map<string, Stream>();
  /** The pace at which a recorded life reads its file for the lines that it added. */
  private readonly following?: ReturnType<typeof setInterval>;
  constructor(readonly reader: Reader) {
    super();
    this.root = reader.root;
    // A recorded life follows its file: each fact that a later line adds is heard as the life said it, and the view
    // hears of it as it hears of a life that runs. The file is read at a pace and not at the event of a watch: on
    // Windows the watch says nothing of an append that comes within a millisecond of the one before.
    if (reader.recorded)
      this.following = setInterval(() => {
        try {
          if (reader.follow()) this.emit("facts");
        } catch {
          // A file that went away, with its folder, adds no line more.
          clearInterval(this.following);
        }
      }, FOLLOW).unref();
  }
  /** The acts that changed after a count of changes, as the daemon gives them, each with the count of the lines of
   * the text and of the characters of the output that the reader left out of it, and the count now. */
  since(count: number): { count: number; acts: ActRow[] } {
    const { activity } = this.reader;
    const changed = activity.since(count);
    const counted = (act: Act): ActRow => {
      const { count: lines, output } = (activity.acts.get(act.id) ?? {}) as {
        count?: number;
        output?: number;
      };
      return {
        ...act,
        ...(lines === undefined ? {} : { count: lines }),
        ...(output === undefined ? {} : { output }),
      };
    };
    return { count: changed.count, acts: activity.hosted(changed.acts).map(counted) };
  }
  /** An act as the daemon gives it, and nothing for a name of no act that a person follows. */
  act(id: string): Act | undefined {
    const act = this.reader.activity.acts.get(id);
    return act && this.reader.activity.hosted([act]).find((one) => one.id === id);
  }
  scope(id: string): string {
    return this.reader.activity.scope(id);
  }
  /** The threads of a chain as the crate folds them from the record, as the daemon folds them for a life that runs. */
  async threads(on: string, at?: number, read?: number): Promise<Threads> {
    return this.reader.meaning.threads(on, at, read);
  }
  /** The threads of every chain of the record but those that `hidden` names, as one tree, as the crate folds them. */
  async rail(hidden: string[], at?: number): Promise<ThreadNode[]> {
    return this.reader.meaning.rail(hidden, at);
  }
  get seenAt(): ReadonlyMap<string, number> {
    return this.reader.activity.seen;
  }
  get compactions(): readonly Compaction[] {
    return this.reader.activity.compactions;
  }
  get completed(): number {
    return this.reader.activity.completed;
  }
  get cost(): number {
    return this.reader.activity.cost;
  }
  /** What the answers of the models cost each chain, by the chain. */
  get spends(): ReadonlyMap<string, Spend> {
    const { activity } = this.reader;
    const kept = this.chainSpends;
    if (kept?.at === activity.spendChanges) return kept.spends;
    const spends = new Map([...activity.spends].filter(([id]) => activity.acts.get(id)?.kind === "chain"));
    this.chainSpends = { at: activity.spendChanges, spends };
    return spends;
  }
  /** How many times a spend changed, which tells a view when to read the spends again. */
  get spendChanges(): number {
    return this.reader.activity.spendChanges;
  }
  private chainSpends?: { at: number; spends: Map<string, Spend> };
  /** What a command that had no end when the record ended printed. */
  output(id: string): { stdout: string; stderr: string } | undefined {
    const act = this.reader.activity.acts.get(id);
    if (act?.kind !== "bash" || act.done) return undefined;
    const exit = act.value as Exit | null;
    return { stdout: exit?.stdout?.content ?? "", stderr: exit?.stderr?.content ?? "" };
  }
  get facts(): readonly Fact[] {
    return this.reader.facts;
  }
  private refuse = async (): Promise<never> => this.reader.refuse();
  prompt = this.refuse;
  rung = this.refuse;
  chain = this.refuse;
  bash = this.refuse;
  grant = this.refuse;
  stand = this.refuse;
  config = this.refuse;
  settings = this.refuse;
  pause = this.refuse;
  wake = this.refuse;
  cancel = this.refuse;
  close = this.refuse;
  write = this.refuse;
  read = this.refuse;
  answer = this.refuse;
  seen = async (): Promise<void> => {};
  queue = this.refuse;
  unqueue = this.refuse;
  readonly queues = new Map<string, FollowUp[]>();
  pin = this.refuse;
  unpin = this.refuse;
  names = async (): Promise<string[]> => [];
  async result(id: string): Promise<unknown> {
    const { done, value } = this.reader.outcome(id);
    if (!done) throw new Error(READS);
    return value;
  }
  async outcome(id: string): Promise<{ done: boolean; value: unknown }> {
    return this.reader.outcome(id);
  }
  async peek(id: string): Promise<unknown> {
    return this.reader.outcome(id).value;
  }
  /** The fact that opened an act, as the record holds it, or nothing for an act that it never opened. */
  async get(id: string): Promise<unknown> {
    return this.reader.facts.find(([, about]) => about === id) ?? null;
  }
  async transcript({ on }: { on?: string } = {}): Promise<Fact[]> {
    return this.reader.transcript({ on });
  }
  async turns({ on }: { on?: string } = {}): Promise<Turn[]> {
    return this.reader.turns({ on });
  }
  async program({ on }: { on?: string } = {}): Promise<Record<string, string>> {
    return this.reader.program({ on });
  }
  async cwd({ on }: { on?: string } = {}): Promise<string> {
    return this.reader.cwd({ on });
  }
  async standing(): Promise<Standing> {
    return this.reader.standing() as unknown as Standing;
  }
  async inspect(name: string): Promise<{ kind: string; representation: string; value: unknown }> {
    const { value } = this.reader.inspect(name);
    return { kind: typeof value, representation: String(value), value };
  }
  async pending(): Promise<Map<string, string>> {
    return new Map();
  }
  /** The share of the window of its model that an answer filled, as filled of the engine gives it, for a record that
   * no engine holds. */
  async filled(id: string): Promise<number | undefined> {
    const turn = this.reader.outcome(id).value;
    if (!Array.isArray(turn) || !Array.isArray(turn[2])) return undefined;
    const to = String(this.reader.activity.acts.get(id)?.words[0] ?? "");
    const roster = ((await this.standing())[0] ?? []) as Roster;
    const window =
      roster.find(
        ([name, efforts]) => to === name || efforts.some((effort) => to === `${name}/${effort}`),
      )?.[2] ?? WINDOW;
    return Math.round((Number(turn[2][0]) / window) * 100) / 100;
  }
  async detach(): Promise<void> {
    clearInterval(this.following);
    this.reader.dispose();
  }
}

/** A life as a window holds it: through the daemon, or a record open to read. */
type Held = RemoteLife | ReadLife;

/** What a snapshot knows of one chain. A value that it must read again is undefined. */
interface ChainView {
  roster?: Roster;
  program?: Record<string, string>;
  /** The words that the runs of the chain gave since the last snapshot, or every word when the program was read
   * again whole. */
  worded?: { whole: boolean; words: Record<string, string> };
  turns?: Turn[];
  directory?: string;
  actor?: string;
}

/** What the last snapshot gave, which the session holds once it takes that snapshot. */
interface Given {
  mark: number;
  chain: string;
  roster: Roster;
  turns: readonly Turn[];
  answers: number;
}

/** The place of the first turn of a list that differs from the turn at that place in another list. */
function firstChange(held: readonly Turn[], turns: readonly Turn[]): number {
  // The turns of a chain that changed nothing are the list that the session holds, which a long life makes long.
  if (held === turns) return held.length;
  const same = (one: unknown, other: unknown) =>
    one === other || JSON.stringify(one) === JSON.stringify(other);
  let at = 0;
  while (at < held.length && at < turns.length) {
    if (held[at] === turns[at]) {
      at++;
      continue;
    }
    const [one, other] = [held[at] as Turn, turns[at] as Turn];
    if (one.length !== other.length || !one.every((part, place) => same(part, other[place]))) break;
    at++;
  }
  return at;
}

/** Each change of a journal that one of the writes made, or each change when no list of writes is given. */
function* written(changes: FileChanges, writes: string[] | null): Generator<FileChange> {
  const named = writes ? new Set(writes) : undefined;
  // A failed read of even an empty journal must not look like a scope with no changes.
  changes.read(0, 0);
  for (let start = 0; start < changes.length; start += 50)
    for (const change of changes.read(start, 50))
      if (!named || (change.act && named.has(change.act))) yield change;
}

/** A question that waits for the operator. */
type Prompt = { id: string; shape: string; message: string };

/** A life as the session of a window sees it: what the life holds, as the window reads it from its events, a view of
 * each chain as of now, and what the window reads of this machine for the life, as its catalog and its providers. It
 * tells `change` when what it holds changed, `facts` after the facts of a change, `fault`, `log` with a notice,
 * `reset` once the window reads the life again from its start, and `authentication` with the state of a sign-in. */
export class HostView extends EventEmitter {
  completed = 0;
  cost = 0;
  directory: string;
  imageDirectory: string;
  actor = "";
  record?: string;
  /** How many file changes the life holds. */
  changes = 0;
  /** Why the record is open to read, when a life on it was refused for a drift, as on a record of another engine. */
  reading?: Reading;
  /** Whether the life is a recording, which shows its log and runs nothing. */
  recorded?: boolean;
  /** The measure of each chain whose models answered, by the chain. */
  measures: Record<string, Measure> = {};
  /** The life and the count of spend changes that the measures were read at. */
  private spent?: { life: Held; at: number };
  compactions: Compaction[] = [];
  /** The count of facts that the window heard. */
  seq = 0;
  prompts = new Map<string, Prompt>();
  seen = new Map<string, number>();
  streams = new Map<string, Stream>();
  pending = new Map<string, string>();
  /** What this window does in the life, as the daemon welcomed it: a window that reads changes nothing. */
  role: "read" | "write" = "write";
  /** Why this window holds no life now, in plain words, and nothing while the life is open. */
  lost = "";
  /** Whether the life runs in the daemon of this machine, which this window can reach again. */
  local = true;
  /** The named verbs of the life that the view reads now, which a record open to read refuses when it would change
   * it. */
  readonly engine: Engine = ((view: HostView): Engine => ({
    root: "chain1",
    get disposed() {
      return view.life.disposed;
    },
    prompt: (...args) => view.life.prompt(...args),
    rung: (...args) => view.life.rung(...args),
    chain: (...args) => view.life.chain(...args),
    bash: (...args) => view.life.bash(...args),
    grant: (...args) => view.life.grant(...args),
    stand: () => view.life.stand(),
    result: (id) => view.life.result(id),
    outcome: (id) => view.life.outcome(id),
    // An act that is not done peeks as nothing, as peek of the engine gives its waiting value, None, where the daemon
    // refuses the peek. The daemon is asked, since the window may hear the end of an act later than the life made it.
    peek: (id) =>
      view.life.peek(id).catch((error: unknown) => (error instanceof Refused ? null : Promise.reject(error))),
    get: (id) => view.life.get(id),
    pause: (...args) => view.life.pause(...args),
    wake: (id) => view.life.wake(id),
    cancel: (...args) => view.life.cancel(...args),
    close: (...args) => view.life.close(...args),
    read: (...args) => view.life.read(...args),
    write: (...args) => view.life.write(...args),
    transcript: (...args) => view.life.transcript(...args),
    turns: (...args) => view.life.turns(...args),
    program: (...args) => view.life.program(...args),
    cwd: (...args) => view.life.cwd(...args),
    names: (chain) => view.life.names(chain),
    inspect: (name, chain) => view.life.inspect(name, chain),
  }))(this);
  /** The journal of the changes of files of the life, as the daemon writes it beside the record. */
  private journal?: FileChanges;
  private life: Held;
  private unheard?: () => void;
  constructor(
    life: Held,
    private readonly options: EngineOptions,
  ) {
    super();
    this.life = life;
    this.directory = resolve(options.cwd ?? (options.record ? dirname(options.record) : "."));
    this.record = options.record && resolve(options.record);
    this.imageDirectory = this.record ? `${this.record}.images` : resolve(this.directory, ".furb/images");
    this.bind(life);
  }

  /** The life that the view reads, and each of its events heard. */
  private bind(life: Held): void {
    this.unheard?.();
    this.life = life;
    this.journal?.dispose();
    this.journal = this.record ? new FileChanges(this.record, true) : undefined;
    if (life instanceof ReadLife) {
      this.reading = life.reader.reading;
      this.recorded = life.reader.recorded;
      this.directory = life.reader.directory;
    } else this.role = life.daemon.peer?.role ?? "write";
    const facts = () => this.heard();
    const change = () => this.update();
    const log = (line: string) => this.emit("log", line);
    const ended = (why: string) => {
      this.lost = why;
      this.emit("fault", new Error(why));
      this.emit("change");
    };
    const reset = () => {
      this.forget();
      this.lost = "";
      this.emit("reset");
    };
    const heard: [string, Listener][] = [
      ["facts", facts],
      ["change", change],
      ["log", log],
      ["ended", ended],
      ["reset", reset],
    ];
    for (const [name, listener] of heard) life.on(name, listener);
    this.unheard = () => {
      for (const [name, listener] of heard) life.off(name, listener);
    };
    this.forget();
  }

  /** The view of the life read again from its start. */
  private forget(): void {
    this.chains.clear();
    this.answers.clear();
    this.replies.clear();
    this.lines.clear();
    this.measured.clear();
    this.given = undefined;
    this.heardAt = 0;
    this.replayed = this.life.facts.length;
    this.timed = this.replayed;
    this.update();
  }

  /** A record that a build of another engine made drifts on a replay, so it opens to read in its place, with the
   * facts that it kept. */
  readInstead(why: string): boolean {
    const record = this.record;
    if (!record || !why.includes("Drift")) return false;
    const { file, made, engine, logged } = retold(record);
    // The reason of the read is the drift itself, after what the end of the life said before it.
    const drift = why.slice(why.indexOf("Drift"));
    const reader = new Reader(this.directory, record, { made, engine, why: drift, logged }, file);
    this.bind(new ReadLife(reader));
    this.lost = "";
    this.emit("reset");
    return true;
  }

  /** What the view holds after a change of the life. */
  private update(): void {
    const life = this.life;
    this.completed = life.completed;
    this.cost = life.cost;
    this.seq = life.facts.length;
    this.streams = new Map(life.streams);
    // A view keys on the measures, the compactions, the seen and the prompts, as the loom of the life does on the
    // measures: each keeps its object while nothing of it changed, so a fact that changes none of them, as a line of a
    // command, weaves nothing again in a long life.
    if (this.spent?.life !== life || this.spent.at !== life.spendChanges) {
      this.spent = { life, at: life.spendChanges };
      const measures = Object.fromEntries(
        [...life.spends].map(([chain, spend]) => [chain, this.measure(chain, spend)]),
      );
      const held = Object.keys(this.measures);
      if (
        held.length !== Object.keys(measures).length ||
        held.some((chain) => this.measures[chain] !== measures[chain])
      )
        this.measures = measures;
    }
    const { compactions, seenAt: seen } = life;
    if (compactions.length !== this.compactions.length || compactions.at(-1) !== this.compactions.at(-1))
      this.compactions = [...compactions];
    if (seen.size !== this.seen.size || [...seen].some(([id, at]) => this.seen.get(id) !== at))
      this.seen = new Map(seen);
    const prompts = new Map<string, Prompt>();
    if (this.life instanceof RemoteLife)
      for (const act of this.life.acts.values())
        if (act.kind === "prompt" && act.operator && act.phase.phase !== "done")
          prompts.set(act.id, {
            id: act.id,
            shape: String(act.words[0] ?? ""),
            message: String(act.words[1] ?? ""),
          });
    const was = this.prompts;
    if (
      prompts.size !== was.size ||
      [...prompts].some(
        ([id, one]) => was.get(id)?.shape !== one.shape || was.get(id)?.message !== one.message,
      )
    )
      this.prompts = prompts;
    this.emit("change");
  }

  /** The measure of a chain from what the answers of its models cost, as the life gives it: it keeps its object while
   * the spend of the chain stays the same. */
  private measure(chain: string, spend: Spend): Measure {
    const known = this.measured.get(chain);
    if (known?.spend === spend) return known.measure;
    const measure = {
      dollars: spend.dollars,
      spent: spend.dollars,
      answers: spend.answers,
      ...(spend.actor ? { actor: spend.actor } : {}),
    };
    this.measured.set(chain, { spend, measure });
    return measure;
  }

  /** The facts of a change: the view holds them, and the work that waits for a wake is read again. */
  private heard(): void {
    this.time();
    this.update();
    if (this.journal) {
      this.journal.readable(0, 0);
      this.changes = this.journal.length;
    }
    this.emit("facts");
    void this.life.pending().then(
      (pending) => {
        this.pending = pending;
        this.emit("change");
      },
      () => {},
    );
  }

  /** When this window saw each act begin, and how long each act that it saw begin took to be done. The record holds
   * no time, so an act that the life told as the window attached took no time that it knows. */
  private readonly began = new Map<string, number>();
  private readonly took = new Map<string, number>();
  private replayed = 0;
  /** The measure of each chain, and the spend that it reads. */
  private readonly measured = new Map<string, { spend: Spend; measure: Measure }>();
  private timed = 0;
  private time(): void {
    const now = clock();
    const facts = this.life.facts;
    for (; this.timed < facts.length; this.timed++) {
      const [kind, id] = facts[this.timed] as Fact;
      const began = this.began.get(id);
      if (isQuestion(kind, id)) this.began.set(id, now);
      else if (kind === "done" && began !== undefined) {
        this.began.delete(id);
        this.took.set(id, now - began);
      }
    }
  }

  private readonly chains = new Map<string, ChainView>();
  /** The usage of each answer of each chain, by the chain that the reply stands on. */
  private readonly answers = new Map<string, Usage[]>();
  /** The last reply of each chain that an answer settled. */
  private readonly replies = new Map<string, string>();
  /** The lines that each write added and removed, by the write. */
  private readonly lines = new Map<string, NonNullable<ActRow["lines"]>>();
  private heardAt = 0;
  private given?: Given;
  private marks = 0;

  /** What a fact changes of the views of the chains, which each read again what it changed. */
  private changed(fact: Fact): void {
    const [kind, id] = fact;
    const answered = kind === "done" ? questionKind(id) : undefined;
    // A word may bind the actor of its chain again, so the actor is read again at each step of a word.
    if (
      ["run", "writing", "ran", "ready", "wants"].includes(kind) ||
      ["run", "writing", "wants"].includes(answered ?? "")
    )
      for (const cached of this.chains.values()) cached.actor = undefined;
    // The standing has one home, the root, and every chain that takes it tells it and holds its actor.
    if (answered === "stand")
      for (const cached of this.chains.values())
        Object.assign(cached, {
          roster: undefined,
          directory: undefined,
          actor: undefined,
          turns: undefined,
        });
    const turn = fact[3];
    const scope = this.life.scope(id);
    if (answered === "reply" && Array.isArray(turn) && Array.isArray(turn[2])) {
      const answers = this.answers.get(scope) ?? [];
      answers.push(usageOf(turn[2]));
      this.answers.set(scope, answers);
      this.replies.set(scope, id);
    }
    const view = this.chains.get(scope);
    if (!view) return;
    if (
      ["tell", "pause", "wake", "cancel", "close", "reply", "prefix"].includes(kind) ||
      answered === "reply"
    )
      view.turns = undefined;
    // A run adds its word to the program as the program reads it.
    if (kind === "run" && isQuestion(kind, id) && view.program) {
      const [rung, word, donor] = fact.slice(4).map(String);
      const key = donor || rung || "";
      view.program[key] = word ?? "";
      view.worded ??= { whole: false, words: {} };
      view.worded.words[key] = word ?? "";
    } else if (kind === "run" || kind === "module" || kind === "prefix") view.program = undefined;
    if (kind === "cd") view.directory = undefined;
    // A write of the door of a prompt edits the program of its ladder and replays it.
    const wrote = kind === "write" ? fact[4] : answered === "write" ? fact[3] : undefined;
    if (
      wrote &&
      typeof wrote === "object" &&
      isQuestion("prompt", String((wrote as { path?: unknown }).path))
    ) {
      view.turns = undefined;
      view.program = undefined;
      view.actor = undefined;
    }
  }

  /** The view of a chain, with the acts that changed after a count of changes of the act table. A session that holds
   * the snapshot of a mark takes only the parts of the chain that changed since that snapshot. */
  async snapshot(requested: string, since = 0, held = 0): Promise<Snapshot> {
    const life = this.life;
    try {
      return await this.snapshotOf(life, requested, since, held);
    } catch (error) {
      // A life that drifted on its record ended while the view read it, and the record opened to read in its place,
      // which the view reads from its start.
      if (this.life !== life) return await this.snapshot(requested, 0, 0);
      throw error;
    }
  }

  private async snapshotOf(life: Held, requested: string, since: number, held: number): Promise<Snapshot> {
    const facts = life.facts;
    for (const fact of facts.slice(this.heardAt)) this.changed(fact);
    const start = facts.length;
    const selected = life.act(requested)?.kind === "chain" ? requested : life.root;
    const view = this.chains.get(selected) ?? {};
    this.chains.set(selected, view);
    view.roster ??= ((await life.standing())[0] ?? []).map(([name, efforts, window]) => [
      name,
      efforts,
      window,
    ]);
    if (!view.program) {
      view.program = { ...(await life.program({ on: selected })) };
      view.worded = { whole: true, words: view.program };
    }
    view.turns ??= await life.turns({ on: selected });
    view.directory ??= expandHome(await life.cwd({ on: selected }));
    view.actor ??= String((await life.inspect("actor", selected)).value ?? "");
    // The actor of the root is the actor of the life, which a chain with no actor of its own takes.
    if (selected === life.root) this.actor = view.actor;
    // A fact that came while the views were read changes them at the next snapshot, which the session takes then.
    this.heardAt = start;
    // The daemon writes the journal after it tells the write, so the counts of a write may come after its act: the
    // act whose counts came is given again with them.
    const counted = new Set<string>();
    for (const change of this.journal?.readable(0, this.journal.length) ?? [])
      if (change.act && !this.lines.has(change.act)) {
        this.lines.set(
          change.act,
          lineCounts(createTwoFilesPatch(change.path, change.path, change.before, change.after)),
        );
        counted.add(change.act);
      }
    const { acts, count } = life.since(since);
    for (const act of acts) counted.delete(act.id);
    for (const id of counted) {
      const act = life.act(id);
      if (act) acts.push(act);
    }
    const given = this.given?.mark === held && this.given.chain === selected ? this.given : undefined;
    const answers = this.answers.get(selected) ?? [];
    const reply = this.replies.get(selected);
    const filled = reply ? await life.filled(reply) : undefined;
    const from = { turns: given ? firstChange(given.turns, view.turns) : 0, answers: given?.answers ?? 0 };
    const program = given ? view.worded : { whole: true, words: view.program };
    view.worded = undefined;
    const mark = ++this.marks;
    this.given = { mark, chain: selected, roster: view.roster, turns: view.turns, answers: answers.length };
    const chain = life.act(selected);
    return {
      mark,
      selected,
      // Each row is a copy of its act as it stands now, and the session finds what changed by the rows that it held
      // before.
      acts: acts.map((act) =>
        row(structuredClone(act), this.lines.get(act.id), this.took.get(act.id), life.output(act.id)),
      ),
      count,
      paused: chain ? isPaused(chain) : false,
      cause: (chain && isPaused(chain) && chain.pause) || null,
      ...(given?.roster === view.roster ? {} : { roster: view.roster }),
      ...(program ? { program } : {}),
      turns: { from: from.turns, list: view.turns.slice(from.turns) },
      answers: { from: from.answers, list: answers.slice(from.answers) },
      filled,
      directory: view.directory,
      actor: view.actor,
    };
  }

  /** The demo asks no model, so it reads nothing of the providers of this machine. */
  private get demo(): boolean {
    return Boolean(this.options.demo || this.options.watch);
  }
  private async refreshCodex(): Promise<void> {
    const { claude, config } = this.options;
    if (
      this.demo ||
      !setup(claude, this.directory, config).providers.some((one) => one.provider === "codex" && one.enabled)
    )
      return;
    let status = codexAuth("refresh", config);
    while (status.state === "connecting") {
      await Bun.sleep(200);
      status = codexAuth("status", config);
    }
  }
  /** The models the catalog of the crate offers, each as its name, its efforts and its window. */
  async catalog(): Promise<Roster> {
    if (this.demo) return [];
    await this.refreshCodex();
    return models(this.options.claude, this.directory, this.options.config).map(
      ({ name, efforts, window }) => [name, efforts, window],
    );
  }
  /** What the first run checks of this machine, and nothing in the demo, which needs no model. */
  async setup(): Promise<SetupInfo | null> {
    return this.demo ? null : setup(this.options.claude, this.directory, this.options.config);
  }
  async providers(): Promise<ProviderChoice[]> {
    if (this.demo) return [{ name: "claude-cli", enabled: true, models: 4 }];
    const found = setup(this.options.claude, this.directory, this.options.config);
    const offered = found.providers.map((one) => ({
      name: one.provider,
      enabled: one.enabled,
      models: one.offered,
    }));
    if (!offered.some((one) => one.name === "claude-cli"))
      offered.push({ name: "claude-cli", enabled: true, models: found.claude ? 4 : 0 });
    if (!offered.some((one) => one.name === "local"))
      offered.push({ name: "local", enabled: true, models: found.local?.offered ?? 0 });
    return offered.sort((a, b) => a.name.localeCompare(b.name));
  }
  /** The sign-in of codex that a change of its provider asks: a cancel when it turns codex off, and a sign-in, which
   * waits for the browser, when it turns codex on. False when the operator canceled the sign-in. */
  private async codexSigned(value: unknown): Promise<boolean> {
    const { config } = this.options;
    if (value === false) codexAuth("cancel", config);
    if (value !== true) return true;
    let status = codexAuth("refresh", config);
    if (["disconnected", "needs-login", "failed", "canceled"].includes(status.state))
      status = codexAuth("start", config);
    this.emit("authentication", status);
    while (["waiting", "connecting"].includes(status.state)) {
      await Bun.sleep(200);
      const next = codexAuth("status", config);
      if (next.state !== status.state) this.emit("authentication", next);
      status = next;
    }
    if (status.state === "canceled") return false;
    if (status.state !== "connected") throw new Error(status.message || "OpenAI sign-in did not complete.");
    return true;
  }
  codexAuth = async (action = "status") => codexAuth(action, this.options.config);
  /** A change of a key of a layer of the configs, which the World writes in the daemon, or a reload of them, each then
   * a stand of the life. False when the operator canceled the sign-in that the change asked. */
  async config(config: Config): Promise<boolean> {
    if (this.demo) throw new Error("Provider settings are not available in the demo.");
    if (config.action === "get") throw new Error("A window reads the configs from the host.");
    if (config.action === "set") {
      const [group, name] = config.key;
      if (group === "providers" && name === "codex" && !(await this.codexSigned(config.value))) return false;
    }
    await this.life.result(await this.life.config(config));
    return true;
  }
  /** Each key of the configs of the folder of the life, with the value that wins for it and the property that a schema
   * gives it, as the host reads them, and none in the demo. */
  async settings(): Promise<Setting[]> {
    return this.demo ? [] : await this.life.settings();
  }
  /** A stand of the life on the configs as the disk holds them, which asks again each model that waits to retry. */
  async stand(): Promise<void> {
    await this.refreshCodex();
    await this.life.result(await this.life.stand());
  }
  /** The name of the model that a name gives in the roster or in the catalog, and nothing when it gives none. */
  async model(name: string): Promise<string | null> {
    const names = ((await this.life.standing())[0] ?? []).map(([one]) => one);
    const found = model(name, this.directory, this.options.config);
    const held = found?.name ?? name;
    const offered =
      names.includes(held) ||
      (!this.demo &&
        models(this.options.claude, this.directory, this.options.config).some((one) => one.name === held));
    return offered ? held : null;
  }
  /** Whether the model of an actor takes an image, as the catalog of the crate says. */
  async sees(actor: string): Promise<boolean> {
    const names = ((await this.life.standing())[0] ?? []).map(([one]) => one);
    return model(actorParts(actor, names).model, this.directory, this.options.config)?.images ?? false;
  }
  async source(): Promise<string> {
    return ENGINE;
  }
  answer = (id: string, value: string) => this.life.answer(id, value);
  /** The words of the operator in place of each value that a question offers, which close it as Refused. */
  decline = (id: string, words: string) =>
    this.life.close(JSON.stringify({ is: "Refused", args: [words] }), { id });
  /** The operator read a chain or a thread up to a place, which the record keeps as a fact seen. */
  async see(about: string, place: number): Promise<null> {
    if (place > (this.life.seenAt.get(about) ?? 0)) await this.life.seen(about);
    return null;
  }
  async attachImage(path: string): Promise<ImageAttachment> {
    if (this.role === "read" || this.reading) throw new Error("Record inspection cannot attach an image.");
    return attachImage(this.imageDirectory, resolve(this.directory, path));
  }
  /** The work that waits for a wake started again: a wake of each chain that holds some. */
  async resume(): Promise<void> {
    const chains = new Set([...this.pending.keys()].map((id) => this.life.scope(id)));
    for (const chain of chains) if (chain) await this.life.wake(chain);
  }
  private get files(): FileChanges {
    if (!this.journal) throw new Error("This life keeps no record, so it keeps no journal of file changes.");
    return this.journal;
  }
  /** A page of the changes of the journal, as the rewind reads them, past each line that is no change. */
  readChanges = async (start: number, count: number) => this.files.readable(start, count);
  /** The net change of each file across the changes that the writes made, or across every change of the life. */
  netChanges = async (writes: string[] | null) => (this.journal ? net(written(this.journal, writes)) : []);
  /** The change of each file of the branch against main, as git holds it in the directory of the session. */
  branchChanges = async () => branchChanges(this.directory);
  /** The journal written again with each change that it holds, and the counts of changes kept and of lines removed. */
  async repairChanges(): Promise<Repaired> {
    const writable = new FileChanges(this.record);
    try {
      return writable.repair();
    } finally {
      writable.dispose();
      this.heard();
    }
  }
  /** A new journal of file changes, and the path that the damaged one is kept at. */
  async renewChanges(): Promise<string> {
    const writable = new FileChanges(this.record);
    try {
      return writable.renew();
    } finally {
      writable.dispose();
      this.heard();
    }
  }
  /** An act whole, with all that a command printed, and nothing when the life holds no such act. */
  /** The threads of a chain as the crate folds them, now or as they stood just after a moment, with the runs of its
   * notes counted past the place up to which the window read it. */
  async threads(chain: string, at?: number, read?: number): Promise<Threads> {
    return await this.life.threads(chain, at, read);
  }
  /** The threads of every chain of the life but those that `hidden` names, as one tree in the order of the list of
   * the threads, as the crate folds them, now or as they stood just after a moment. */
  async rail(hidden: string[], at?: number): Promise<ThreadNode[]> {
    return await this.life.rail(hidden, at);
  }
  async act(id: string): Promise<Act | undefined> {
    if (!(this.life instanceof ReadLife)) return this.life.acts.get(id);
    const whole = this.life.reader.act(id);
    return whole && this.life.reader.activity.hosted([whole]).find((one) => one.id === id);
  }
  /** The text at a path, from where a chain stands, which makes no act and keeps nothing. */
  async look(path: string, chain: string): Promise<{ path: string; content: string }> {
    const at = resolve(this.directory, expandHome(await this.life.cwd({ on: chain })), expandHome(path));
    try {
      return { path: at, content: readFileSync(at, "utf8") };
    } catch {
      throw new Error(`There is no file at ${at}.`);
    }
  }
  /** The window reaches the life again, and reads it from its start. */
  async reopen(): Promise<void> {
    if (!(this.life instanceof RemoteLife))
      throw new Error("This record is open to read, so it has no life to reach.");
    await this.life.open();
    this.lost = "";
    this.forget();
    this.emit("reset");
  }
  /** The life pinned under the name of a holder, so it lives with no window, until an unpin of that holder. */
  /** The follow-ups that wait on each chain of the life, in order, as the life told them last. */
  get queued(): FollowUp[] {
    return [...this.life.queues.values()].flat();
  }
  /** A message put in the queue of a chain, which the life sends once the chain rests. A second queue of the same
   * entry changes nothing. */
  queue = (shape: string, message: string, on: string, entry: string = randomUUID()) =>
    this.life.queue(entry, shape, message, on);
  /** A follow-up taken out of its queue before the life sent it. */
  unqueue = (entry: string) => this.life.unqueue(entry);
  pin = (holder: string) => this.life.pin(holder);
  unpin = (holder: string) => this.life.unpin(holder);
  /** The window lets the life go: the life ends once no window and no pin holds it, after the grace of the daemon. */
  async dispose(): Promise<void> {
    this.unheard?.();
    this.journal?.dispose();
    await this.life.detach();
  }
}

/** The life of a record, as a window holds it: in the daemon of this machine, or open to read for a recorded life, and
 * a life that drifts on its record opens to read in its place. The demo answers the models of its life in the window.
 * A life that the open makes stands on the model and the roster that the window names. Each request of the view waits
 * for the open, in the order it was made, and ready tells when the life is open, or why it is not. */
export function openingEngine(options: EngineOptions): {
  engine: Engine;
  host: HostView;
  ready: Promise<void>;
} {
  const record = options.record && resolve(options.record);
  if (record && isRecorded(record)) {
    const life = new ReadLife(Reader.recorded(record, resolve(options.cwd ?? dirname(record))));
    const host = new HostView(life, options);
    return { engine: host.engine, host, ready: Promise.resolve() };
  }
  // The demo of a folder keeps its record in it, and each other life names its record.
  const kept =
    record ?? (options.demo || options.watch ? resolve(options.cwd ?? ".", "demo.jsonl") : undefined);
  if (!kept) throw new Error("A life of the daemon needs a record.");
  const choices: Choices = {
    cwd: options.cwd && resolve(options.cwd),
    actor: [options.model, options.effort].filter(Boolean).join("/") || undefined,
    roster: roster(options),
    extensions: options.demo ? options.extensions === true : options.extensions,
    // The life reads the settings and runs the claude command line that this window reads, whatever the environment
    // of the daemon names.
    config: options.config ?? process.env.FURB_CONFIG_DIR,
    claude: options.claude ?? process.env.FURB_CLAUDE_BIN,
    models: options.demo || options.watch ? demoModels(options.watch) : undefined,
  };
  const life = new RemoteLife(daemon(), kept, choices);
  const host = new HostView(life, { ...options, record: kept });
  const ready = life.open().then(
    () => {
      host.role = life.daemon.peer?.role ?? "write";
      // The folder of the life stands beside its record, where a later --resume of the record reads it.
      if (host.role === "write")
        saveFile(`${kept}.session.json`, JSON.stringify({ options: { cwd: host.directory } }));
    },
    (error: unknown) => {
      const why = error instanceof Error ? error.message : String(error);
      if (!host.readInstead(why)) throw error;
    },
  );
  life.once("ended", (why: string) => host.readInstead(why));
  return { engine: host.engine, host, ready };
}

/** The life of a record once it is open, or why it did not open. */
export async function openEngine(options: EngineOptions): Promise<{ engine: Engine; host: HostView }> {
  const { engine, host, ready } = openingEngine(options);
  await ready;
  return { engine, host };
}
