import { randomUUID } from "node:crypto";
import { EventEmitter } from "node:events";
import { readFileSync } from "node:fs";
import { mkdir, writeFile } from "node:fs/promises";
import { basename, dirname, join, resolve } from "node:path";
import type { Fact, ImageAttachment, LiveAct, TuiContext, Turn } from "@furb/engine";
import {
  actorParts,
  decodeRecord,
  furbDirectory,
  imagePath,
  imageReference,
  imageReferences,
  saveFile,
  shapes,
} from "@furb/engine";
import type { FileChange } from "@furb/engine/world";
import { createTwoFilesPatch } from "diff";
import type { Engine, HostView } from "./bridge.ts";
import { refusal } from "./conversation.ts";
import { expandHome, projectFiles, shortenHome } from "./files.ts";
import { Preferences } from "./preferences.ts";
import { shareHtml, shareMarkdown } from "./share.ts";
import { palettes, type ThemeName } from "./theme.ts";

/** The views of a chain: the feed of what it did, the transcript that the model reads, and the files it changed. */
export const views = ["feed", "transcript", "changes"] as const;
export type View = (typeof views)[number];
/** An act as the views read it. A command that printed more than a row carries holds the tail of each stream, and
 * the length of all it printed; the view reads the whole act when it shows the output. */
export type ActRow = LiveAct & { output?: number };
/** A change of a file, with the patch that shows it. */
export type ShownChange = FileChange & { patch: string };

export type SessionStatus =
  | "saved"
  | "idle"
  | "working"
  | "blocked"
  | "paused"
  | "done"
  | "error"
  | "opening";
export const statusLabels: Record<SessionStatus, string> = {
  saved: "Saved",
  idle: "Ready",
  working: "Working",
  blocked: "Input needed",
  paused: "Paused",
  done: "Finished, unread",
  error: "Error",
  opening: "Opening",
};
/** Whether an act is at work: it lives, no pause holds it, and its work takes time: a prompt, a rung, or an act the
 * World was started on. */
export const working = (act: ActRow): boolean =>
  !act.done && !act.paused && (act.kind === "prompt" || act.kind === "rung" || act.started);
/** Whether an act failed: a rung that the gate refused or whose run raised, or an act done with an exception other
 * than a cancel. */
export function failed(act: ActRow): boolean {
  if (act.run) return act.run.status === "failed" && !cancelled(act);
  const value = act.value;
  return Boolean(
    value && typeof value === "object" && "is" in value && "args" in value && value.is !== "CancelledError",
  );
}

/** Whether an act was cancelled: a rung whose run ended with a cancel, or an act done with a cancel. A cancel is what
 * the operator asked for, so it is no failure. */
export function cancelled(act: ActRow): boolean {
  if (act.run) return act.run.status === "failed" && /^CancelledError\b/.test(act.run.reason ?? "");
  const value = act.value;
  return Boolean(value && typeof value === "object" && "is" in value && value.is === "CancelledError");
}

export interface FollowUp {
  id: string;
  chain: string;
  text: string;
  shape: string;
  actor: string;
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
  "mode",
  "shape",
  "editing",
  "histories",
  "stashes",
  "started",
  "queued",
  "queueError",
  "images",
  "redo",
  "drafts",
  "scrolls",
  "folds",
] as const;
/** What `<record>.ui.json` holds: the kept fields of a session, and what the session had cost. */
export type SavedView = Partial<Pick<Session, (typeof kept)[number]>> & { cost?: number };

/** The saved view of a record: nothing when it has none, and the defaults with the reason when its file holds no
 * view that can be read. */
export function savedView(record: string): { view: SavedView; damage?: string } {
  const path = `${record}.ui.json`;
  try {
    const view: unknown = JSON.parse(readFileSync(path, "utf8"));
    if (view && typeof view === "object" && !Array.isArray(view)) return { view };
    throw new Error("It holds no map.");
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
  turns: Turn[] = [];
  changes: ShownChange[] = [];
  changePage = 0;
  program: Record<string, string> = {};
  /** The text of the search box, which the rows of the view must hold. */
  search = "";
  private message = "";
  private noticeTimer?: ReturnType<typeof setTimeout>;
  readonly errors: Record<string, string> = {};
  loading = false;
  paused = false;
  editing?: string;
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
  findings: string[] = [];
  rejectedWord = "";
  rejectedAct = "";
  queued: FollowUp[] = [];
  dispatched: string[] = [];
  images: Record<string, ImageAttachment[]> = {};
  queueHeld = false;
  queueError = "";
  redo: { from: string; to: string }[] = [];
  private draining = false;
  private savedCost = 0;
  histories: Record<string, string[]> = {};
  started: Record<string, number> = {};
  directory = "";
  private refreshTask?: Promise<void>;
  /** The count of changes of the act table that `acts` holds. */
  private counted = 0;
  private dirty = false;
  private closed = false;
  /** The page of changes and the count of changes that `changes` was read at. */
  private changesRead = "";
  /** The actor the operator chose for each chain, which the session shows until the rung that sets it has run. */
  private readonly choices = new Map<string, { actor: string; rung?: string }>();
  constructor(
    readonly life: Engine,
    readonly world: HostView,
    readonly demo = false,
    preferences?: Preferences,
  ) {
    super();
    this.selected = life.root;
    this.actor = world.actor;
    this.sessionName = basename(world.directory);
    const path = world.records.path;
    this.preferences =
      preferences ?? new Preferences(demo && path ? join(dirname(path), "ui-preferences.json") : undefined);
    if (path) {
      const { view, damage } = savedView(path);
      Object.assign(this, Object.fromEntries(kept.flatMap((key) => (key in view ? [[key, view[key]]] : []))));
      this.savedCost = view.cost ?? 0;
      // A saved queue waits for the choice of the operator before it sends.
      this.queueHeld = this.queued.length > 0;
      if (damage) this.notice = damage;
    }
    // A view that an older release saved, and that this release has no more, opens as the feed.
    if (!views.includes(this.view)) this.view = "feed";
    world.on("change", this.changed);
    world.on("facts", this.factsChanged);
    world.on("fault", this.fail);
    this.save();
  }
  private changed = () => {
    this.emit("change");
  };
  get theme(): ThemeName {
    return this.preferences.theme;
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
  set theme(value: ThemeName) {
    this.preferences.save(value);
  }
  /** A fact refreshes the views, whatever its kind, unless it is of a query, whose name holds an @. */
  private factsChanged = (facts: Fact[]) => {
    if (facts.some(([, id]) => !id.includes("@"))) void this.refresh().catch(this.fail);
  };
  fail = (error: unknown) => {
    this.error = error instanceof Error ? error.message : String(error);
    this.emit("change");
  };
  get error(): string {
    return this.errors[`${this.selected}:${this.view}`] ?? "";
  }
  set error(value: string) {
    this.errors[`${this.selected}:${this.view}`] = value;
  }

  refresh(): Promise<void> {
    this.dirty = true;
    if (this.refreshTask) return this.refreshTask;
    this.loading = true;
    this.emit("change");
    this.refreshTask = (async () => {
      do {
        this.dirty = false;
        const selected = this.selected;
        const { acts, whole, count, ...snapshot } = await this.world.snapshot(selected, this.counted);
        if (this.selected !== selected) {
          this.dirty = true;
          continue;
        }
        Object.assign(this, snapshot);
        // A snapshot carries the acts that changed since the session last read them, in the order of the table.
        if (whole) this.acts = acts;
        else if (acts.length) {
          const rows = new Map(this.acts.map((act) => [act.id, act]));
          for (const act of acts) rows.set(act.id, act);
          this.acts = [...rows.values()];
        }
        this.counted = count;
        // The acts and the actor of one snapshot are of one moment: the choice holds until its rung is done there.
        const choice = this.choices.get(selected);
        if (choice?.rung && this.acts.find((act) => act.id === choice.rung)?.done)
          this.choices.delete(selected);
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
        const page = `${this.changePage}:${this.world.changes}`;
        if (this.view === "changes" && page !== this.changesRead) {
          this.changesRead = page;
          this.changes = (await this.world.readChanges(this.changePage * 20, 20)).map((change) => ({
            ...change,
            patch: createTwoFilesPatch(change.path, change.path, change.before, change.after),
          }));
        }
        const rows = this.acts;
        for (const act of rows) if (!act.done) this.started[act.id] ??= Date.now();
        const lastWord = rows
          .filter((act) => act.on === this.selected && act.kind === "rung" && act.by === "operator")
          .at(-1);
        this.findings = lastWord ? refusal(this.turns, lastWord.id) : [];
        this.rejectedWord = this.findings.length ? String(lastWord?.words[0] ?? "") : "";
        this.rejectedAct = this.findings.length ? (lastWord?.id ?? "") : "";
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

  private fileList?: { directory: string; read: Promise<string[]> };
  /** The files of the project the session works in: read again when asked fresh or when the directory moved, and
   * otherwise the read already made, so every reader of one read sees the same list. */
  projectFiles(fresh = false): Promise<string[]> {
    const directory = this.workingDirectory;
    if (fresh || this.fileList?.directory !== directory)
      this.fileList = { directory, read: projectFiles(directory) };
    return this.fileList.read;
  }
  /** The directory that the paths of the selected chain resolve against, as the World resolves them. */
  get workingDirectory(): string {
    return resolve(this.world.directory, this.directory);
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
  /** What the session is doing, over the acts of one chain or of every chain. */
  status(chain?: string): SessionStatus {
    const acts = this.acts.filter((act) => !this.world.parts.hidden(act) && (!chain || act.on === chain));
    if (chain ? this.error : Object.values(this.errors).some(Boolean)) return "error";
    if (this.queueHeld && this.queued.length) return "blocked";
    if (this.world.pending.size) return "paused";
    if (chain ? this.operatorPrompt : this.world.prompts.size) return "blocked";
    if (acts.some(working)) return "working";
    if (this.paused || acts.some((act) => act.paused && !act.done)) return "paused";
    const latest = acts.at(-1);
    return latest && failed(latest) ? "error" : "idle";
  }
  /** A message or a word that the operator sends to a paused chain wakes the chain first, since the operator who
   * writes to it wants it to go on: work that a reopened record held starts again, and the pause over the chain ends.
   * It says whether the chain was paused. */
  async wakeForInput(): Promise<boolean> {
    if (!this.paused) return false;
    if (this.world.pending.size) await this.world.resume();
    await this.refresh();
    if (this.paused) await this.life.wake(this.selected);
    await this.refresh();
    return true;
  }
  /** The chains that the operator started and that finished their work while another chain was shown, which wait for
   * the operator to look at them, and the state that each chain had when the view last read it. */
  readonly unread = new Set<string>();
  readonly phases = new Map<string, SessionStatus>();
  get chains(): ActRow[] {
    return this.acts.filter((act) => act.kind === "chain");
  }
  get activity(): ActRow[] {
    return this.acts.filter((act) => act.on === this.selected && act.kind !== "chain");
  }
  /** What the answers of the chain cost, each token counted once. The first number of the usage of a turn is the
   * whole prompt of that call, which holds the reads and the writes of the cache, so the fresh input of a turn is
   * what is left of its prompt after them. */
  get spend(): { input: number; output: number; cacheRead: number; cacheWrite: number; dollars: number } {
    const sum = { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, dollars: 0 };
    for (const [, , usage] of this.turns) {
      if (!usage) continue;
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
  /** The share of the window that the last answer of the chain filled, as the ledger of a grant says it: the whole
   * prompt of that answer over the window of the model that the last prompt went to, or the window of the engine for
   * a model that the roster does not name. Nothing before the first answer. */
  get filled(): number | undefined {
    const usage = this.turns.findLast(([role, , used]) => role === "assistant" && used)?.[2];
    if (!usage) return undefined;
    const asked = this.activity.findLast((act) => act.kind === "prompt" && act.words[2] !== "operator");
    const names = this.roster.map(([name]) => name);
    const { model } = actorParts(String(asked?.words[2] || this.actor), names);
    const window = Number(this.roster.find(([name]) => name === model)?.[2]) || 200_000;
    return usage[0] / window;
  }
  get label(): string {
    return this.labelOf(this.selected);
  }
  get actorChoice(): { model: string; effort: string } {
    return actorParts(
      this.actor,
      this.roster.map(([name]) => name),
    );
  }
  get operatorPrompt() {
    return [...this.world.prompts.values()].find(
      (prompt) => this.acts.find((act) => act.id === prompt.id)?.on === this.selected,
    );
  }
  isUserPrompt(act: ActRow): boolean {
    return act.kind === "prompt" && act.by === "operator";
  }
  /** The names of the extensions whose life words the World plays on each chain. */
  get played(): string[] {
    return this.world.extensions.flatMap((one) => (one.life ? [one.name] : []));
  }
  /** The text at a path, as a read of the chain on screen gives it: a file, or the program of a prompt, which its door
   * gives. A read of the operator outside an act tells nothing, so it takes no show. */
  async read(path: string): Promise<string> {
    return ((await this.life.call("read", [path], { on: this.selected })) as { content: string }).content;
  }
  /** The act that a name or a door names: the act whose name is the first part of the path. */
  actOf(path: string): ActRow | undefined {
    const [name] = path.split("/");
    return this.acts.find((act) => act.id === name);
  }
  /** How many acts made an act, one under the other, up to the operator or the outside. */
  depth(act: ActRow): number {
    let depth = 0;
    for (
      let maker = this.acts.find((one) => one.id === act.by);
      maker;
      maker = this.acts.find((one) => one.id === maker?.by)
    )
      depth++;
    return depth;
  }
  async attachImage(path: string): Promise<void> {
    if (!(await this.world.route(this.actor)).input.includes("image"))
      throw new Error("Choose a model that accepts images before attaching one.");
    const image = await this.world.attachImage(this.path(path));
    const images = this.images[this.selected] ?? [];
    this.images[this.selected] = images;
    if (!images.some((current) => current.uri === image.uri)) images.push(image);
    this.save();
    this.notice = `${image.name} attached.`;
  }
  private withImages(text: string): string {
    return [text, ...(this.images[this.selected] ?? []).map(imageReference)].join("\n");
  }
  labelOf(id: string): string {
    return id === this.life.root
      ? "Main"
      : String(this.chains.find((chain) => chain.id === id)?.words[0] || "Chain");
  }
  async select(id: string): Promise<void> {
    // The turns and the program of the chain left are not the chain selected, which shows them once it has read them.
    if (id !== this.selected) {
      this.editing = undefined;
      this.turns = [];
      this.program = {};
    }
    this.selected = id;
    this.search = "";
    await this.refresh();
  }
  show(view: View): void {
    this.view = view;
    this.search = "";
    this.emit("change");
    void this.refresh().catch(this.fail);
  }

  private track(id: string): void {
    void this.life
      .result(id)
      .then(
        () => {},
        async (error: unknown) => {
          if (this.closed) return;
          this.notice = "";
          // A finished act carries its failure in the record and in the feed.
          if (!(await this.life.outcome(id)).done) throw error;
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
      actor: this.actor,
    });
    delete this.images[this.selected];
    this.save();
    this.notice = "Follow-up queued.";
    void this.drainQueue().catch(this.fail);
  }
  async drainQueue(): Promise<void> {
    if (this.closed || this.draining || this.queueHeld || this.world.pending.size || !this.queued.length)
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
              !this.acts.find((act) => act.id === entry.chain)?.paused &&
              !this.acts.some(
                (act) => act.on === entry.chain && !act.done && ["prompt", "rung"].includes(act.kind),
              ),
          );
    try {
      for (let entry = next(); entry; entry = next()) {
        passed.add(entry.id);
        await this.prompting(entry.text, entry.chain);
        if (!queued(entry)) continue;
        this.track(await this.world.sendQueued(entry));
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
  /** What the parts of the extensions do before a message is sent on a chain. */
  private async prompting(text: string, chain = this.selected): Promise<void> {
    // The chain answers where it stands itself, since a life that takes no files binds no cwd.
    const [, here] = (await this.life.call("ask", ["cwd", chain], {})) as [unknown, string];
    await this.world.parts.prompting(text, this.tuiContext(chain, resolve(this.world.directory, here)));
  }
  /** What a part of an extension is given on a chain whose paths resolve against a directory, with the files of the
   * project as the view has read them. */
  tuiContext(chain: string, directory: string, files = (): string[] | undefined => undefined): TuiContext {
    return {
      life: this.life,
      chain,
      directory,
      acts: this.acts,
      call: (verb, args = [], kwargs = {}) => this.life.call(verb, args, { on: chain, ...kwargs }),
      path: (typed) => resolve(directory, expandHome(typed)),
      projectFiles: files,
      notify: (message) => {
        this.notice = message;
      },
      submit: (message) => this.submit(message),
      track: (id) => this.track(id),
      show: (view) => {
        this.view = view;
      },
    };
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
    const rung = await this.life.rung(
      `chain(${JSON.stringify(label)}, ${JSON.stringify(source)}, ${filter})`,
      {
        on: source,
      },
    );
    await this.life.result(rung);
    await this.refresh();
    const chain = this.chains.find((chain) => chain.by === rung);
    if (!chain) throw new Error("The rung made no chain.");
    await this.select(chain.id);
    return chain.id;
  }
  /** A new branch of the chain of an act, which the session selects. The branch reads the transcript of that chain
   * through the act, or up to a message of the operator, whose text returns to the composer to be sent again. The
   * module and the files keep the state they have. */
  async rewind(id: string, label?: string): Promise<string> {
    if (this.paused) throw new Error("Resume this chain before rewinding.");
    const chain = this.acts.find((act) => act.id === id)?.on;
    if (!chain) throw new Error(`There is no act ${id} to rewind to.`);
    if (chain !== this.selected) await this.select(chain);
    const at = this.activity.findIndex((act) => act.id === id);
    const act = this.activity[at];
    if (!act || this.world.parts.hidden(act)) throw new Error(`There is no act ${id} to rewind to.`);
    const message = this.isUserPrompt(act);
    // Every act after the point leaves the branch, a grant or a chain among them, so that the branch reads no ceiling
    // and no branch that came later.
    const omitted = this.activity.slice(message ? at : at + 1).map((later) => later.id);
    const branch = await this.branch(label ?? this.branchLabel(), omitted);
    if (message) await this.restore(String(act.words[1] ?? ""));
    this.notice = message
      ? "The message is back in the input on a new branch. The module and the files keep their state."
      : `The new branch reads the transcript through ${id}. The module and the files keep their state.`;
    return branch;
  }
  /** A message sent before as the draft of the composer, with the images it referred to attached again. */
  private async restore(text: string): Promise<void> {
    let message = text;
    for (const reference of imageReferences(message)) {
      const image = await this.world.attachImage(imagePath(this.world.imageDirectory, reference.uri).path);
      image.name = reference.name || image.name;
      this.images[this.selected] ??= [];
      this.images[this.selected]?.push(image);
      message = message.replace(reference.text, "");
    }
    this.emit("compose", message.trimEnd());
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
   * choice and sends prompts to it. */
  private async choose(actor: string): Promise<void> {
    const chain = this.selected;
    const choice: { actor: string; rung?: string } = { actor };
    this.choices.set(chain, choice);
    this.actor = actor;
    try {
      choice.rung = await this.life.rung(`actor = ${JSON.stringify(actor)}`, { on: chain });
    } catch (error) {
      if (this.choices.get(chain) === choice) this.choices.delete(chain);
      throw error;
    }
    this.track(choice.rung);
  }

  async submit(input: string): Promise<void> {
    const text = input.trim();
    if (!text) return;
    this.error = "";
    this.notice = "";
    const prefixed = this.world.parts.prefixed(text);
    // No program of Python starts with a slash, so a slash command is a command under edit too, as it is in Python.
    if (text.startsWith("/")) await this.command(text);
    else if (this.editing) {
      await this.life.result(
        await this.life.rung(`write(Text(${JSON.stringify(this.editing)}, ${JSON.stringify(input)}))`, {
          on: this.selected,
        }),
      );
      this.notice = "Program updated and replayed.";
      this.editing = undefined;
    } else if (prefixed) await this.command(prefixed);
    else {
      const pending = this.operatorPrompt;
      if (pending) await this.world.answer(pending.id, input);
      else {
        await this.prompting(input);
        this.redo = [];
        const pending = this.world.pending.size > 0;
        const id = await this.life.prompt(this.shape, this.withImages(input), {
          on: this.selected,
          to: this.actor,
        });
        delete this.images[this.selected];
        this.save();
        if (pending) this.emit("resume");
        this.track(id);
      }
    }
    await this.refresh();
  }

  async command(text: string): Promise<void> {
    const space = text.indexOf(" ");
    const command = text.slice(1, space < 0 ? undefined : space);
    const argument = space < 0 ? "" : text.slice(space + 1).trim();
    switch (command) {
      case "name":
        this.sessionName = argument || basename(this.world.directory);
        this.save();
        break;
      case "pause":
        await this.life.pause(argument || this.selected);
        this.notice = "Paused. In-flight work can finish.";
        break;
      case "wake":
        if (this.world.pending.size) {
          this.emit("resume");
          break;
        }
        await this.life.wake(argument || this.selected);
        this.notice = "Work resumed.";
        break;
      case "cancel":
        await this.life.cancel(argument || this.selected);
        this.notice = "Work cancelled.";
        break;
      case "chain":
        await this.select(await this.life.chain(argument || "New chain"));
        break;
      case "fork":
        await this.select(await this.life.chain(argument || `${this.label} fork`, this.selected));
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
        this.preferences.foldRungs = !this.preferences.foldRungs;
        this.preferences.save();
        this.notice = this.preferences.foldRungs
          ? "Completed rungs collapse automatically."
          : "Rungs keep their open state.";
        break;
      case "model": {
        if (!argument) throw new Error("Use /model followed by a model name.");
        const name = await this.world.model(argument);
        const entry = this.roster.find(([candidate]) => candidate === name);
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
        const id = await this.life.rung(argument, { on: this.selected });
        this.view = "feed";
        if ((await this.life.outcome(id)).done) {
          try {
            await this.life.result(id);
          } catch {
            await this.refresh();
          }
        } else this.track(id);
        break;
      }
      case "shape":
        if (!shapes.some((name) => name === argument)) throw new Error(`Choose ${shapes.join(", ")}.`);
        this.shape = argument;
        break;
      case "theme":
        if (!Object.hasOwn(palettes, argument))
          throw new Error(`Choose ${Object.keys(palettes).join(", ")}.`);
        this.theme = argument as ThemeName;
        this.preferences.save(this.theme);
        break;
      case "edit": {
        // The latest prompt with a program: one of its rungs holds a word the gate let run on this chain.
        const id =
          argument ||
          [...this.activity]
            .reverse()
            .find(
              (act) =>
                act.kind === "prompt" &&
                this.activity.some((rung) => rung.by === act.id && Object.hasOwn(this.program, rung.id)),
            )?.id;
        if (!id) throw new Error("There is no prompt program to edit.");
        const program = await this.read(id);
        this.editing = id;
        this.emit("compose", this.drafts[this.draftKey] ?? program);
        this.notice = "Edit the Python program, then submit to replay it.";
        break;
      }
      case "close": {
        const split = argument.indexOf(" ");
        if (split < 0) throw new Error("Use /close followed by an act id and a JSON value.");
        // The record reader keeps what JSON.parse loses: 2.0 stays a float, and a whole number past the safe range
        // is refused.
        await this.life.close(decodeRecord(argument.slice(split + 1)), argument.slice(0, split));
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
          : join(furbDirectory(this.world.directory, "shares"), `${Date.now()}.html`);
        await mkdir(dirname(path), { recursive: true });
        await writeFile(path, shareHtml(this), { flag: "wx", mode: 0o600 });
        this.notice = `Conversation saved to ${shortenHome(path)}.`;
        this.emit("shared", path, shareMarkdown(this));
        break;
      }
      default: {
        const given = this.world.parts.commands.get(command);
        if (!given) throw new Error(`Unknown command /${command}. Press F1 for the command list.`);
        await given.run(argument, this.tuiContext(this.selected, this.workingDirectory));
      }
    }
  }

  get cost(): number {
    return Math.max(this.savedCost, this.world.cost);
  }
  save(): void {
    this.preferences.save(this.theme);
    const record = this.world.records.path;
    if (!record) return;
    const path = `${record}.ui.json`;
    this.savedCost = this.cost;
    const view: SavedView = {
      ...Object.fromEntries(kept.map((key) => [key, this[key]])),
      cost: this.savedCost,
    };
    saveFile(path, JSON.stringify(view));
  }
  /** Save the view, then end the World whatever the save came to. */
  async dispose(): Promise<void> {
    clearTimeout(this.noticeTimer);
    this.closed = true;
    this.world.off("change", this.changed);
    this.world.off("fault", this.fail);
    this.world.off("facts", this.factsChanged);
    try {
      this.save();
    } finally {
      await this.refreshTask?.catch(() => {});
      await this.world.dispose();
    }
  }
}
