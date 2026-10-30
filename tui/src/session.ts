import { randomUUID } from "node:crypto";
import { EventEmitter } from "node:events";
import { readFileSync } from "node:fs";
import { mkdir, writeFile } from "node:fs/promises";
import { basename, dirname, isAbsolute, join, relative, resolve } from "node:path";
import type { FileChange, ImageAttachment, LiveAct, SetupInfo, Turn } from "furb";
import {
  actorParts,
  decodeRecord,
  enabled,
  furbDirectory,
  imagePath,
  imageReference,
  imageReferences,
  ROOT,
  saveFile,
  shapes,
  WORK,
} from "furb";
import type { PauseCause } from "furb/life";
import {
  type ActIndex,
  type Ask,
  aimed,
  beside,
  cancelled,
  compactedRun,
  type Desk,
  desk,
  failed,
} from "furb/life";
import type { Engine, HostView, Usage } from "./bridge.ts";
import { clock } from "./clock.ts";
import { expandHome, fileReferences, fromDirectory, projectFiles, shortenHome } from "./files.ts";
import { dollars } from "./format.ts";
import { append, modeOf, seed, sentIn, workspaceRoot } from "./history.ts";
import { Preferences } from "./preferences.ts";
import { type Hunk, type Reviewed, revertWord, reviewed, type Scope, scopes } from "./review.ts";
import { shareHtml, shareMarkdown } from "./share.ts";
import { isThemeChoice, palettes, type ThemeChoice, type ThemeName, themeOf } from "./theme.ts";
import { asOf, branchNotice } from "./watching.ts";

/** The views of a chain: the feed of what it did, the transcript that the model reads, and the files it changed. */
export const views = ["feed", "transcript", "changes"] as const;
export type View = (typeof views)[number];
/** An act as the views read it. A command that printed more than a row carries holds the tail of each stream, and
 * the length of all it printed; the view reads the whole act when it shows the output. */
export type ActRow = LiveAct & {
  output?: number;
  /** The lines of the text that a read gave or that a write wrote, whose text the row leaves out. */
  count?: number;
  /** The lines that the change of a write added and removed. */
  lines?: { added: number; removed: number };
  /** How many milliseconds the act took, when this life ran it. */
  took?: number;
};
/** The net change of a file in the scope of the review, with its hunks. */
export type ShownChange = Reviewed;
/** What a rewind to a point gives back of the files: for each file that a write changed after the point, the change
 * whose text it replaced first, and how many commands ran after the point, whose changes stay. */
export interface Restore {
  files: FileChange[];
  commands: number;
}
/** A path as a word of the shell, between single quotes. */
const quoted = (path: string) => `'${path.replaceAll("'", "'\\''")}'`;

export type SessionStatus =
  | "saved"
  | "idle"
  | "working"
  | "blocked"
  | "paused"
  | "done"
  | "error"
  | "reading"
  | "opening";
export const statusLabels: Record<SessionStatus, string> = {
  saved: "Saved",
  idle: "Ready",
  working: "Working",
  blocked: "Input needed",
  paused: "Paused",
  done: "Finished, unread",
  error: "Error",
  reading: "Read only",
  opening: "Opening",
};
/** Whether an act is at work: it lives, no pause holds it, and its kind is one whose work takes time. */
export const working = (act: ActRow): boolean =>
  !act.done && !act.paused && !act.unfinished && WORK.includes(act.kind);
/** The phase of an act that a view of the whole life reads: whether it is done or held, when it began and ended, and
 * how its rung ran. The output that a command streams is no part of it. */
function phaseOf(act?: ActRow): string {
  return act
    ? `${act.done}|${act.paused}|${act.began}|${act.ended}|${act.run?.status}|${act.run?.reason}`
    : "";
}

/** The acts of a life by their name, by the chain that they stand on, by their maker, and the chains, which the view
 * reads many times in each frame, with the place of each act in each list, so that an act that changed takes its place
 * with no pass over the list. */
interface Index {
  of: readonly ActRow[];
  ids: Map<string, ActRow>;
  /** The place of each act in the list of the acts, by its name. */
  at: Map<string, number>;
  on: Map<string, ActRow[]>;
  /** The acts that each act made, in the order of the acts. */
  made: Map<string, ActRow[]>;
  chains: ActRow[];
  /** The place of each act in its list of on, of made, and of the chains for a chain, and whether a write is done. */
  places: Map<string, Place>;
  /** The last word of the operator on each chain. */
  words: Map<string, ActRow>;
  /** The acts that are not done, and the rungs whose run runs, which the thread of a chain keeps shown. */
  live: Map<string, ActRow>;
  /** How many times an act that each act made came or changed, which tells the thread when to read a step again, and
   * how many times an act on each chain came or changed, which tells the board when to read a chain again. */
  stamps: Map<string, number>;
  chained: Map<string, number>;
  /** How many times a prompt came or changed, and for each chain that count when a prompt on it last did: the threads of
   * a chain depend on its prompts alone, so an act of another kind reads no thread again. */
  threaded: Map<string, number>;
  prompts: number;
  /** The chain of each act that came or changed, once each time, which tells the board which chains to read again. */
  touched: string[];
  /** The writes that are done, in the order that the session saw them done. */
  written: ActRow[];
  /** The acts on each chain but its chains, as activity reads them, which only grows. */
  work: Map<string, ActRow[]>;
  /** The last act on each chain that is no chain and no grant, and the chains whose last act failed. */
  latest: Map<string, ActRow>;
  failing: Set<string>;
}

/** The place of an act in each list of an index that holds it, and whether it is among the writes that are done. */
interface Place {
  on: number;
  made: number;
  work?: number;
  chain?: number;
  written: boolean;
}

/** An act that changed, or a new one, in its place in each list of an index. */
function placed(index: Index, act: ActRow): void {
  const at = index.places.get(act.id);
  const into = (list: ActRow[], place?: number) => {
    if (place === undefined) return list.push(act) - 1;
    list[place] = act;
    return place;
  };
  const listed = (lists: Map<string, ActRow[]>, key: string) => {
    const list = lists.get(key) ?? [];
    lists.set(key, list);
    return list;
  };
  const written = Boolean(at?.written) || (act.kind === "write" && act.done);
  if (written && !at?.written) index.written.push(act);
  index.places.set(act.id, {
    on: into(listed(index.on, act.on), at?.on),
    made: into(listed(index.made, act.by), at?.made),
    ...(act.kind === "chain"
      ? { chain: into(index.chains, at?.chain) }
      : { work: into(listed(index.work, act.on), at?.work) }),
    written,
  });
  if (!act.done || act.run?.status === "running") index.live.set(act.id, act);
  else index.live.delete(act.id);
  index.stamps.set(act.by, (index.stamps.get(act.by) ?? 0) + 1);
  index.chained.set(act.on, (index.chained.get(act.on) ?? 0) + 1);
  if (act.kind === "prompt") index.threaded.set(act.on, ++index.prompts);
  index.touched.push(act.on);
  const latest = index.latest.get(act.on);
  if (
    act.kind !== "chain" &&
    act.kind !== "grant" &&
    (!latest || (index.at.get(latest.id) ?? 0) <= (index.at.get(act.id) ?? 0))
  ) {
    index.latest.set(act.on, act);
    if (failed(act)) index.failing.add(act.on);
    else index.failing.delete(act.on);
  }
  if (act.kind !== "rung" || act.by !== "operator") return;
  const word = index.words.get(act.on);
  if (!word || (index.at.get(word.id) ?? 0) <= (index.at.get(act.id) ?? 0)) index.words.set(act.on, act);
}

/** The running work that a cancel of an act ends, and the chains that it stands on. */
export interface Reach {
  acts: ActRow[];
  chains: string[];
}
/** A cancel of an act: the work that it ends, and whether it takes back the last message of the operator. */
export interface Cancel {
  act: ActRow;
  reach: Reach;
  back: boolean;
}

export interface FollowUp {
  id: string;
  chain: string;
  text: string;
  shape: string;
}

/** Where a view stands: the end of a view that follows its end while it stands there, and an offset otherwise. */
export type Scroll = number | "end";

/** The fields of a session that `<record>.ui.json` keeps, so that a later open shows the session as it was left. */
const kept = [
  "sessionName",
  "demo",
  "selected",
  "actor",
  "view",
  "scope",
  "mode",
  "shape",
  "editing",
  "histories",
  "stashes",
  "started",
  "changedAt",
  "took",
  "queued",
  "queueError",
  "images",
  "redo",
  "drafts",
  "scrolls",
  "folds",
  "models",
] as const;
/** What `<record>.ui.json` holds: the kept fields of a session, and what the session had cost. */
export type SavedView = Partial<Pick<Session, (typeof kept)[number]>> & { cost?: number };

/** The form of each field that `<record>.ui.json` keeps, which a field must have to be read: a text, a flag, a
 * number, a map, a list of texts, or a list of maps. */
const forms: Record<(typeof kept)[number] | "cost", "text" | "flag" | "number" | "map" | "texts" | "maps"> = {
  sessionName: "text",
  demo: "flag",
  selected: "text",
  actor: "text",
  view: "text",
  scope: "text",
  mode: "text",
  shape: "text",
  editing: "text",
  histories: "map",
  stashes: "map",
  started: "map",
  changedAt: "map",
  took: "map",
  queued: "maps",
  queueError: "text",
  images: "map",
  redo: "maps",
  drafts: "map",
  scrolls: "map",
  folds: "map",
  models: "texts",
  cost: "number",
};

/** Whether a value has a form. */
function formed(value: unknown, form: (typeof forms)[keyof typeof forms]): boolean {
  const map = (one: unknown) => Boolean(one) && typeof one === "object" && !Array.isArray(one);
  switch (form) {
    case "text":
      return typeof value === "string";
    case "flag":
      return typeof value === "boolean";
    case "number":
      return typeof value === "number";
    case "map":
      return map(value);
    case "texts":
      return Array.isArray(value) && value.every((one) => typeof one === "string");
    case "maps":
      return Array.isArray(value) && value.every(map);
  }
}

/** The saved view of a record: nothing when it has none, the defaults with the reason when its file holds no view
 * that can be read, and the view less each field of another form, with the names of those fields. */
export function savedView(record: string): { view: SavedView; damage?: string } {
  const path = `${record}.ui.json`;
  try {
    const view: unknown = JSON.parse(readFileSync(path, "utf8"));
    if (!view || typeof view !== "object" || Array.isArray(view)) throw new Error("It holds no map.");
    const held = view as Record<string, unknown>;
    const wrong = Object.entries(forms).flatMap(([key, form]) =>
      key in held && !formed(held[key], form) ? [key] : [],
    );
    for (const key of wrong) delete held[key];
    if (!wrong.length) return { view: held };
    const fields = wrong.join(", ");
    return {
      view: held,
      damage: `${path} holds ${fields} in a form this release does not read: each opened empty.`,
    };
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return { view: {} };
    const reason = error instanceof Error ? error.message : String(error);
    return {
      view: {},
      damage: `Could not read ${path}: ${reason.replace(/\.?$/, ".")} The session opened with its default view.`,
    };
  }
}

export class Session extends EventEmitter {
  selected: string;
  view: View = "feed";
  actor: string;
  sessionName: string;
  acts: ActRow[] = [];
  /** The count of the changes of the acts that a view of the whole life reads: an act that comes, or an act that
   * starts, ends, pauses or wakes. An act whose output grows alone keeps it, so the board and the desk of a life
   * whose command streams are not read again. */
  version = 0;
  turns: Turn[] = [];
  /** The usage of each answer of the chain itself, which the turns of a chain with a source do not tell apart. */
  answers: Usage[] = [];
  /** The share of the window that the last answer of the chain filled, as filled of the engine gives it, which the
   * advance binds too. Nothing before the first answer. */
  filled?: number;
  /** The net change of each file in the scope of the review. */
  changes: ShownChange[] = [];
  /** Which changes the view of changes reviews: this turn, this chain, the life, or the branch against main. */
  scope: Scope = "chain";
  program: Record<string, string> = {};
  /** The text of the search box, which the rows of the view must hold. */
  search = "";
  private message = "";
  private noticeTimer?: ReturnType<typeof setTimeout>;
  readonly errors: Record<string, string> = {};
  loading = false;
  paused = false;
  cause: PauseCause | null = null;
  editing?: string;
  /** The thread that the operator chose in the sidebar, to which the input sends each message as prompt(None, message,
   * on=thread): its model reads it at its next step, and a message to a resolved thread opens it again. */
  thread?: string;
  /** The thread lit in the sidebar, to which the input sends a message by default, as long as the operator chose no
   * thread and did not ask for a new one. */
  lit?: string;
  /** Whether the next message starts a new thread, in place of going to the thread lit. */
  fresh = false;
  /** The target of the input as it stood when the operator typed the first character of his message, or null for a new
   * thread. It holds until he sends or clears the message, so a thread that opens while he writes takes nothing that
   * he wrote for another. */
  pinned?: string | null;
  /** The thread that the input sends its next message to: the thread that the operator chose, else the open thread lit,
   * unless he asked for a new thread. Python, and a prompt of another shape than str, start no message of a thread. */
  get target(): string | undefined {
    if (this.mode !== "prompt" || this.editing || this.operatorPrompt) return undefined;
    if (this.pinned !== undefined) return this.pinned ?? undefined;
    return this.thread ?? (this.fresh || this.shape !== "str" ? undefined : this.lit);
  }
  /** The input sends its next message to the thread named, or starts a new thread when none is named. */
  aim(thread?: string): void {
    this.thread = thread;
    this.fresh = !thread;
    this.pinned = undefined;
  }
  readonly preferences: Preferences;
  mode: "prompt" | "python" = "prompt";
  shape = "str";
  drafts: Record<string, string> = {};
  /** Where each view was left, by its key. */
  scrolls: Record<string, Scroll> = {};
  /** The text that Ctrl+S put aside, by the key of its draft. */
  stashes: Record<string, string> = {};
  folds: Record<string, boolean> = {};
  roster: [string, string[], number][] = [];
  /** The models the catalog of the crate offers, which the operator may add to the roster of the session. */
  catalog: [string, string[], number][] = [];
  /** Whether the worker said what the catalog offers. */
  catalogRead = false;
  /** What the first run checks of this machine, once the worker says it, and null in the demo, which needs no
   * model. */
  setup?: SetupInfo | null;
  /** Whether the operator watches this session: the demo that plays a life by itself and asks no model. */
  watching = false;
  /** The moment of the life that the feed shows, as the count of facts that the life had said then, or nothing for
   * now. */
  moment?: number;
  /** The models the operator added to the roster of the session from the catalog, which each later life offers. */
  models: string[] = [];
  findings: string[] = [];
  rejectedWord = "";
  queued: FollowUp[] = [];
  dispatched: string[] = [];
  images: Record<string, ImageAttachment[]> = {};
  queueHeld = false;
  queueError = "";
  redo: { from: string; to: string }[] = [];
  private draining = false;
  private savedCost = 0;
  /** The folder that owns the history of the workspace of the session: the main checkout, for a worktree. */
  private readonly workspace: string;
  histories: Record<string, string[]> = {};
  started: Record<string, number> = {};
  /** When this session saw each act change its phase, once the acts of the record it opened on were read. The record
   * holds no time, so an act that was done before that has none. */
  changedAt: Record<string, number> = {};
  /** How many milliseconds each act took that a life of the session ran, which a later life does not know, since the
   * record holds no time. */
  took: Record<string, number> = {};
  directory = "";
  private refreshTask?: Promise<void>;
  /** The count of changes of the act table that `acts` holds. */
  private counted = 0;
  /** The mark of the last snapshot that the session took, which tells the worker which parts of the chain it holds. */
  private held = 0;
  private dirty = false;
  private closed = false;
  /** The scope, the writes and the count of changes that `changes` was read at. */
  private changesRead = "";
  /** Whether the dialog of a record open to read was shown, so it shows once for each life of the session. */
  readingTold = false;
  /** The actor the operator chose for each chain, which the session shows until the rung that sets it has run. */
  private readonly choices = new Map<string, { actor: string; rung?: string }>();
  constructor(
    readonly engine: Engine,
    readonly host: HostView,
    readonly demo = false,
    preferences?: Preferences,
  ) {
    super();
    this.selected = engine.root;
    this.actor = host.actor;
    this.sessionName = basename(host.directory);
    const path = host.record;
    this.preferences =
      // A recorded life keeps nothing beside its log, so its demo reads the preferences of the user.
      preferences ??
      new Preferences(
        demo && path && !host.recorded ? join(dirname(path), "ui-preferences.json") : undefined,
      );
    if (path) {
      const { view, damage } = savedView(path);
      Object.assign(this, Object.fromEntries(kept.flatMap((key) => (key in view ? [[key, view[key]]] : []))));
      this.savedCost = view.cost ?? 0;
      // A saved queue waits for the choice of the operator before it sends.
      this.queueHeld = this.queued.length > 0;
      if (damage) this.notice = damage;
    }
    this.workspace = workspaceRoot(host.directory);
    if (path && this.keeps) {
      try {
        seed(this.workspace, path, this.histories);
      } catch {
        // The history of the workspace is a convenience: a folder that cannot take it leaves the session its own.
      }
    }
    if (host.role === "read") this.notice = "This window reads the life only, so it changes nothing in it.";
    // A view that an older release saved, and that this release has no more, opens as the feed.
    if (!views.includes(this.view)) this.view = "feed";
    host.on("change", this.changed);
    host.on("facts", this.factsChanged);
    host.on("fault", this.fail);
    host.on("authentication", (status: import("furb").CodexAuthInfo) => {
      this.notice =
        status.state === "connected"
          ? "OpenAI is connected."
          : ["connecting", "waiting"].includes(status.state)
            ? "Connecting to OpenAI."
            : status.message || "Codex sign-in canceled.";
      this.emit("authentication", status);
      this.emit("change");
    });
    // A window that attached again to another host of its life reads its acts from the start, since the counts of the
    // acts are of the host that made them.
    host.on("reset", () => {
      this.counted = 0;
      void this.refresh();
    });
    // A report of the life that no model reads, such as a file that the scratch directory leaves out, shows as a
    // notice, since a write to stderr would draw over the screen.
    host.on("log", (line: string) => {
      this.notice = line;
    });
    void host.catalog().then((catalog) => {
      this.catalog = catalog;
      this.catalogRead = true;
      this.emit("change");
    }, this.fail);
    void host.setup().then((found) => {
      this.setup = found;
      this.emit("change");
    }, this.fail);
    this.save();
  }
  /** The catalog and the check of the setup read again, as after the operator signed in to a model. */
  async checkSetup(): Promise<void> {
    const [catalog, found] = await Promise.all([this.host.catalog(), this.host.setup()]);
    this.catalog = catalog;
    this.catalogRead = true;
    this.setup = found;
    this.emit("change");
  }
  private changed = () => {
    // A notice asks nothing, so it settles as the console takes it, and no key of the operator dismisses it.
    for (const id of this.host.notices.keys())
      if (!this.settled.has(id)) {
        this.settled.add(id);
        void this.host.dismiss(id).catch(this.fail);
      }
    for (const id of this.settled) if (!this.host.notices.has(id)) this.settled.delete(id);
    this.emit("change");
  };
  /** The notices that the session dismissed, until the console drops them. */
  private readonly settled = new Set<string>();
  /** The theme that the session shows: the one that the preferences name, or the one of the mode of the terminal. */
  get theme(): ThemeName {
    return themeOf(this.preferences.theme, this.preferences.terminalMode);
  }
  get notice(): string {
    return this.message;
  }
  set notice(value: string) {
    clearTimeout(this.noticeTimer);
    this.message = value;
    this.emit("change");
    if (value) {
      this.noticeTimer = setTimeout(() => {
        this.message = "";
        this.emit("change");
      }, 4000);
      this.noticeTimer.unref();
    }
  }
  set theme(value: ThemeChoice) {
    this.preferences.save(value);
  }
  /** Each batch of facts that the host hears reads the view again, whatever kind of act, of the engine or of an
   * extension, they are about. */
  private factsChanged = () => {
    void this.refresh().catch(this.fail);
  };
  fail = (error: unknown) => {
    this.error = error instanceof Error ? error.message : String(error);
    // A damaged journal is no failure of the view alone: the view offers to repair it or to start a new one.
    this.damaged[`${this.selected}:${this.view}`] =
      (error as { name?: unknown } | undefined)?.name === "DamagedJournal";
    this.emit("change");
  };
  get error(): string {
    return this.errors[`${this.selected}:${this.view}`] ?? "";
  }
  set error(value: string) {
    this.errors[`${this.selected}:${this.view}`] = value;
    if (!value) delete this.damaged[`${this.selected}:${this.view}`];
  }
  /** Whether the journal of file changes is known to be damaged, whatever the view that is shown. */
  get journalDamaged(): boolean {
    return Object.values(this.damaged).some(Boolean);
  }
  /** Whether the error of each view of a chain is a damaged journal of file changes. */
  private readonly damaged: Record<string, boolean> = {};
  /** Why the journal of file changes cannot be read, which the view shown offers to repair; empty when it reads. */
  get damagedJournal(): string {
    return this.damaged[`${this.selected}:${this.view}`] ? this.error : "";
  }

  /** The journal of file changes written again with each change that it holds, and the view read again. */
  async repairJournal(): Promise<void> {
    this.damagedOnly();
    const { kept, removed } = await this.host.repairChanges();
    await this.journalMended(
      `The journal is repaired: ${kept} ${kept === 1 ? "change" : "changes"} kept, ${removed} ${removed === 1 ? "line" : "lines"} removed.`,
    );
  }
  /** A new journal of file changes, with the damaged one kept beside it, and the view read again. */
  async renewJournal(): Promise<void> {
    this.damagedOnly();
    const aside = await this.host.renewChanges();
    await this.journalMended(`A new journal is started. The damaged one is kept at ${shortenHome(aside)}.`);
  }
  /** A repair or a new journal acts on a journal that a view found damaged. */
  private damagedOnly(): void {
    if (this.host.reading) throw new Error("This record is open to read.");
    if (!this.journalDamaged) throw new Error("The journal of file changes is not damaged.");
  }
  private async journalMended(notice: string): Promise<void> {
    // The journal is one for the whole session, so each view that found it damaged reads it again.
    for (const key of Object.keys(this.damaged)) {
      delete this.errors[key];
      delete this.damaged[key];
    }
    this.error = "";
    this.changesRead = "";
    this.notice = notice;
    await this.refresh();
  }

  refresh(): Promise<void> {
    // A session that closed, as one that opens again on its record does, has nothing more to read.
    if (this.closed) return Promise.resolve();
    this.dirty = true;
    if (this.refreshTask) return this.refreshTask;
    this.loading = true;
    this.emit("change");
    this.refreshTask = (async () => {
      do {
        this.dirty = false;
        const selected = this.selected;
        const { acts, count, mark, turns, answers, program, ...snapshot } = await this.host.snapshot(
          selected,
          this.counted,
          this.held,
        );
        if (this.selected !== selected) {
          this.dirty = true;
          continue;
        }
        this.held = mark;
        Object.assign(this, snapshot);
        // The tails of the turns and of the answers take the place of what they follow, and the words that the program
        // changed join it, so the parts that did not change keep their objects, which a view that reads them keys on.
        if (turns.list.length || turns.from < this.turns.length)
          this.turns = [...this.turns.slice(0, turns.from), ...turns.list];
        if (answers.list.length || answers.from < this.answers.length)
          this.answers = [...this.answers.slice(0, answers.from), ...answers.list];
        if (program?.whole) this.program = program.words;
        else if (program) Object.assign(this.program, program.words);
        // A snapshot carries the acts that changed since the session last read them, in the order of the table.
        if (acts.length) {
          // Each act that changed takes its place in the list of the acts, and a new act stands after the others, so
          // the list and its index change in the time of the acts that changed.
          const index = this.indexed();
          const next = this.acts;
          let moved = false;
          for (const act of acts) {
            const place = index.at.get(act.id);
            const phased = place === undefined || phaseOf(next[place]) !== phaseOf(act);
            moved ||= phased;
            if (place !== undefined) next[place] = act;
            else index.at.set(act.id, next.push(act) - 1);
            index.ids.set(act.id, act);
            placed(index, act);
            if (!act.done) this.started[act.id] ??= clock();
            // Only the prompts tell when a thread changed, so the table stays as small as the threads: a step stamps the
            // prompt that it works for, and a message stamps the head of its thread.
            if (phased && this.counted !== 0 && (act.kind === "prompt" || act.kind === "rung")) {
              const now = clock();
              const maker = act.kind === "rung" ? act.by : act.id;
              const head = index.ids.get(maker)?.thread;
              for (const id of [act.id, maker, head]) if (id) this.changedAt[id] = now;
            }
            if (act.took !== undefined) this.took[act.id] = act.took;
          }
          if (moved) this.version++;
        }
        this.counted = count;
        // The acts and the actor of one snapshot are of one moment: the choice holds until its rung is done there.
        const choice = this.choices.get(selected);
        if (choice?.rung && this.actOf(choice.rung)?.done) this.choices.delete(selected);
        else if (choice) this.actor = choice.actor;
        const dispatched = new Set(this.dispatched);
        const queued = this.queued.filter((entry) => !dispatched.has(entry.id));
        if (queued.length !== this.queued.length) {
          this.queued = queued;
          if (!queued.length) {
            this.queueHeld = false;
            this.queueError = "";
          }
          this.save();
        }
        if (this.view === "changes") {
          const writes = this.scope === "branch" ? null : this.writes();
          const read = `${this.scope}:${this.host.changes}:${writes ? `${writes[0]}:${writes.length}:${writes.at(-1)}` : ""}`;
          if (read !== this.changesRead) {
            try {
              const changes = await (this.scope === "branch"
                ? this.host.branchChanges()
                : this.host.netChanges(this.scope === "life" ? null : writes));
              this.changes = changes.map(reviewed);
              this.changesRead = read;
            } catch (error) {
              // A damaged journal is the failure of the view of changes alone, and it never stops the session: the
              // view says it and offers to repair it, and the next read tries again.
              if ((error as { name?: unknown } | undefined)?.name !== "DamagedJournal") throw error;
              this.fail(error);
            }
          }
        }
        // The findings of the gate that refused the last word of the operator on the chain shown.
        const lastWord = this.indexed().words.get(this.selected);
        this.findings = lastWord?.run?.refused ? lastWord.run.reason.split("\n") : [];
        this.rejectedWord = this.findings.length ? String(lastWord?.words[0] ?? "") : "";
        this.emit("change");
      } while (this.dirty && !this.closed);
    })().finally(() => {
      this.refreshTask = undefined;
      this.loading = false;
      if (!this.closed) this.emit("change");
      if (!this.closed) void this.drainQueue().catch(this.fail);
    });
    return this.refreshTask;
  }

  /** The read of the files of the project the session works in, and the paths or the error it came to. */
  files?: { directory: string; read: Promise<string[]>; paths?: string[]; error?: string };
  /** The files of the project the session works in: read again when asked fresh or when the directory moved, and
   * otherwise the read already made, so every reader of one read sees the same list. The end of a read is a change of
   * the session. */
  projectFiles(fresh = false): Promise<string[]> {
    const directory = this.workingDirectory;
    if (fresh || this.files?.directory !== directory) {
      const files: NonNullable<Session["files"]> = { directory, read: projectFiles(directory) };
      this.files = files;
      files.read.then(
        (paths) => {
          files.paths = paths;
          this.emit("change");
        },
        (error: unknown) => {
          files.error = error instanceof Error ? error.message : String(error);
          this.emit("change");
        },
      );
    }
    return this.files.read;
  }
  /** The directory that the paths of the selected chain resolve against, as the files resolve them. */
  get workingDirectory(): string {
    return resolve(this.host.directory, this.directory);
  }
  /** A path that the operator typed, with a leading `~` read as the home directory, against the directory of the
   * selected chain. */
  path(typed: string): string {
    return resolve(this.workingDirectory, expandHome(typed));
  }
  /** The key of the draft that the composer shows: the selected chain, and the program it edits or its mode of
   * input. */
  get draftKey(): string {
    return `${this.selected}:${this.editing ?? this.mode}`;
  }
  /** Whether the session saves what it sends to the history of its workspace: a recorded life or a record open to
   * read keeps nothing, and a window that reads only changes nothing. */
  private get keeps(): boolean {
    const { host } = this;
    return Boolean(host.record) && !host.recorded && !host.reading && host.role !== "read";
  }
  /** Keep a line that the input sent under the key of a draft: in the history of its chain, which the record of the
   * session keeps, and in the history of the workspace, which the other sessions of it read. */
  remember(key: string, text: string): void {
    const history = this.histories[key] ?? [];
    this.histories[key] = history;
    if (text && history.at(-1) !== text) history.push(text);
    if (history.length > 200) history.shift();
    if (!text || !this.keeps) return;
    try {
      append(this.workspace, { text, mode: modeOf(key), session: this.host.record as string });
    } catch (error) {
      this.notice = `The history of the workspace was not saved: ${error instanceof Error ? error.message : error}`;
    }
  }
  /** What the other sessions of the workspace sent in a mode, the oldest first. The sessions of its worktrees count as
   * its own. */
  workspaceHistory(mode: string): string[] {
    const record = this.host.record;
    if (!record) return [];
    return sentIn(this.workspace)
      .filter((one) => one.mode === mode && one.session !== record)
      .map((one) => one.text);
  }
  /** What the session is doing, over the acts of the selected chain or of every chain: the view failed, the queue or
   * a question waits for the operator, a reopened record holds work, or what the acts do. A pause of the chain holds
   * it while none of its acts runs or asks. */
  status(chain?: string): SessionStatus {
    if (this.host.reading) return "reading";
    // A damaged journal of file changes fails the view that shows it, and not the session that the list marks.
    if (chain ? this.error : Object.entries(this.errors).some(([key, error]) => error && !this.damaged[key]))
      return "error";
    if (this.queueHeld && this.queued.length) return "blocked";
    if (this.host.pending.size) return "paused";
    if (!chain && this.host.prompts.size) return "blocked";
    const state = this.actStatus(chain);
    return this.paused && (state === "idle" || state === "error") ? "paused" : state;
  }
  /** What the acts of a chain do, or the acts of every chain: one asks the operator, one waits for the resume of a
   * reopened record, one runs, a pause holds one, or the last act failed. */
  private actStatus(chain?: string): SessionStatus {
    // The state holds while no act starts, ends, pauses or wakes, and the prompts and the held work stay as they are.
    if (this.states.version !== this.version) this.states = { version: this.version, of: new Map() };
    const key = `${chain ?? ""}\n${[...this.host.prompts.keys()].join(" ")}\n${[...this.host.pending.keys()].join(" ")}`;
    const known = this.states.of.get(key);
    if (known) return known;
    const status = this.statusOf(chain);
    this.states.of.set(key, status);
    return status;
  }
  /** The acts that are not done, by the chain that they stand on, as they stood at a version of the acts. */
  private liveRead = { version: -1, on: new Map<string, ActRow[]>() };
  private liveOn(): Map<string, ActRow[]> {
    if (this.liveRead.version === this.version) return this.liveRead.on;
    const on = new Map<string, ActRow[]>();
    for (const act of this.indexed().live.values()) {
      const list = on.get(act.on);
      if (list) list.push(act);
      else on.set(act.on, [act]);
    }
    this.liveRead = { version: this.version, on };
    return on;
  }
  /** The state of the acts of each chain, or of every chain, as actStatus read it at a version of the acts. */
  private states = { version: -1, of: new Map<string, SessionStatus>() };
  private statusOf(chain?: string): SessionStatus {
    const index = this.indexed();
    const mine = (act?: ActRow): act is ActRow =>
      act !== undefined && act.kind !== "chain" && act.kind !== "grant" && (!chain || act.on === chain);
    const asked = (ids: Iterable<string>) => [...ids].some((id) => mine(index.ids.get(id)));
    if (asked(this.host.prompts.keys())) return "blocked";
    if (asked(this.host.pending.keys())) return "paused";
    // Only an act that is not done works or waits under a pause, and the index holds each such act.
    const live = (chain ? (this.liveOn().get(chain) ?? []) : [...index.live.values()]).filter(
      (act) => mine(act) && !act.unfinished,
    );
    if (live.some(working)) return "working";
    if (live.some((act) => act.paused && !act.done)) return "paused";
    const latest = chain ? index.latest.get(chain) : this.acts.findLast(mine);
    return latest && failed(latest) ? "error" : "idle";
  }
  /** The state of a chain. A chain that the operator started, directly or by a word of their own, and that finished
   * its work while another chain was shown, is finished and not yet seen until the operator opens it. */
  chainStatus(id: string): SessionStatus {
    const status = this.actStatus(id);
    const before = this.phases.get(id);
    this.phases.set(id, status);
    const chain = this.indexed().ids.get(id);
    const mine = chain?.by === "operator" || this.actOf(chain?.by ?? "")?.by === "operator";
    if (id === this.selected || status !== "idle") this.unread.delete(id);
    else if (before === "working" && mine) this.unread.add(id);
    return status === "idle" && this.unread.has(id) ? "done" : status;
  }
  /** The chains that a view of the chains shows apart, each with its state: the root, the chain shown, and each chain
   * whose state is not idle, in the order of the chains, and how many other chains rest. It reads the state of each
   * chain that can be other than idle: one with work that is not done or held, a question of the operator, a last act
   * that failed, or a state that was not idle when it was read last, and no other chain. */
  activeChains(): { active: [ActRow, SessionStatus][]; resting: number } {
    const index = this.indexed();
    // A chain of a run of compactions into new chains stands as its run, which the last chain of it heads.
    const head = (id: string) => this.runOf(id).at(-1) ?? id;
    const root = head(this.engine.root);
    const selected = head(this.selected);
    const restless = new Set(
      [
        this.engine.root,
        this.selected,
        ...this.liveOn().keys(),
        ...index.failing,
        ...this.unread,
        ...[...this.phases].flatMap(([id, phase]) => (phase === "idle" ? [] : [id])),
        ...[...this.host.prompts.keys(), ...this.host.pending.keys()].flatMap(
          (id) => index.ids.get(id)?.on ?? [],
        ),
      ].map(head),
    );
    const active = [...restless]
      .flatMap((id) => {
        const chain = index.ids.get(id);
        return chain?.kind === "chain" ? [[chain, this.chainStatus(id)] as [ActRow, SessionStatus]] : [];
      })
      .filter(([chain, status]) => chain.id === root || chain.id === selected || status !== "idle")
      .sort(
        ([one], [other]) => (index.places.get(one.id)?.chain ?? 0) - (index.places.get(other.id)?.chain ?? 0),
      );
    const listed = index.chains.filter((chain) => this.listed(chain.id)).length;
    return { active, resting: listed - active.length };
  }
  /** The first chains that rest, in the order of the chains, up to a count. */
  restingChains(count: number): ActRow[] {
    const shown = new Set(this.activeChains().active.map(([chain]) => chain.id));
    const out: ActRow[] = [];
    for (const chain of this.indexed().chains) {
      if (out.length >= count) break;
      if (!shown.has(chain.id) && this.listed(chain.id)) out.push(chain);
    }
    return out;
  }
  /** A message or a word that the operator sends to a paused chain wakes the chain first, since the operator who
   * writes to it wants it to go on: work that a reopened record held starts again, and the pause over the chain ends.
   * It says whether the chain was paused. */
  async wakeForInput(): Promise<boolean> {
    if (!this.paused) return false;
    if (this.host.pending.size) await this.host.resume();
    await this.refresh();
    if (this.paused) await this.engine.wake(this.selected);
    await this.refresh();
    return true;
  }
  /** The chains that the operator started and that finished their work while another chain was shown, which wait for
   * the operator to look at them, and the state that each chain had when the view last read it. */
  private readonly unread = new Set<string>();
  private readonly phases = new Map<string, SessionStatus>();
  get chains(): ActRow[] {
    return this.indexed().chains;
  }
  /** The index of the acts. A snapshot that changes the acts places each act that changed, and a list of acts that
   * the session did not index, as one that a test sets, is indexed whole. */
  private index?: Index;
  private indexed(): Index {
    if (this.index?.of === this.acts) return this.index;
    const index: Index = {
      of: this.acts,
      ids: new Map(),
      at: new Map(),
      on: new Map(),
      made: new Map(),
      chains: [],
      places: new Map(),
      words: new Map(),
      live: new Map(),
      stamps: new Map(),
      chained: new Map(),
      threaded: new Map(),
      prompts: this.index?.prompts ?? 0,
      touched: [],
      written: [],
      work: new Map(),
      latest: new Map(),
      failing: new Set(),
    };
    for (const [place, act] of this.acts.entries()) {
      index.ids.set(act.id, act);
      index.at.set(act.id, place);
      placed(index, act);
    }
    this.index = index;
    return index;
  }
  /** The index of the acts that the ladder of the chain shown reads, so that a frame makes no pass over every act. */
  get ladderIndex(): ActIndex & { live: Iterable<ActRow>; stamp: (id: string) => number } {
    const index = this.indexed();
    return {
      on: this.activity,
      made: index.made,
      ids: index.ids,
      live: index.live.values(),
      stamp: (id) => index.stamps.get(id) ?? 0,
    };
  }
  /** How many times a prompt came or changed, which the threads of the life read again on, and no other act. */
  get promptsVersion(): number {
    return this.indexed().prompts;
  }
  /** The value of promptsVersion when a prompt on the chain last came or changed. */
  threadStamp(chain: string): number {
    return this.indexed().threaded.get(chain) ?? 0;
  }
  /** The writes that are done, in the order that the session saw them done. */
  get written(): readonly ActRow[] {
    return this.indexed().written;
  }
  /** The acts of the chain shown, but its chains, as the index holds them: a list that grows as acts come, in which an
   * act that changed takes its place. */
  get activity(): ActRow[] {
    return this.indexed().work.get(this.selected) ?? [];
  }
  /** The acts on every chain but its chains, as the index holds them, by the chain that they stand on. */
  get activityByChain(): ReadonlyMap<string, ActRow[]> {
    return this.indexed().work;
  }
  /** The acts of the chain shown that are not done, but its chains, in the order of the acts: what a frame reads of
   * the work that runs, with no pass over every act of the chain. */
  get open(): ActRow[] {
    return (this.liveOn().get(this.selected) ?? []).filter((act) => !act.done && act.kind !== "chain");
  }
  /** What the answers of the chain itself cost, each token counted once: a chain with a source does not count the
   * answers of its origin again. The first number of the usage of an answer is the whole prompt of that call, which
   * holds the reads and the writes of the cache, so the fresh input of an answer is what is left of its prompt after
   * them. */
  get spend(): { input: number; output: number; cacheRead: number; cacheWrite: number; dollars: number } {
    const sum = { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, dollars: 0 };
    for (const usage of this.answers) {
      const [prompt, output, read, write, dollars] = usage;
      sum.input += Math.max(0, prompt - read - write);
      sum.output += output;
      sum.cacheRead += read;
      sum.cacheWrite += write;
      sum.dollars += dollars;
    }
    return sum;
  }
  /** The whole prompt of the last answer of the chain, in tokens: what the model read on its last call, its system
   * prompt, the reads of the cache, and the fresh input together. Nothing before the first answer. */
  get context(): number | undefined {
    return this.turns.findLast(([role, , used]) => role === "assistant" && used)?.[2]?.[0];
  }
  /** The name of the chain shown, which a run of compactions into new chains names as a whole. */
  get label(): string {
    return this.labelOf(this.runOf(this.selected).at(-1) ?? this.selected);
  }
  get actorChoice(): { model: string; effort: string } {
    return actorParts(
      this.actor,
      this.roster.map(([name]) => name),
    );
  }
  /** The ask that the operator points at on the desk, which the input answers while its chain is shown. */
  pointed?: string;
  /** What waits for the operator in the life: each question of every chain, the oldest first. A notice asks nothing,
   * so it settles at once and waits on no desk. */
  get desk(): Desk {
    // An ask that the operator answered leaves the desk at once, though the console holds it until it hears the close.
    for (const id of this.gone) if (!this.host.prompts.has(id)) this.gone.delete(id);
    const held = (one: { id: string }) => !this.gone.has(one.id);
    const { acts, host } = this;
    const gone = [...this.gone].join(" ");
    const cached = this.deskRead;
    if (cached?.version === this.version && cached.prompts === host.prompts && cached.gone === gone)
      return cached.desk;
    const made = desk({
      acts,
      ids: this.indexed().ids,
      prompts: [...host.prompts.values()].filter(held),
      notices: [],
      label: (id) => this.labelOf(id),
    });
    this.deskRead = { version: this.version, prompts: host.prompts, gone, desk: made };
    return made;
  }
  /** The desk as it was last read, with what it was read from, since every draw reads it many times. */
  private deskRead?: { version: number; prompts: unknown; gone: string; desk: Desk };
  /** The questions that the operator answered, until the console drops them. */
  private readonly gone = new Set<string>();
  /** The question that the operator acts on, on the chain shown: the one that he points at, else the oldest. */
  get aimed(): Ask | undefined {
    return aimed(this.desk.asks, this.selected, this.pointed);
  }
  /** The question that the input answers: the question aimed. */
  get operatorPrompt(): Ask | undefined {
    return this.aimed;
  }
  /** Point at an ask of the desk, and show its chain. */
  async point(id: string): Promise<void> {
    const ask = this.desk.asks.find((one) => one.id === id);
    if (!ask) return;
    this.pointed = id;
    if (ask.chain !== this.selected) await this.select(ask.chain);
    else this.emit("change");
  }
  /** Point at the ask a step after or before the one aimed, round from the last to the first, or at the first ask of
   * the life when the chain shown has none. */
  async step(by: number): Promise<Ask | undefined> {
    const next = beside(this.desk.asks, this.aimed?.id, by);
    if (next) await this.point(next.id);
    return next;
  }
  /** A line of the operator as the answer of a question, which the console reads in the shape of the question. The
   * notice says where the answer went and what still waits. */
  async answer(ask: Ask, line: string): Promise<void> {
    await this.host.answer(ask.id, line);
    this.landed(ask, `Your answer went to ${ask.id} on ${ask.label}.`);
  }
  /** The operator takes none of the values that a question offers and answers in his own words, which close it as
   * Refused: the await of the word that asked raises them, and its model reads them. */
  async decline(ask: Ask, words: string): Promise<void> {
    await this.host.decline(ask.id, words);
    this.landed(ask, `Your words went to ${ask.id} on ${ask.label}.`);
  }
  /** The operator resolves a thread: its open prompt closes as the engine closes a prompt, with None where its shape
   * takes None, with the word resolved where it takes a str, and else as Refused of the words resolved by the operator,
   * which the await of the word that asked raises. The rungs that work for the thread end with it. */
  async resolve(id: string): Promise<void> {
    const act = this.indexed().ids.get(id);
    if (!act || act.kind !== "prompt" || act.done) return;
    const shape = String(act.words[0] ?? "");
    const value = /\bNone\b|^object$/.test(shape)
      ? null
      : shape === "str"
        ? "resolved"
        : { is: "Refused", args: ["resolved by the operator"] };
    await this.engine.close(value, { id });
    this.notice = `You resolved ${act.thread ?? id}. A message to it opens it again.`;
    await this.refresh();
  }
  /** What the operator did to an ask, and what waits after it, in the notice. The pointer goes on to what waits next
   * on the chain shown. */
  private landed(ask: Ask, said: string): void {
    if (this.pointed === ask.id) this.pointed = undefined;
    this.gone.add(ask.id);
    const rest = this.desk.asks.length;
    this.notice = `${said} ${rest ? `${rest} more ${rest === 1 ? "waits" : "wait"}.` : "Nothing else waits."}`;
  }
  isUserPrompt(act: ActRow): boolean {
    return act.kind === "prompt" && act.by === "operator";
  }
  /** Whether a prompt has a program that an edit changes and replays: one of its rungs holds a word the gate let run
   * on this chain. */
  editable(act: ActRow): boolean {
    return (
      act.kind === "prompt" &&
      this.activity.some((rung) => rung.by === act.id && Object.hasOwn(this.program, rung.id))
    );
  }
  /** The acts on a chain, its own act among them, in the order of the acts. */
  actsOn(chain: string): readonly ActRow[] {
    return this.indexed().on.get(chain) ?? [];
  }
  /** The first act after an act that the maker of that act made and that a test takes. */
  madeAfter(act: ActRow, test: (other: ActRow) => boolean): ActRow | undefined {
    const index = this.indexed();
    const list = index.made.get(act.by) ?? [];
    for (let at = (index.places.get(act.id)?.made ?? list.length) + 1; at < list.length; at++) {
      const other = list[at];
      if (other && test(other)) return other;
    }
    return undefined;
  }
  /** The acts by their name, by the chain that they stand on, and the chains, with the acts that are not done, which
   * the board of the life reads in place of a pass over every act. */
  get boardIndex(): {
    ids: ReadonlyMap<string, ActRow>;
    on: ReadonlyMap<string, readonly ActRow[]>;
    chains: readonly ActRow[];
    live: Iterable<ActRow>;
    stamp: (chain: string) => number;
    touched: readonly string[];
  } {
    const index = this.indexed();
    return {
      ids: index.ids,
      on: index.on,
      chains: index.chains,
      live: index.live.values(),
      stamp: (chain) => index.chained.get(chain) ?? 0,
      touched: index.touched,
    };
  }
  /** The act that a name or a door names: the act whose name is the first part of the path. */
  actOf(path: string): ActRow | undefined {
    const [name] = path.split("/");
    return this.indexed().ids.get(name ?? "");
  }
  async attachImage(path: string): Promise<void> {
    if (!(await this.host.sees(this.actor)))
      throw new Error("Choose a model that accepts images before attaching one.");
    const image = await this.host.attachImage(this.path(path));
    const images = this.images[this.selected] ?? [];
    this.images[this.selected] = images;
    if (!images.some((current) => current.uri === image.uri)) images.push(image);
    this.save();
    this.notice = `${image.name} attached.`;
  }
  private withImages(text: string): string {
    return [
      text,
      ...(this.images[this.selected] ?? []).map(({ name, uri }) => imageReference(name, uri)),
    ].join("\n");
  }
  /** The name of a chain: its label, or Main for the root. The last chain of a run of compactions into new chains
   * names the run, as the first chain and the last one: Main -> compacted. */
  labelOf(id: string): string {
    const own = (name: string) => {
      const chain = this.indexed().ids.get(name);
      return name === this.engine.root
        ? "Main"
        : String((chain?.kind === "chain" && chain.words[0]) || "Chain");
    };
    const run = this.runOf(id);
    return run.length > 1 && run.at(-1) === id ? `${own(run[0] ?? id)} -> ${own(id)}` : own(id);
  }
  /** The run of compactions into new chains that holds a chain, as `furb/life` gives it. */
  runOf(id: string): string[] {
    return compactedRun(id, this.host.compactions);
  }
  /** Whether a list of the chains shows a chain: a chain that a later chain of its run goes on from shows as that run,
   * under the last chain of it. */
  listed(id: string): boolean {
    return this.runOf(id).at(-1) === id;
  }
  async select(chosen: string): Promise<void> {
    // A chain of a run of compactions into new chains is shown as its run, whose last chain the operator speaks to: a
    // message, a command and a grant go on that chain, whichever chain of the run a link or the graph names.
    const id = this.runOf(chosen).at(-1) ?? chosen;
    // The moment that the feed shows is one of the chain shown, so another chain is shown as it stands now.
    if (id !== this.selected) this.moment = undefined;
    // The turns and the program of the chain left are not the chain selected, which shows them once it has read them.
    if (id !== this.selected) {
      this.editing = undefined;
      this.held = 0;
      this.thread = undefined;
      this.fresh = false;
      this.turns = [];
      this.answers = [];
      this.filled = undefined;
      this.program = {};
    }
    this.selected = id;
    this.search = "";
    await this.refresh();
  }
  /** The writes of the scope of the review on the chain shown: those since the last message of the operator for this
   * turn, and those of the chain for this chain. */
  private writes(): string[] {
    const writes = this.activity.filter((act) => act.kind === "write");
    const message =
      this.scope === "turn" ? this.activity.findLast((act) => this.isUserPrompt(act)) : undefined;
    return writes.filter((act) => !message || act.began > message.began).map((act) => act.id);
  }
  /** The scope of the review set, or moved one step through the scopes, and the changes read again. The branch is read
   * again each time that it is chosen, since git may change with no act of the life. */
  review(scope: Scope | -1 | 1): void {
    this.scope =
      typeof scope === "string"
        ? scope
        : (scopes[(scopes.indexOf(this.scope) + scope + scopes.length) % scopes.length] ?? "chain");
    this.changesRead = "";
    this.save();
    this.show("changes");
  }
  /** One hunk of a change reverted by a word of the operator on the chain shown, so the World writes the file and the
   * record keeps the write. The word refuses when the file no longer holds the hunk where the review showed it. */
  async revert(change: ShownChange, hunk: Hunk): Promise<void> {
    if (this.host.reading) {
      this.notice = "This record is open to read.";
      return;
    }
    const path = fromDirectory(this.host.directory, change.path);
    await this.engine.result(
      await this.engine.rung({ word: revertWord(change.path, hunk), on: this.selected }),
    );
    this.notice = `The hunk at line ${hunk.newStart} of ${path} is reverted.`;
    this.changesRead = "";
    await this.refresh();
  }
  show(view: View): void {
    this.view = view;
    this.search = "";
    this.emit("change");
    void this.refresh().catch(this.fail);
  }

  private track(id: string): void {
    void this.engine
      .result(id)
      .then(
        () => {},
        async (error: unknown) => {
          // A cancel that the operator chose said what it ended already.
          if (this.closed || this.cancels.delete(id)) return;
          this.notice = "";
          // A finished act carries its failure in the record and in the feed.
          if (!(await this.engine.outcome(id)).done) throw error;
          if (error instanceof Error && error.message.startsWith("CancelledError"))
            this.notice = "Work cancelled.";
        },
      )
      .then(() => {
        if (!this.closed) return this.refresh();
      })
      .catch(this.fail);
  }
  enqueue(text: string): void {
    if (!text.trim()) return;
    this.queued.push({
      id: randomUUID(),
      chain: this.selected,
      text: this.withImages(text),
      shape: this.shape,
    });
    delete this.images[this.selected];
    this.save();
    this.notice = "Follow-up queued.";
    void this.drainQueue().catch(this.fail);
  }
  async drainQueue(): Promise<void> {
    if (this.closed || this.draining || this.queueHeld || this.host.pending.size || !this.queued.length)
      return;
    this.draining = true;
    // The queue is read again before each send, since the operator may remove or take back a follow-up meanwhile.
    const queued = (entry: FollowUp) => this.queued.some((item) => item.id === entry.id);
    const passed = new Set<string>();
    const next = () =>
      this.closed || this.queueHeld
        ? undefined
        : this.queued.find(
            (entry) =>
              !passed.has(entry.id) &&
              !this.actOf(entry.chain)?.paused &&
              !(this.liveOn().get(entry.chain) ?? []).some(
                (act) => !act.done && ["prompt", "rung"].includes(act.kind),
              ),
          );
    try {
      for (let entry = next(); entry; entry = next()) {
        passed.add(entry.id);
        await this.attachFiles(entry.text, entry.chain);
        if (!queued(entry)) continue;
        this.track(await this.host.sendQueued(entry));
        this.queued = this.queued.filter((item) => item.id !== entry.id);
        this.save();
        await this.refresh();
      }
    } catch (error) {
      if (!this.closed) {
        this.queueHeld = true;
        this.queueError = error instanceof Error ? error.message : String(error);
        this.notice = `Queue stopped: ${this.queueError}`;
        this.save();
      }
    } finally {
      this.draining = false;
      this.emit("change");
    }
  }
  removeQueued(id: string): void {
    if (!this.queued.some((entry) => entry.id === id))
      throw new Error("This follow-up has already been sent.");
    this.queued = this.queued.filter((entry) => entry.id !== id);
    if (!this.queued.length) {
      this.queueHeld = false;
      this.queueError = "";
    }
    this.save();
    this.notice = "Follow-up removed.";
  }
  private async attachFiles(text: string, chain = this.selected): Promise<void> {
    const directory = resolve(this.host.directory, expandHome(await this.engine.cwd({ on: chain })));
    for (const path of await fileReferences(text, directory))
      await this.engine.result(await this.engine.rung({ word: `read(${JSON.stringify(path)})`, on: chain }));
  }
  /** The name of a new branch of this chain: its name and the first number that no chain takes, as Main 2. */
  private branchLabel(): string {
    const base = this.label.replace(/ \d+$/, "");
    const taken = new Set(this.chains.map((chain) => this.labelOf(chain.id)));
    let count = 2;
    while (taken.has(`${base} ${count}`)) count++;
    return `${base} ${count}`;
  }
  /** A new chain that retells the selected chain without some of its acts, and that the session selects. A rung of
   * the operator makes it, so that a later life makes it again. */
  async branch(label: string, omitted: string[]): Promise<string> {
    const source = this.selected;
    const filter = `take(${[...omitted.map((id) => JSON.stringify(id)), "inside=False"].join(", ")})`;
    const rung = await this.engine.rung({
      word: `chain(${JSON.stringify(label)}, ${JSON.stringify(source)}, ${filter})`,
      on: source,
    });
    await this.engine.result(rung);
    await this.refresh();
    const chain = this.chains.find((chain) => chain.by === rung);
    if (!chain) throw new Error("The rung made no chain.");
    await this.select(chain.id);
    return chain.id;
  }
  /** A new branch of the chain of an act, which the session selects. The branch reads the transcript of that chain
   * through the act, or up to a message of the operator, whose text returns to the composer to be sent again. The
   * module and the files keep the state they have. */
  async rewind(id: string, label?: string, files?: Restore): Promise<string> {
    if (this.paused) throw new Error("Wake this chain before rewinding.");
    const chain = this.acts.find((act) => act.id === id)?.on;
    if (!chain) throw new Error(`There is no act ${id} to rewind to.`);
    if (chain !== this.selected) await this.select(chain);
    const at = this.activity.findIndex((act) => act.id === id);
    const act = this.activity[at];
    if (!act || ["chain", "grant"].includes(act.kind)) throw new Error(`There is no act ${id} to rewind to.`);
    const message = this.isUserPrompt(act);
    // Every act after the point leaves the branch, a grant or a chain among them, so that the branch reads no ceiling
    // and no branch that came later.
    const omitted = this.activity.slice(message ? at : at + 1).map((later) => later.id);
    const branch = await this.branch(label ?? this.branchLabel(), omitted);
    if (files) await this.restoreFiles(files, branch);
    if (message) await this.restore(String(act.words[1] ?? ""));
    const kept = files
      ? `${this.restored(files)} The module keeps its state.`
      : "The module and the files keep their state.";
    // The watch says it in common words, as the first minute of furb.
    this.notice = this.watching
      ? branchNotice
      : message
        ? `The message is back in the input on a new branch. ${kept}`
        : `The new branch reads the transcript through ${id}. ${kept}`;
    return branch;
  }
  /** What a rewind to an act gives back of the files: the files as they stood when the act was made, or before the
   * message for a message of the operator. A write after the point counts, on any chain, but a write that the act
   * itself made, which the branch keeps. A change that an older session kept names no write, and stays. */
  async filesAt(id: string): Promise<Restore> {
    const byId = new Map(this.acts.map((act) => [act.id, act]));
    const point = byId.get(id);
    if (!point) return { files: [], commands: 0 };
    const message = this.isUserPrompt(point);
    const under = (act: ActRow): boolean => {
      for (let up: ActRow | undefined = act; up; up = byId.get(up.by)) if (up.id === id) return true;
      return false;
    };
    const after = (act: ActRow | undefined): act is ActRow =>
      !!act && act.began >= point.began && (message || !under(act));
    const files = new Map<string, FileChange>();
    // The count of changes in the state of the host lags the acts, which the session asks for, so the journal is read
    // to its end: a page shorter than asked is its last.
    for (let at = 0, page: FileChange[] = []; at === 0 || page.length === 100; at += 100) {
      page = await this.host.readChanges(at, 100);
      for (const change of page)
        if (change.act && after(byId.get(change.act)) && !files.has(change.path))
          files.set(change.path, change);
    }
    const commands = this.acts.filter((act) => act.kind === "bash" && after(act)).length;
    return { files: [...files.values()], commands };
  }
  /** The files given back as a rewind found them, by a rung of the operator on a chain, so that the record holds each
   * write and the chain reads it. A write puts back the text of a file, and a command removes a file that a write
   * made. */
  async restoreFiles({ files }: Restore, on = this.selected): Promise<void> {
    if (!files.length) return;
    const word = files
      .map((change) =>
        change.made
          ? `await bash(${JSON.stringify(`rm -f -- ${quoted(change.path)}`)})`
          : `write(Text(${JSON.stringify(this.near(change.path))}, ${JSON.stringify(change.before)}))`,
      )
      .join("\n");
    const rung = await this.engine.rung({ word, on });
    await this.engine.result(rung);
    await this.refresh();
    const run = this.acts.find((act) => act.id === rung)?.run;
    if (run?.status === "failed") throw new Error(`The files were not restored: ${run.reason}`);
  }
  /** A path as a word names it: from the directory of the chain shown when it stands under it, and whole else. */
  private near(path: string): string {
    const near = relative(this.workingDirectory, path);
    return near && !near.startsWith("..") && !isAbsolute(near) ? near : path;
  }
  /** What a restore of the files did, in words, with the changes that it could not undo. */
  restored({ files, commands }: Restore): string {
    const count = !files.length
      ? "No file changed after that point."
      : files.length === 1
        ? "1 file is back as it was."
        : `${files.length} files are back as they were.`;
    const stays = commands
      ? ` ${commands} ${commands === 1 ? "command" : "commands"} ran after that point, and a change that a command made is not restored.`
      : "";
    return `${count}${stays}`;
  }
  /** The files of the selected chain given back as they stood at an act, with no branch. */
  async restoreAt(id: string, files?: Restore): Promise<void> {
    const found = files ?? (await this.filesAt(id));
    await this.restoreFiles(found);
    this.notice = `${this.restored(found)} The conversation goes on as it stands.`;
  }
  /** The acts as they stood at the moment that the feed shows: an act that began later is not there yet, and an act
   * that ended later still runs, with no value yet. */
  at(acts: ActRow[]): ActRow[] {
    const moment = this.moment;
    if (moment === undefined) return acts;
    return asOf(acts, moment);
  }
  /** Move the moment that the feed shows one step back or on in the selected chain: a step is a moment when an act of
   * the chain began or ended. A step on to the last moment returns to now. */
  travel(by: -1 | 1): void {
    const moments = [
      ...new Set(
        this.acts
          .filter((act) => act.on === this.selected)
          .flatMap((act) => [act.began + 1, ...(act.ended === undefined ? [] : [act.ended + 1])]),
      ),
    ].sort((one, other) => one - other);
    const last = moments.at(-1);
    const now = this.moment ?? last;
    if (now === undefined) return;
    const next =
      by < 0 ? (moments.findLast((one) => one < now) ?? moments[0]) : moments.find((one) => one > now);
    this.moment = next === undefined || next === last ? undefined : next;
    this.emit("change");
  }
  /** A new branch of the selected chain at the moment that the feed shows, which the session selects: it reads the
   * transcript through the last act that began before that moment, as a rewind to that act does, and the files that a
   * write changed after it come back as they were. */
  async branchHere(): Promise<string> {
    const last = this.at(this.activity).findLast((act) => !["chain", "grant"].includes(act.kind));
    if (!last) throw new Error("Nothing happened before this moment, so there is nothing to branch from.");
    this.moment = undefined;
    return this.rewind(last.id, undefined, await this.filesAt(last.id));
  }
  /** A message sent before as the draft of the composer, with the images it referred to attached again. */
  private async restore(text: string): Promise<void> {
    let message = text;
    for (const reference of imageReferences(message)) {
      const image = await this.host.attachImage(imagePath(this.host.imageDirectory, reference.uri).path);
      image.name = reference.name || image.name;
      this.images[this.selected] ??= [];
      this.images[this.selected]?.push(image);
      message = message.replace(reference.text, "");
    }
    this.emit("compose", message.trimEnd());
  }
  /** The last act that the operator made on the chain shown and that is not over: the message, the word or the
   * command of his that the feed shows last. A cancel that names no act ends it. */
  get lastOwn(): ActRow | undefined {
    return this.activity.findLast((act) => act.by === "operator" && !act.done && WORK.includes(act.kind));
  }
  /** The running work that a cancel of an act ends besides the act: each act under it that is not over and whose work
   * takes time, on any chain, with the chains that they stand on. The rungs of a prompt are its own answer, and no
   * running work of their own. An act stands under the act that made it, a prompt on a thread under its thread, and
   * an act on a chain under that chain, as a control of the engine reaches them. */
  reach(id: string): Reach {
    const byId = new Map(this.acts.map((act) => [act.id, act]));
    const steps = byId.get(id)?.kind === "prompt";
    const under = (act: ActRow): boolean => {
      if (act.on === id) return true;
      for (let up: ActRow | undefined = act; up; up = byId.get(up.by))
        for (let at: ActRow | undefined = up; at; at = at.thread ? byId.get(at.thread) : undefined)
          if (at.id === id) return true;
      return false;
    };
    const acts = this.acts.filter(
      (act) =>
        act.id !== id &&
        !act.done &&
        WORK.includes(act.kind) &&
        !(steps && act.kind === "rung" && act.by === id) &&
        under(act),
    );
    return { acts, chains: [...new Set(acts.map((act) => act.on))] };
  }
  /** A cancel of the act that the operator names, or of his last act on the chain shown when he names none, and of
   * that act alone with what stands under it: never the thread or the chain that it stands on. A cancel that names
   * no act and ends a message takes the message back, so the operator writes it again. A cancel that ends running
   * work asks him first, and ends it once he confirms. */
  async cancel(id = ""): Promise<void> {
    const act = id ? this.acts.find((one) => one.id === id) : this.lastOwn;
    if (!act || act.done) {
      this.notice = id
        ? `${id} is no act that runs.`
        : "Nothing that you sent runs on this chain. /cancel <id> ends another act.";
      return;
    }
    const cancel: Cancel = { act, reach: this.reach(act.id), back: !id && this.isUserPrompt(act) };
    if (cancel.reach.acts.length) this.emit("confirm", cancel);
    else await this.end(cancel);
  }
  /** A cancel that the operator chose, or confirmed when it ends running work. */
  async end({ act, reach, back }: Cancel): Promise<void> {
    this.cancels.add(act.id);
    await this.engine.cancel(act.id);
    const ended = reach.acts.length;
    const under = `${ended} ${ended === 1 ? "act" : "acts"} under it on ${this.chainsOf(reach)}`;
    if (back) {
      // The words go back to where they went: the thread that they answered, or a new thread when they began one, and
      // not to the thread lit at this moment, which may be the thread that this cancel ends.
      const into = this.sentTo.get(act.id);
      this.aim(into);
      this.pinned = into ?? null;
      await this.restore(String(act.words[1] ?? ""));
      this.notice = `You took back ${act.id}${ended ? `, and ended ${under}` : ""}. Change it in the input, and send it again.`;
    } else this.notice = `${act.id} is cancelled${ended ? `, and ${under}` : ""}.`;
    await this.refresh();
  }
  /** The thread that each message of the input went to, or undefined for a message that began a thread. */
  private readonly sentTo = new Map<string, string | undefined>();
  /** The acts that a cancel of the operator ended, whose notice that cancel gave. */
  private readonly cancels = new Set<string>();
  /** The chains that a cancel reaches, each with its label, in the order of the chains. */
  chainsOf(reach: Reach): string {
    return this.chains
      .filter((chain) => reach.chains.includes(chain.id))
      .map((chain) => `${chain.id} (${this.labelOf(chain.id)})`)
      .join(", ");
  }
  async undo(): Promise<void> {
    if (this.paused || this.activity.some((act) => !act.done && ["prompt", "rung"].includes(act.kind)))
      throw new Error("Let the current prompt finish, or cancel and wake it before undo.");
    const prompt = this.activity.findLast((act) => this.isUserPrompt(act));
    if (!prompt) throw new Error("There is no user message to undo.");
    const source = this.selected;
    this.redo.push({ from: source, to: await this.rewind(prompt.id) });
    this.notice = "Message removed from this branch. Module and files keep their current state.";
    this.save();
  }

  /** Set the actor of the selected chain by a rung of the operator. Until that rung has run, the session shows the
   * choice. A prompt leaves its actor unsaid, so each of its rungs asks the actor of the chain at that time. */
  private async choose(actor: string): Promise<void> {
    const chain = this.selected;
    const choice: { actor: string; rung?: string } = { actor };
    this.choices.set(chain, choice);
    this.actor = actor;
    try {
      choice.rung = await this.engine.rung({ word: `actor = ${JSON.stringify(actor)}`, on: chain });
    } catch (error) {
      if (this.choices.get(chain) === choice) this.choices.delete(chain);
      throw error;
    }
    this.track(choice.rung);
  }

  /** The last message of the operator in a thread that the input can take back: one that still runs, or that a cancel
   * ended, and that he did not take back yet. */
  lastMessage(thread: string): ActRow | undefined {
    return this.acts.findLast(
      (act) =>
        act.thread === thread &&
        act.kind === "prompt" &&
        this.isUserPrompt(act) &&
        !this.retaken.has(act.id) &&
        (!act.done || cancelled(act)),
    );
  }
  /** The messages that the operator took back from the box of a thread, which the box does not offer again. */
  private readonly retaken = new Set<string>();
  /** The last message of the operator in a thread that still runs, or that a cancel ended, taken back: a message that
   * runs is cancelled, and its text comes back to be edited and sent again. The cancelled message leaves the box. */
  async takeBackMessage(thread: string): Promise<void> {
    const last = this.lastMessage(thread);
    if (!last) return;
    this.retaken.add(last.id);
    if (!last.done) {
      this.cancels.add(last.id);
      await this.engine.cancel(last.id);
    }
    await this.refresh();
    this.emit("compose", String(last.words[1] ?? ""));
  }
  /** A message to a thread, from the box at the end of the thread. It goes to the thread whatever the input is
   * doing, and a message to a thread that is over opens it again. */
  async reply(thread: string, input: string): Promise<void> {
    if (!input.trim() || this.host.reading) return;
    this.error = "";
    this.notice = "";
    const pending = this.host.pending.size > 0;
    const id = await this.engine.prompt("str", { message: input.trim(), on: thread });
    this.notice = `Your message went to the thread ${thread}.`;
    if (pending) this.emit("resume");
    this.track(id);
    await this.refresh();
  }

  async submit(input: string): Promise<void> {
    const text = input.trim();
    if (!text) return;
    this.error = "";
    this.notice = "";
    // No program of Python starts with a slash, so a slash command is a command under edit too, as it is in Python.
    if (text.startsWith("/")) await this.command(text);
    else if (this.editing) {
      await this.engine.result(
        await this.engine.rung({
          word: `write(Text(${JSON.stringify(this.editing)}, ${JSON.stringify(input)}))`,
          on: this.selected,
        }),
      );
      this.notice = "Program updated and replayed.";
      this.editing = undefined;
    } else if (text.startsWith("!")) await this.command(`/bash ${text.slice(1).trimStart()}`);
    else {
      const pending = this.operatorPrompt;
      if (pending) await this.answer(pending, input);
      else {
        await this.attachFiles(input);
        this.redo = [];
        const pending = this.host.pending.size > 0;
        const thread = this.target;
        this.fresh = false;
        this.pinned = undefined;
        const id = thread
          ? await this.engine.prompt("str", { message: this.withImages(input), on: thread })
          : await this.engine.prompt(this.shape, { message: this.withImages(input), on: this.selected });
        if (thread) this.notice = `Your message went to the thread ${thread}.`;
        this.sentTo.set(id, thread);
        delete this.images[this.selected];
        this.save();
        if (pending) this.emit("resume");
        this.track(id);
      }
    }
    await this.refresh();
  }

  /** Runs a word on the chain on screen, as /run does, and gives, once the word ended, the value of the last act that
   * it made on that chain, as the record holds it once that act is done, or undefined for a word that made no act. A
   * word that raises or that the gate refuses rejects with its error. */
  async runWord(word: string): Promise<unknown> {
    this.findings = [];
    const on = this.selected;
    const id = await this.engine.rung({ word, on });
    try {
      await this.engine.result(id);
      const made = (await this.engine.transcript({ on })).filter(
        (fact) => fact[2] === id && new RegExp(`^${String(fact[0])}\\d+$`).test(String(fact[1])),
      );
      const last = made.at(-1);
      return last ? await this.engine.result(String(last[1])) : undefined;
    } finally {
      await this.refresh();
    }
  }

  /** The extensions that the life runs, in the order that it enabled them, each with its life word. */
  async extensionsRun(): Promise<{ name: string; life: string }[]> {
    return enabled(await this.engine.transcript({ on: ROOT })).map(({ name, life }) => ({ name, life }));
  }

  async command(text: string): Promise<void> {
    const space = text.indexOf(" ");
    const command = text.slice(1, space < 0 ? undefined : space);
    const argument = space < 0 ? "" : text.slice(space + 1).trim();
    switch (command) {
      case "reload":
        await this.host.reload();
        await this.reloadCatalog();
        this.notice = "Provider settings reloaded. The life stands on the new settings.";
        break;
      case "retry":
        await this.host.reload();
        this.notice = "Each model that waited to retry is asked again now, on the provider settings of now.";
        break;
      case "provider": {
        const [verb, name, url, id, window, key_env, protocol] = argument.split(/\s+/);
        if (!name || !["add", "remove"].includes(verb ?? ""))
          throw new Error(
            "Use /provider add <name> [URL model window key_env protocol], or /provider remove <name>.",
          );
        const value =
          verb === "remove"
            ? false
            : url
              ? {
                  url,
                  protocol: protocol || "chat",
                  ...(key_env && key_env !== "-" ? { key_env } : {}),
                  models: { [id || ""]: { window: Number(window) } },
                }
              : true;
        if (!(await this.host.provider(name, value))) {
          this.notice = "Codex sign-in canceled.";
          break;
        }
        await this.reloadCatalog();
        this.notice =
          name === "codex"
            ? verb === "remove"
              ? "Codex subscription disconnected."
              : "Codex subscription connected."
            : `Provider ${name} ${verb === "remove" ? "removed" : "added"}. The settings are saved in .furb/config.json.`;
        if (name === "codex" && verb === "add") this.emit("codexModels");
        break;
      }
      case "name":
        this.sessionName = argument || basename(this.host.directory);
        this.save();
        break;
      case "pause":
        await this.engine.pause(argument || this.selected);
        this.notice = "Paused. In-flight work can finish.";
        break;
      case "wake": {
        if (this.host.pending.size) {
          this.emit("resume");
          break;
        }
        // With no chain named, the wake goes to the whole life: a pause of one chain holds the chains it made, and the
        // chains that stand outside it, so a wake of the selected chain alone leaves them paused.
        const stopped = argument
          ? [argument]
          : [
              ...new Set([
                this.selected,
                ...this.chains.filter((chain) => chain.paused && !chain.done).map((chain) => chain.id),
              ]),
            ];
        for (const id of stopped) await this.engine.wake(id);
        this.notice = stopped.length > 1 ? `Work resumed on ${stopped.length} chains.` : "Work resumed.";
        break;
      }
      case "cancel":
        await this.cancel(argument);
        break;
      case "extensions": {
        const each = (await this.extensionsRun()).map(({ name, life }) =>
          life ? `${name} with ${life}` : name,
        );
        this.notice = each.length
          ? `This life runs ${each.join(", ")}. Run a word of one with /run.`
          : "This life runs no extension.";
        break;
      }
      case "chain":
        await this.select(await this.engine.chain({ label: argument || "New chain" }));
        break;
      case "fork":
        await this.select(
          await this.engine.chain({ label: argument || `${this.label} fork`, source: this.selected }),
        );
        break;
      case "undo":
        await this.undo();
        break;
      case "redo": {
        const point = this.redo.at(-1);
        if (!point || point.to !== this.selected) throw new Error("There is no message to redo here.");
        this.redo.pop();
        await this.select(point.from);
        this.save();
        break;
      }
      case "queue":
        if (!argument) throw new Error("Use /queue followed by a message.");
        this.enqueue(argument);
        break;
      case "autocollapse":
        this.preferences.foldThreads = !this.preferences.foldThreads;
        this.preferences.save();
        this.notice = this.preferences.foldThreads
          ? "The threads that are over fold automatically."
          : "The threads that are over stay open.";
        break;
      case "newthread":
        this.aim();
        this.notice = "Your next message starts a new thread.";
        break;
      case "grant": {
        const amount = Number(argument);
        if (!argument || !Number.isFinite(amount) || amount < 0)
          throw new Error("Use /grant followed by a dollar amount.");
        const id = await this.engine.grant({ usd: amount, on: this.selected });
        const got = await this.engine.outcome(id);
        if (got.done) throw new Error(JSON.stringify(got.value));
        this.notice = `Budget set to ${dollars(amount)}. Use /wake if paused.`;
        break;
      }
      case "context": {
        const amount = Number(argument);
        if (!argument || !Number.isFinite(amount) || amount < 0 || amount > 1)
          throw new Error("Use /context with a number from 0 to 1.");
        await this.engine.grant({ share: amount, on: this.selected });
        break;
      }
      case "model": {
        if (!argument) throw new Error("Use /model followed by a model name.");
        const name = this.roster.some(([candidate]) => candidate === argument)
          ? argument
          : await this.host.model(argument);
        const entry = this.roster.find(([candidate]) => candidate === name);
        // A model of the catalog that the roster does not hold joins the roster of the next life, which the session
        // opens on its record at once, and which the model goes to.
        if (!entry && name) {
          this.models = [...new Set([...this.models, name])];
          this.save();
          this.emit("reopen", `/model ${name}`);
          break;
        }
        if (!entry)
          throw new Error(
            `Choose one of ${this.roster
              .filter(([candidate]) => candidate !== "operator")
              .map(([candidate]) => candidate)
              .join(", ")}.`,
          );
        const current = this.actorChoice.effort;
        const effort = entry[1].includes(current) ? current : entry[1][0];
        await this.choose(effort ? `${entry[0]}/${effort}` : entry[0]);
        break;
      }
      case "effort": {
        if (!argument) throw new Error("Use /effort followed by a level.");
        const { model } = this.actorChoice;
        const offered = this.roster.find(([name]) => name === model)?.[1] ?? [];
        if (!offered.includes(argument))
          throw new Error(`This model offers ${offered.join(", ") || "no reasoning efforts"}.`);
        await this.choose(`${model}/${argument}`);
        break;
      }
      case "run": {
        this.findings = [];
        const id = await this.engine.rung({ word: argument, on: this.selected });
        this.view = "feed";
        if ((await this.engine.outcome(id)).done) {
          try {
            await this.engine.result(id);
          } catch {
            await this.refresh();
          }
        } else this.track(id);
        break;
      }
      case "shape":
        if (!shapes().includes(argument)) throw new Error(`Choose ${shapes().join(", ")}.`);
        this.shape = argument;
        break;
      case "changes":
        if (argument && !(scopes as readonly string[]).includes(argument))
          throw new Error(`Choose ${scopes.join(", ")}.`);
        this.review((argument as Scope) || this.scope);
        break;
      case "theme":
        if (!isThemeChoice(argument))
          throw new Error(`Choose terminal, ${Object.keys(palettes).join(", ")}.`);
        this.theme = argument;
        break;
      case "bash":
        this.track(await this.engine.bash(argument, { on: this.selected }));
        this.view = "feed";
        break;
      case "read":
      case "cd":
        this.track(
          await this.engine.rung({ word: `${command}(${JSON.stringify(argument)})`, on: this.selected }),
        );
        if (command === "read") this.view = "feed";
        break;
      case "edit": {
        // The latest prompt with a program.
        const id = argument || this.activity.findLast((act) => this.editable(act))?.id;
        if (!id) throw new Error("There is no prompt program to edit.");
        const got = await this.engine.read(id, { on: this.selected });
        this.editing = id;
        this.emit("compose", this.drafts[this.draftKey] ?? got.content);
        this.notice = "Edit the Python program, then submit to replay it.";
        break;
      }
      case "feed": {
        const space = argument.indexOf(" ");
        const id = space < 0 ? argument : argument.slice(0, space);
        const text = space < 0 ? "" : argument.slice(space + 1);
        if (!id) throw new Error("Use /feed followed by an act id and text.");
        // A fed text is one line of input, and no text closes the input.
        await this.engine.write({ path: `${id}/stdin`, content: text && `${text}\n` }, { on: this.selected });
        break;
      }
      case "close": {
        const split = argument.indexOf(" ");
        if (split < 0) throw new Error("Use /close followed by an act id and a JSON value.");
        // The record reader keeps what JSON.parse loses: 2.0 stays a float, and a whole number past the safe range
        // is refused.
        await this.engine.close(decodeRecord(argument.slice(split + 1)), { id: argument.slice(0, split) });
        break;
      }
      case "export": {
        if (!argument) throw new Error("Use /export followed by a file path.");
        const path = this.path(argument);
        await writeFile(
          path,
          JSON.stringify({ chain: this.selected, turns: this.turns, program: this.program }, null, 2),
          { flag: "wx" },
        );
        this.notice = `Exported to ${shortenHome(path)}.`;
        break;
      }
      case "share": {
        const path = argument
          ? this.path(argument)
          : join(furbDirectory(this.host.directory, "shares"), `${Date.now()}.html`);
        await mkdir(dirname(path), { recursive: true });
        await writeFile(path, shareHtml(this), { flag: "wx", mode: 0o600 });
        this.notice = `Conversation saved to ${shortenHome(path)}.`;
        this.emit("shared", path, shareMarkdown(this));
        break;
      }
      default:
        throw new Error(`Unknown command /${command}. Press F1 for the command list.`);
    }
  }

  async reloadCatalog(): Promise<void> {
    this.catalog = await this.host.catalog();
    this.catalogRead = true;
    const authenticated = (models: readonly [string, string[], number][]) =>
      models
        .filter(([name]) => name.startsWith("codex:"))
        .map(([name, efforts, window]) => [name, efforts, window]);
    if (
      !this.demo &&
      this.host.role === "write" &&
      JSON.stringify(authenticated(this.roster)) !== JSON.stringify(authenticated(this.catalog))
    )
      await this.host.reload();
    this.setup = await this.host.setup();
    await this.refresh();
  }

  get cost(): number {
    return Math.max(this.savedCost, this.host.cost);
  }
  save(): void {
    this.preferences.save();
    const record = this.host.record;
    // A recorded life keeps no view beside its log, which a later reader takes as it was made, and a record open to
    // read stays as it was.
    if (!record || this.host.recorded || this.host.reading) return;
    const path = `${record}.ui.json`;
    this.savedCost = this.cost;
    const view: SavedView = {
      ...Object.fromEntries(kept.map((key) => [key, this[key]])),
      cost: this.savedCost,
    };
    saveFile(path, JSON.stringify(view));
  }
  /** The window reaches the life again, after the life ended in another window or the host was lost or stopped. */
  async reopen(): Promise<void> {
    await this.host.reopen();
    this.error = "";
    this.notice = this.host.lost || "The life is open again.";
    this.emit("change");
  }
  /** The host of the life ends by force, for a host that the end of the life does not reach. */
  async stopHost(): Promise<void> {
    await this.host.stop();
    this.notice = "The host of the life was stopped. /reopen opens the life again.";
    this.emit("change");
  }
  /** Save the view, then let the session go whatever the save came to: the window detaches from the life, which goes
   * on when it runs in a host of its own, or the life ends when end is set. */
  async dispose(end = false): Promise<void> {
    clearTimeout(this.noticeTimer);
    this.closed = true;
    this.host.off("change", this.changed);
    this.host.off("fault", this.fail);
    this.host.off("facts", this.factsChanged);
    try {
      this.save();
    } finally {
      await this.refreshTask?.catch(() => {});
      await (end ? this.host.end() : this.host.dispose());
    }
  }
}
