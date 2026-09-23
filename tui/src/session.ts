import { randomUUID } from "node:crypto";
import { EventEmitter } from "node:events";
import { readFileSync, renameSync, writeFileSync } from "node:fs";
import { mkdir, writeFile } from "node:fs/promises";
import { basename, dirname, join, resolve } from "node:path";
import type { Fact, ImageAttachment, LiveAct, Turn, Usage } from "@furb/engine";
import {
  actorParts,
  decodeRecord,
  furbDirectory,
  imagePath,
  imageReference,
  imageReferences,
  shapes,
} from "@furb/engine";
import type { FileChange } from "@furb/engine/world";
import { createTwoFilesPatch } from "diff";
import type { Engine, HostView } from "./bridge.ts";
import { expandHome, fileReferences, projectFiles } from "./files.ts";
import { dollars } from "./format.ts";
import { Preferences } from "./preferences.ts";
import { shareHtml, shareMarkdown } from "./share.ts";
import { palettes, type ThemeName } from "./theme.ts";

export type View = "conversation" | "program" | "activity" | "facts" | "transcript" | "changes";
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
/** Whether an act is at work: it lives, no pause holds it, and its kind is one whose work takes time. */
export const working = (act: ActRow): boolean =>
  !act.done && !act.paused && ["prompt", "rung", "bash", "wait"].includes(act.kind);
/** Whether an act failed: a rung that the gate refused or whose run raised, or an act done with an exception other
 * than a cancel. */
export function failed(act: ActRow): boolean {
  if (act.run) return act.run.status === "failed";
  const value = act.value;
  return Boolean(
    value && typeof value === "object" && "is" in value && "args" in value && value.is !== "CancelledError",
  );
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
  "started",
  "ladder",
  "repls",
  "queued",
  "queueError",
  "images",
  "redo",
  "drafts",
  "scrolls",
  "panes",
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
  view: View = "conversation";
  actor: string;
  sessionName: string;
  acts: ActRow[] = [];
  turns: Turn[] = [];
  rendered: string[] = [];
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
  ladder?: string;
  readonly preferences: Preferences;
  mode: "prompt" | "python" = "prompt";
  shape = "str";
  drafts: Record<string, string> = {};
  /** Where each view was left, by its key. */
  scrolls: Record<string, Scroll> = {};
  panes = { inspector: 28 };
  folds: Record<string, boolean> = {};
  roster: [string, string[], number][] = [];
  findings: string[] = [];
  rejectedWord = "";
  rejectedAct = "";
  repls: Record<string, string[]> = {};
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
  /** The lower-case text of each fact that a search of the facts view has read, which it reads once. */
  private readonly factTexts = new WeakMap<Fact, string>();
  /** The facts that hold the search, with the count of facts they were read from. */
  private factRows = { search: "", seen: 0, rows: [] as Fact[] };
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
  private factsChanged = (facts: Fact[]) => {
    if (
      facts.some(
        ([kind, id]) =>
          [
            "chain",
            "prompt",
            "rung",
            "bash",
            "wait",
            "grant",
            "answer",
            "out",
            "exited",
            "close",
            "cancel",
            "pause",
            "wake",
            "tell",
          ].includes(kind) ||
          (kind === "done" && /^(prompt|rung|bash|wait|grant):/.test(id)),
      )
    )
      void this.refresh().catch(this.fail);
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
        const refused = this.turns
          .flatMap((turn) => turn[1])
          .find(
            (tag) =>
              typeof tag !== "string" &&
              tag[0] === "refused" &&
              tag[1].some(([key, value]) => key === "id" && value === lastWord?.id),
          );
        this.findings =
          refused && typeof refused !== "string"
            ? typeof refused[2] === "string"
              ? refused[2].split("\n")
              : Array.isArray(refused[2])
                ? refused[2].map(String)
                : []
            : [];
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
  /** The key of the draft that the composer shows: the selected chain, and the program it edits, the ladder it
   * reads, or its mode of input. */
  get draftKey(): string {
    return `${this.selected}:${this.editing ?? this.ladder ?? this.mode}`;
  }
  /** What the session is doing, over the acts of one chain or of every chain. */
  status(chain?: string): SessionStatus {
    const acts = this.acts.filter(
      (act) => act.kind !== "chain" && act.kind !== "grant" && (!chain || act.on === chain),
    );
    if (chain ? this.error : Object.values(this.errors).some(Boolean)) return "error";
    if (this.queueHeld && this.queued.length) return "blocked";
    if (this.world.held.size) return "paused";
    if (chain ? this.operatorPrompt : this.world.prompts.size) return "blocked";
    if (acts.some(working)) return "working";
    if (this.paused || acts.some((act) => act.paused && !act.done)) return "paused";
    const latest = acts.at(-1);
    return latest && failed(latest) ? "error" : "idle";
  }
  get chains(): ActRow[] {
    return this.acts.filter((act) => act.kind === "chain");
  }
  get activity(): ActRow[] {
    return this.acts.filter((act) => act.on === this.selected && act.kind !== "chain");
  }
  get usage(): Usage {
    return this.turns.reduce<Usage>(
      (sum, turn) => (turn[2] ? (sum.map((value, index) => value + (turn[2]?.[index] ?? 0)) as Usage) : sum),
      [0, 0, 0, 0, 0],
    );
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
    if (id !== this.selected) {
      this.ladder = undefined;
      this.editing = undefined;
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
    if (this.closed || this.draining || this.queueHeld || this.world.held.size || !this.queued.length) return;
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
        await this.attachFiles(entry.text, entry.chain);
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
  private async attachFiles(text: string, chain = this.selected): Promise<void> {
    const directory = resolve(this.world.directory, await this.life.cwd(chain));
    for (const path of await fileReferences(text, directory))
      await this.life.result(await this.life.rung(`read(${JSON.stringify(path)})`, { on: chain }));
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
  async undo(): Promise<void> {
    if (this.paused || this.activity.some((act) => !act.done && ["prompt", "rung"].includes(act.kind)))
      throw new Error("Let the current prompt finish, or cancel and wake it before undo.");
    const index = this.activity.findLastIndex((act) => this.isUserPrompt(act));
    const prompt = this.activity[index];
    if (!prompt) throw new Error("There is no user message to undo.");
    const source = this.selected;
    const omitted = this.activity.slice(index).map((act) => act.id);
    this.redo.push({ from: source, to: await this.branch(`${this.label} undo`, omitted) });
    let message = String(prompt.words[1] ?? "");
    for (const reference of imageReferences(message)) {
      const image = await this.world.attachImage(imagePath(this.world.imageDirectory, reference.uri).path);
      image.name = reference.name || image.name;
      this.images[this.selected] ??= [];
      this.images[this.selected]?.push(image);
      message = message.replace(reference.text, "");
    }
    this.emit("compose", message.trimEnd());
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
    } else if (text.startsWith("!")) await this.command(`/bash ${text.slice(1).trimStart()}`);
    else {
      const pending = this.operatorPrompt;
      if (pending) await this.world.answer(pending.id, input);
      else {
        await this.attachFiles(input);
        this.redo = [];
        const held = this.world.held.size > 0;
        const id = await this.life.prompt(this.shape, this.withImages(input), {
          on: this.selected,
          to: this.actor,
        });
        delete this.images[this.selected];
        this.save();
        if (held) this.emit("resume");
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
        if (this.world.held.size) {
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
        this.preferences.autoCollapseRungs = !this.preferences.autoCollapseRungs;
        this.preferences.save();
        this.notice = this.preferences.autoCollapseRungs
          ? "Completed rungs collapse automatically."
          : "Rungs keep their open state.";
        break;
      case "grant": {
        const amount = Number(argument);
        if (!argument || !Number.isFinite(amount) || amount < 0)
          throw new Error("Use /grant followed by a dollar amount.");
        const id = await this.life.grant({ usd: amount, on: this.selected });
        const got = await this.life.outcome(id);
        if (got.done) throw new Error(JSON.stringify(got.value));
        this.notice = `Budget set to ${dollars(amount)}. Use /wake if paused.`;
        break;
      }
      case "context": {
        const amount = Number(argument);
        if (!argument || !Number.isFinite(amount) || amount < 0 || amount > 1)
          throw new Error("Use /context with a number from 0 to 1.");
        await this.life.grant({ share: amount, on: this.selected });
        break;
      }
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
        if (this.ladder) {
          this.repls[this.ladder] ??= [];
          this.repls[this.ladder]?.push(id);
        }
        this.view = "program";
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
      case "bash":
        this.track(await this.life.bash(argument, { on: this.selected }));
        this.view = "activity";
        break;
      case "read": {
        this.track(await this.life.rung(`read(${JSON.stringify(argument)})`, { on: this.selected }));
        this.view = "conversation";
        break;
      }
      case "cd":
        this.track(await this.life.rung(`cd(${JSON.stringify(argument)})`, { on: this.selected }));
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
        const got = (await this.life.read(id, undefined, this.selected)) as { content: string };
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
        await this.life.write({ path: `${id}/stdin`, content: text && `${text}\n` }, this.selected);
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
        this.notice = `Exported to ${path}.`;
        break;
      }
      case "share": {
        const path = argument
          ? this.path(argument)
          : join(furbDirectory(this.world.directory, "shares"), `${Date.now()}.html`);
        await mkdir(dirname(path), { recursive: true });
        await writeFile(path, shareHtml(this), { flag: "wx", mode: 0o600 });
        this.notice = `Conversation saved to ${path}`;
        this.emit("shared", path, shareMarkdown(this));
        break;
      }
      default:
        throw new Error(`Unknown command /${command}. Press F1 for the command list.`);
    }
  }

  filteredFacts(): Fact[] {
    const search = this.search.toLowerCase();
    if (!search) return this.world.facts;
    if (this.factRows.search !== search) this.factRows = { search, seen: 0, rows: [] };
    const facts = this.world.facts;
    for (; this.factRows.seen < facts.length; this.factRows.seen++) {
      const fact = facts[this.factRows.seen] as Fact;
      const text = this.factTexts.get(fact) ?? JSON.stringify(fact).toLowerCase();
      this.factTexts.set(fact, text);
      if (text.includes(search)) this.factRows.rows.push(fact);
    }
    return this.factRows.rows;
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
    writeFileSync(`${path}.tmp`, JSON.stringify(view), { mode: 0o600 });
    renameSync(`${path}.tmp`, path);
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
