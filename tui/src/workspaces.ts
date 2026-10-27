import { randomUUID } from "node:crypto";
import { EventEmitter } from "node:events";
import { existsSync, mkdirSync, readFileSync } from "node:fs";
import { mkdir, readdir, realpath, rename, rm, stat, writeFile } from "node:fs/promises";
import { basename, dirname, join, resolve } from "node:path";
import { enabled, furbDirectory, ROOT, saveFile, scratch, store } from "furb";
import { openEngine, openingEngine } from "./bridge.ts";
import { checkoutOf, checkouts, mainCheckout } from "./checkout.ts";
import { expandHome } from "./files.ts";
import { type EngineOptions, roster } from "./models.ts";
import { Preferences } from "./preferences.ts";
import type { Carry } from "./reading.ts";
import { inspectRecords } from "./records.ts";
import { Session, type SessionStatus, savedView } from "./session.ts";

export interface SessionEntry {
  path: string;
  name: string;
  session?: Session;
  status: SessionStatus;
  unread: boolean;
  error?: string;
  modified?: number;
  size?: number;
  cost?: number;
  /** Whether the operator archived the session, which the sidebar folds under a row of its own. */
  archived?: boolean;
  /** For a session that opens in the background, whether its life opened: it tells false once the life failed. */
  ready?: Promise<boolean>;
}
export interface Workspace {
  directory: string;
  name: string;
  collapsed: boolean;
  sessions: SessionEntry[];
}
/** A workspace as workspaces.json keeps it. */
interface SavedWorkspace {
  directory: string;
  name: string;
  collapsed: boolean;
  records: string[];
  /** The records of the sessions that the operator archived. */
  archived: string[];
}
/** The longest record, in bytes, that the list replays to tell whether it holds unfinished work. A replay holds about
 * ten times the size of its record in memory, so a record past it shows as saved until the operator opens it. */
const INSPECT_LIMIT = 16 * 2 ** 20;
/** What the replay of a record told, and the key of the record, its time and size, when it told it. */
interface InspectedState {
  path: string;
  key: string;
  pending: number;
  error?: string;
  reading?: boolean;
}
function readInspected(path: string): Map<string, InspectedState> {
  try {
    const list: unknown = JSON.parse(readFileSync(path, "utf8"));
    return new Map(
      (Array.isArray(list) ? list : [])
        .filter((one) => one && typeof one.path === "string" && typeof one.key === "string")
        .map((one) => [one.path, one as InspectedState]),
    );
  } catch {
    return new Map();
  }
}

/** One change of the saved list: a workspace it names, made when the list has none, and what changes in it. */
interface ListChange {
  directory: string;
  collapsed?: boolean;
  add?: string;
  remove?: string;
  archive?: string;
  restore?: string;
  /** The name that the operator gave the workspace. */
  name?: string;
  /** Whether the workspace leaves the list. */
  drop?: boolean;
}

/** The workspaces that the file at a path lists, and none when there is no file. A file that holds no list is
 * refused. */
function readList(path: string): SavedWorkspace[] {
  let data: { workspaces?: unknown } | null;
  try {
    data = JSON.parse(readFileSync(path, "utf8"));
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return [];
    throw error;
  }
  if (!Array.isArray(data?.workspaces)) throw new Error(`${path} holds no list of workspaces.`);
  const listed: SavedWorkspace[] = data.workspaces
    .filter((item) => item && typeof item.directory === "string")
    .map((item) => ({
      directory: item.directory,
      name: typeof item.name === "string" ? item.name : basename(item.directory),
      collapsed: item.collapsed === true,
      records: Array.isArray(item.records)
        ? item.records.filter((path: unknown): path is string => typeof path === "string")
        : [],
      archived: Array.isArray(item.archived)
        ? item.archived.filter((path: unknown): path is string => typeof path === "string")
        : [],
    }));
  return grouped(listed);
}

/** The list with each worktree folded into the workspace of its main checkout, as a list of an earlier version may
 * hold it as a workspace of its own: its records and its archived records go there, and a name that the operator
 * gave stays. */
function grouped(list: SavedWorkspace[]): SavedWorkspace[] {
  const merged = new Map<string, SavedWorkspace>();
  for (const item of list) {
    const directory = mainCheckout(item.directory);
    const named = item.name !== basename(item.directory);
    const seen = merged.get(directory);
    if (!seen) {
      merged.set(directory, { ...item, directory, name: named ? item.name : basename(directory) });
      continue;
    }
    if (named && seen.name === basename(directory)) seen.name = item.name;
    seen.collapsed &&= item.collapsed;
    seen.records.push(...item.records.filter((record) => !seen.records.includes(record)));
    seen.archived.push(...item.archived.filter((record) => !seen.archived.includes(record)));
  }
  return [...merged.values()];
}

export class Workspaces extends EventEmitter {
  readonly groups: Workspace[] = [];
  current?: SessionEntry;
  notice = "";
  private readonly opening = new Map<string, Promise<SessionEntry>>();
  private readonly subscriptions = new Map<Session, () => void>();
  private readonly inspected = new Map<string, string>();
  /** What the replays of saved records told, by record, which a file beside the list keeps from one run to the next. */
  private remembered?: Map<string, InspectedState>;
  private get inspectedPath(): string {
    return join(dirname(this.path), "inspected.json");
  }
  /** Ends the inspections of saved records that are still at work when the workspaces close. */
  private readonly stop = new AbortController();
  private selection = 0;
  private closed = false;
  /** Whether the list file could not be read, so that this run leaves it as it is. */
  private damaged = false;
  /** The directories of the sessions that the list shows while they run and never saves, as the demo that the
   * operator watches, whose directory goes when the process ends. */
  private readonly passing = new Set<string>();
  readonly path: string;
  constructor(
    readonly preferences = new Preferences(),
    readonly options: EngineOptions = {},
    path = join(dirname(preferences.path), "workspaces.json"),
  ) {
    super();
    this.path = path;
    try {
      for (const item of readList(path))
        this.groups.push({
          directory: item.directory,
          name: item.name,
          collapsed: item.collapsed,
          sessions: item.records.map((record) => ({
            path: record,
            name: basename(record, ".jsonl"),
            status: "saved",
            unread: false,
            archived: item.archived.includes(record),
          })),
        });
    } catch (error) {
      this.refuseList(error);
    }
  }
  private refuseList(error: unknown): void {
    this.damaged = true;
    const reason = error instanceof Error ? error.message : String(error);
    this.notice = `Could not read the workspace list at ${this.path}: ${reason.replace(/\.?$/, ".")} Repair or remove it; this run does not change it.`;
  }
  /** Apply a change to the list as the file holds it now, so that what another instance saved stays. */
  private save(change: ListChange): void {
    if (this.passing.has(change.directory)) return;
    let list: SavedWorkspace[];
    try {
      list = readList(this.path);
    } catch (error) {
      this.refuseList(error);
      return;
    }
    const index = list.findIndex((group) => group.directory === change.directory);
    if (change.drop) {
      if (index >= 0) list.splice(index, 1);
    } else {
      const group = list[index] ?? {
        directory: change.directory,
        name: basename(change.directory),
        collapsed: false,
        records: [],
        archived: [],
      };
      if (index < 0) list.push(group);
      if (change.name !== undefined) group.name = change.name;
      if (change.collapsed !== undefined) group.collapsed = change.collapsed;
      if (change.add && !group.records.includes(change.add)) group.records.unshift(change.add);
      if (change.remove) {
        group.records = group.records.filter((record) => record !== change.remove);
        group.archived = group.archived.filter((record) => record !== change.remove);
      }
      if (change.archive && !group.archived.includes(change.archive)) group.archived.push(change.archive);
      if (change.restore) group.archived = group.archived.filter((record) => record !== change.restore);
    }
    mkdirSync(dirname(this.path), { recursive: true });
    saveFile(this.path, JSON.stringify({ workspaces: list }));
    if (this.damaged) {
      this.damaged = false;
      this.notice = "";
    }
  }
  groupOf(entry = this.current): Workspace | undefined {
    return this.groups.find((group) => group.sessions.includes(entry as SessionEntry));
  }
  groupStatus(group: Workspace): SessionStatus {
    const order: SessionStatus[] = [
      "blocked",
      "error",
      "reading",
      "paused",
      "working",
      "done",
      "opening",
      "idle",
      "saved",
    ];
    return order.find((status) => group.sessions.some((entry) => entry.status === status)) ?? "idle";
  }
  /** Add a workspace to the list. Its records, which the list saved, stand in it at once, and the scan of its folder,
   * which finds the others and tells their size, their cost and their state, waits for the next refresh when scan is
   * false, so a window draws before it reads any record. */
  async add(directory: string, scan = true): Promise<Workspace> {
    const opened = await realpath(resolve(expandHome(directory)));
    if (!(await stat(opened)).isDirectory()) throw new Error("A workspace must be a project folder.");
    // A worktree belongs to the workspace of its main checkout.
    const path = mainCheckout(opened);
    let group = this.groups.find((group) => group.directory === path);
    if (!group) {
      group = { directory: path, name: basename(path), collapsed: false, sessions: [] };
      this.groups.push(group);
    }
    if (scan) await this.scan(group);
    this.save({ directory: group.directory });
    this.emit("change");
    return group;
  }
  async scan(group: Workspace): Promise<void> {
    // The sessions of the main checkout and of each of its worktrees are the sessions of one workspace.
    const found: { name: string; path: string }[] = [];
    for (const checkout of checkouts(group.directory)) {
      const directory = join(checkout, ".furb/sessions");
      const names = await readdir(directory).catch((error: NodeJS.ErrnoException) => {
        if (error.code === "ENOENT") return [] as string[];
        throw error;
      });
      for (const name of names)
        if (name.endsWith(".jsonl") && !name.endsWith(".changes.jsonl"))
          found.push({ name, path: join(directory, name) });
    }
    found.sort((a, b) => (a.name < b.name ? 1 : a.name > b.name ? -1 : 0));
    for (const { name, path } of found)
      if (!group.sessions.some((entry) => entry.path === path))
        group.sessions.push({ path, name: basename(name, ".jsonl"), status: "saved", unread: false });
    // A session that opens stands in the list before its record does.
    const saved = (entry: SessionEntry) => !entry.session && !this.opening.has(entry.path);
    group.sessions = group.sessions.filter((entry) => !saved(entry) || existsSync(entry.path));
    await Promise.all(
      group.sessions.map(async (entry) => {
        if (entry.session) entry.cost = entry.session.cost;
        if (!saved(entry)) return;
        const info = await stat(entry.path);
        entry.modified = info.mtimeMs;
        entry.size = info.size;
        const { view } = savedView(entry.path);
        if (typeof view.sessionName === "string") entry.name = view.sessionName;
        entry.cost = typeof view.cost === "number" ? view.cost : 0;
      }),
    );
    const changed: string[] = [];
    for (const entry of group.sessions.filter(saved)) {
      const key = `${entry.modified}:${entry.size}`;
      if (this.inspected.get(entry.path) === key) continue;
      this.inspected.set(entry.path, key);
      // A record that no replay read since it last changed, and that is small enough, is replayed. A record that a
      // replay read keeps what that replay told, and a long or archived one stays as saved until the operator opens it.
      const told = this.told(entry.path, key);
      if (told) this.apply(entry, told);
      else if (!entry.archived && (entry.size ?? 0) <= INSPECT_LIMIT) changed.push(entry.path);
    }
    if (changed.length) this.inspect(changed);
  }
  /** What the replay of a record told when the record stood as the key says, from this run or from an earlier one. */
  private told(path: string, key: string): InspectedState | undefined {
    this.remembered ??= readInspected(this.inspectedPath);
    const found = this.remembered.get(path);
    return found?.key === key ? found : undefined;
  }
  /** The state of a saved row that a replay told, unless the session of the row runs. */
  private apply(entry: SessionEntry, state: Pick<InspectedState, "pending" | "error" | "reading">): void {
    if (entry.session) return;
    // A record that a build of another version made drifts, and opens to read: that is no error.
    entry.error = state.reading ? undefined : state.error;
    entry.status = state.reading ? "reading" : state.error ? "error" : state.pending ? "paused" : "saved";
  }
  /** Keep what the replays told in a file beside the list, so that the next run replays only the records that changed
   * since. */
  private remember(told: InspectedState[]): void {
    const kept = readInspected(this.inspectedPath);
    for (const state of told) kept.set(state.path, state);
    for (const path of kept.keys()) if (!existsSync(path)) kept.delete(path);
    this.remembered = kept;
    try {
      mkdirSync(dirname(this.inspectedPath), { recursive: true });
      saveFile(this.inspectedPath, JSON.stringify([...kept.values()]));
    } catch {}
  }
  /** Find the unfinished work of saved records by their replay, which takes a time that grows with each record, so
   * the rows show their state when it comes and nothing waits for it. */
  private inspect(paths: string[]): void {
    // A replay tells a record as it was when the replay began, so a row whose record changed or opened since then keeps
    // what that change or that open told it.
    const keys = new Map(paths.map((path) => [path, this.inspected.get(path)]));
    inspectRecords(paths, this.stop.signal).then(
      (states) => {
        this.remember(
          paths.flatMap((path) => {
            const key = keys.get(path);
            const state = states[path];
            return key && state && this.inspected.get(path) === key ? [{ path, key, ...state }] : [];
          }),
        );
        for (const group of this.groups)
          for (const entry of group.sessions) {
            const state = states[entry.path];
            if (!state || this.inspected.get(entry.path) !== keys.get(entry.path)) continue;
            this.apply(entry, state);
          }
        this.emit("change");
      },
      (error: unknown) => {
        if (this.closed) return;
        for (const path of paths) this.inspected.delete(path);
        this.notice = String(error);
        this.emit("change");
      },
    );
  }
  async refresh(): Promise<void> {
    await Promise.all(
      this.groups.map(async (group) => {
        try {
          await this.scan(group);
        } catch (error) {
          this.notice = String(error);
        }
      }),
    );
    this.emit("change");
  }
  adopt(session: Session, group: Workspace): SessionEntry {
    const path = session.host.record;
    if (!path) throw new Error("A workspace session needs a record.");
    let entry = group.sessions.find((entry) => entry.path === path);
    if (!entry) {
      entry = { path, name: session.sessionName, status: "idle", unread: false };
      group.sessions.unshift(entry);
    }
    if (entry.session && entry.session !== session) throw new Error("This session is already open.");
    entry.session = session;
    const row = entry;
    let completed = session.host.completed;
    const changed = () => {
      const status = session.status();
      const next = session.host.completed;
      if (row !== this.current && next > completed) row.unread = true;
      completed = next;
      if (row === this.current) row.unread = false;
      row.status = row.unread && status === "idle" ? "done" : status;
      row.name = session.sessionName;
      row.cost = session.cost;
      this.emit("change");
    };
    this.subscriptions.set(session, changed);
    session.on("change", changed);
    session.on("reopen", (command: string) => void this.reopen(row, command).catch(session.fail));
    changed();
    this.save({ directory: group.directory, add: path });
    return row;
  }
  /** A session that opens in the background: its view stands at once, on a life that is not open yet, and the life
   * fills it in when it opens. A life that fails to open leaves the row with its error. */
  private lazily(entry: SessionEntry, group: Workspace): SessionEntry {
    this.inspected.delete(entry.path);
    entry.error = undefined;
    const { view } = savedView(entry.path);
    const demo = view.demo ?? this.options.demo ?? false;
    const named = { ...this.options, demo };
    const life = openingEngine({
      ...named,
      cwd: checkoutOf(entry.path, group.directory),
      record: entry.path,
      roster: roster(named, view.models),
    });
    const session = new Session(life.engine, life.host, demo, this.preferences);
    const row = this.adopt(session, group);
    // The first read of the life waits for its open, and the view shows that it loads meanwhile.
    const first = session.refresh();
    first.catch(() => {});
    entry.ready = life.ready.then(
      async () => {
        await first;
        return !this.closed;
      },
      async (error: unknown) => {
        const listener = this.subscriptions.get(session);
        if (listener) session.off("change", listener);
        this.subscriptions.delete(session);
        if (entry.session === session) entry.session = undefined;
        if (this.closed) return false;
        entry.status = "error";
        entry.error = error instanceof Error ? error.message : String(error);
        session.fail(error);
        this.save({ directory: group.directory, remove: entry.path });
        await session.dispose().catch(() => {});
        this.emit("failed", entry, error);
        this.emit("change");
        return false;
      },
    );
    return row;
  }
  private async load(entry: SessionEntry, group: Workspace, lazy = false): Promise<SessionEntry> {
    if (entry.session) return entry;
    const pending = this.opening.get(entry.path);
    if (pending) return pending;
    if (lazy) return this.lazily(entry, group);
    const opening = (async () => {
      this.inspected.delete(entry.path);
      entry.status = "opening";
      entry.error = undefined;
      this.emit("change");
      let session: Session | undefined;
      let opened: Awaited<ReturnType<typeof openEngine>> | undefined;
      try {
        const { view } = savedView(entry.path);
        const demo = view.demo ?? this.options.demo ?? false;
        const named = { ...this.options, demo };
        opened = await openEngine({
          ...named,
          cwd: checkoutOf(entry.path, group.directory),
          record: entry.path,
          roster: roster(named, view.models),
        });
        session = new Session(opened.engine, opened.host, demo, this.preferences);
        await session.refresh();
        if (this.closed) throw new Error("The workspace is closed.");
        return this.adopt(session, group);
      } catch (error) {
        if (session) await session.dispose();
        else await opened?.host.dispose();
        entry.status = "error";
        entry.error = error instanceof Error ? error.message : String(error);
        this.emit("change");
        throw error;
      } finally {
        this.opening.delete(entry.path);
      }
    })();
    this.opening.set(entry.path, opening);
    return opening;
  }
  /** Show a session that is open already, in a workspace of its directory that the list holds while this process
   * runs and never saves, and select it. */
  async show(session: Session): Promise<void> {
    const directory = await realpath(resolve(session.host.directory));
    this.passing.add(mainCheckout(directory));
    const group = await this.add(directory);
    await this.select(this.adopt(session, group));
  }
  async select(entry: SessionEntry): Promise<void> {
    const group = this.groupOf(entry);
    if (!group) throw new Error("This session has no workspace.");
    if (entry.archived) this.restore(entry);
    const selection = ++this.selection;
    await this.load(entry, group);
    // An archive, a delete or a removal of the workspace that came while the session opened closed it again, and the
    // selection gives way to it.
    if (selection !== this.selection || this.closed) return;
    if (!entry.session || entry.archived || this.groupOf(entry) !== group) return;
    this.current = entry;
    entry.unread = false;
    entry.status = entry.session.status();
    group.collapsed = false;
    this.save({ directory: group.directory, collapsed: false });
    this.emit("select", entry.session);
    this.emit("change");
  }
  /** Open a session again on its record, as a later life whose roster holds the models the operator added, which
   * starts again the work that the life before it left, and then runs a command of the operator in it. */
  async reopen(entry: SessionEntry, command: string): Promise<void> {
    await this.release(entry);
    await this.select(entry);
    const session = entry.session;
    if (!session) return;
    if (session.host.pending.size) await session.host.resume();
    await session.command(command);
  }
  /** A new session. It selects after its life opens, unless lazy is set: the session then stands and selects at once,
   * on a life that opens in the background and fills it in, and a life that fails to open tells so with the event
   * failed. Its record stands in the .furb/sessions of `checkout`, the main checkout of the workspace or one of its
   * worktrees, where it runs. */
  async create(
    group = this.groupOf(),
    name?: string,
    carry?: Carry,
    resume = false,
    checkout = group?.directory,
    lazy = false,
  ): Promise<SessionEntry> {
    if (!group || !checkout) throw new Error("Add a workspace first.");
    const directory = furbDirectory(checkout, "sessions");
    const path = join(
      directory,
      `${new Date().toISOString().replaceAll(":", "-")}-${randomUUID().slice(0, 8)}.jsonl`,
    );
    const label = name ?? `Session ${group.sessions.length + 1}`;
    const entry: SessionEntry = { path, name: label, status: "opening", unread: false };
    group.sessions.unshift(entry);
    await this.load(entry, group, lazy);
    // A life that starts from a record opens with the earlier work of that record in its first word, or resumes the
    // record: it goes on from the turns of its root chain and asks again its open prompts.
    if (carry && entry.session) {
      if (entry.ready && !(await entry.ready)) throw new Error(entry.error ?? "The session did not open.");
      const { engine } = entry.session;
      // The resume is the word of the compaction extension, so a life that runs none carries the text instead.
      const goes =
        resume && enabled(await engine.transcript({ on: ROOT })).some((one) => one.name === "compaction");
      await engine.result(await engine.rung({ word: goes ? carry.resume : carry.word, on: engine.root }));
      if (resume)
        entry.session.notice = goes
          ? "Resumed from the turns of the record."
          : "The extension compaction is off, so the earlier work came as text.";
    }
    if (entry.session) {
      entry.session.sessionName = label;
      entry.name = label;
      entry.session.save();
    }
    await this.select(entry);
    return entry;
  }
  async import(path: string, group: Workspace): Promise<SessionEntry> {
    const requested = resolve(path);
    await mkdir(dirname(requested), { recursive: true });
    const record = existsSync(requested)
      ? await realpath(requested)
      : join(await realpath(dirname(requested)), basename(requested));
    let entry = group.sessions.find((entry) => entry.path === record);
    if (!entry) {
      entry = { path: record, name: basename(record, ".jsonl"), status: "saved", unread: false };
      group.sessions.unshift(entry);
    }
    await this.select(entry);
    return entry;
  }
  toggle(group?: Workspace): void {
    if (group) {
      group.collapsed = !group.collapsed;
      this.save({ directory: group.directory, collapsed: group.collapsed });
    } else {
      this.preferences.sidebar = !this.preferences.sidebar;
      this.preferences.save();
    }
    this.emit("change");
  }
  /** Archive a session: it leaves the list of its workspace for a row of its own, which the sidebar folds, and its
   * record stays. The current session moves to another session first, as it does for a delete, and an open session
   * closes. */
  async archive(entry: SessionEntry): Promise<void> {
    await this.opening.get(entry.path);
    const group = this.groupOf(entry);
    if (!group) throw new Error("This session has no workspace.");
    await this.leave(entry, group);
    if (await this.release(entry)) entry.status = "saved";
    entry.archived = true;
    this.save({ directory: group.directory, add: entry.path, archive: entry.path });
    this.emit("change");
  }
  /** When a session is the current one, the current session moves to the first other session of its workspace that
   * opens, and to a new one when none does. A session that does not open keeps its error on its row. */
  private async leave(entry: SessionEntry, group: Workspace): Promise<void> {
    for (const next of group.sessions.filter((other) => other !== entry && !other.archived)) {
      if (this.current !== entry) break;
      await this.select(next).catch(() => {});
    }
    if (this.current === entry)
      await this.create(group, undefined, undefined, false, checkoutOf(entry.path, group.directory));
  }
  /** End the life of the open session of an entry, and tell whether it was open. */
  private async release(entry: SessionEntry): Promise<boolean> {
    const session = entry.session;
    if (!session) return false;
    const listener = this.subscriptions.get(session);
    if (listener) session.off("change", listener);
    this.subscriptions.delete(session);
    entry.session = undefined;
    await session.dispose(true);
    return true;
  }
  /** Give a session or a workspace the name that the operator chose. The saved view of a session keeps its name,
   * whether the session is open or not, and the list keeps the name of a workspace. */
  rename(item: SessionEntry | Workspace, name: string): void {
    item.name = name;
    if ("sessions" in item) this.save({ directory: item.directory, name });
    else if (item.session) {
      item.session.sessionName = name;
      item.session.save();
    } else
      saveFile(`${item.path}.ui.json`, JSON.stringify({ ...savedView(item.path).view, sessionName: name }));
    this.emit("change");
  }
  /** Take a workspace off the list. Its folder and its records stay, and /workspace adds it back. The current
   * session moves to a session of another workspace first, and the open sessions of the workspace close. */
  async remove(group: Workspace): Promise<void> {
    if (!this.groups.includes(group)) return;
    if (group === this.groupOf()) {
      const other = this.groups.find((one) => one !== group);
      if (!other) throw new Error("This is the only workspace. Add another one before you remove it.");
      const next = other.sessions.find((entry) => !entry.archived);
      await (next ? this.select(next).catch(() => this.create(other)) : this.create(other));
    }
    for (const entry of group.sessions) {
      await this.opening.get(entry.path)?.catch(() => {});
      await this.release(entry);
    }
    // A second removal of the same workspace that ran meanwhile took it off already.
    const index = this.groups.indexOf(group);
    if (index < 0) return;
    this.groups.splice(index, 1);
    this.save({ directory: group.directory, drop: true });
    this.emit("change");
  }
  /** Bring an archived session back to the list of its workspace. */
  restore(entry: SessionEntry): void {
    const group = this.groupOf(entry);
    if (!group) throw new Error("This session has no workspace.");
    entry.archived = false;
    this.save({ directory: group.directory, restore: entry.path });
    this.emit("change");
  }
  async delete(entry: SessionEntry): Promise<string> {
    await this.opening.get(entry.path);
    const group = this.groupOf(entry);
    if (!group) throw new Error("This session has no workspace.");
    await this.leave(entry, group);
    await this.release(entry);
    // The record and its files move to the trash while this store holds the lease, so no other process opens them
    // meanwhile. The lock file stays at its path, empty: on macOS a lookup of a renamed path can go on giving the
    // moved file for seconds, so the lease holds only while no holder moves or removes the lock file. The scratch
    // directory of the record goes from the cache of furb, since a life on the record that is restored makes again
    // the files that its writes made.
    const lease = store(entry.path)[1];
    const scratchDirectory = scratch(entry.path);
    const moved: [string, string][] = [];
    try {
      const directory = furbDirectory(group.directory, "trash", randomUUID());
      for (const suffix of ["", ".ui.json", ".session.json", ".changes.jsonl", ".images", ".told"]) {
        const source = `${entry.path}${suffix}`;
        if (!existsSync(source)) continue;
        const target = join(directory, basename(source));
        await rename(source, target);
        moved.push([source, target]);
      }
      await writeFile(
        join(directory, "restore.json"),
        JSON.stringify({ directory: group.directory, name: entry.name, files: moved }),
        { mode: 0o600 },
      );
      group.sessions = group.sessions.filter((candidate) => candidate !== entry);
      this.save({ directory: group.directory, remove: entry.path });
      this.emit("change");
      // The delete is done: the scratch directory moves aside before it goes, so a remove that stops part of the way
      // leaves no directory in its place, and a failure here undoes nothing.
      if (scratchDirectory) {
        const aside = `${scratchDirectory}.gone-${randomUUID()}`;
        await rename(scratchDirectory, aside)
          .then(() => rm(aside, { recursive: true, force: true }))
          .catch(() => {});
      }
      if (this.current?.session) this.current.session.notice = `Session moved to ${directory}`;
      return directory;
    } catch (error) {
      for (const [source, target] of moved.reverse()) await rename(target, source);
      throw error;
    } finally {
      lease.dispose();
    }
  }
  /** Close the workspaces: the window detaches from each life, which goes on when it runs in a host of its own, or
   * each life ends when end is set. */
  async dispose(end = false): Promise<void> {
    this.closed = true;
    this.stop.abort(new Error("The workspaces are closed."));
    await Promise.allSettled([...this.opening.values()]);
    for (const [session, changed] of this.subscriptions) session.off("change", changed);
    await Promise.all([...this.subscriptions.keys()].map((session) => session.dispose(end)));
    this.subscriptions.clear();
    this.removeAllListeners();
  }
}
