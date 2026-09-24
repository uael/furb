import { randomUUID } from "node:crypto";
import { EventEmitter } from "node:events";
import { existsSync, mkdirSync, readFileSync } from "node:fs";
import { mkdir, readdir, realpath, rename, stat, writeFile } from "node:fs/promises";
import { basename, dirname, join, resolve } from "node:path";
import { furbDirectory, RecordLock, saveFile } from "@furb/engine";
import { openEngine } from "./bridge.ts";
import { expandHome } from "./files.ts";
import type { EngineOptions } from "./models.ts";
import { Preferences } from "./preferences.ts";
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
  return data.workspaces
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
}

export class Workspaces extends EventEmitter {
  readonly groups: Workspace[] = [];
  current?: SessionEntry;
  notice = "";
  private readonly opening = new Map<string, Promise<SessionEntry>>();
  private readonly subscriptions = new Map<Session, () => void>();
  private readonly inspected = new Map<string, string>();
  /** Ends the inspections of saved records that are still at work when the workspaces close. */
  private readonly stop = new AbortController();
  private selection = 0;
  private closed = false;
  /** Whether the list file could not be read, so that this run leaves it as it is. */
  private damaged = false;
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
      "paused",
      "working",
      "done",
      "opening",
      "idle",
      "saved",
    ];
    return order.find((status) => group.sessions.some((entry) => entry.status === status)) ?? "idle";
  }
  async add(directory: string): Promise<Workspace> {
    const path = await realpath(resolve(expandHome(directory)));
    if (!(await stat(path)).isDirectory()) throw new Error("A workspace must be a project folder.");
    let group = this.groups.find((group) => group.directory === path);
    if (!group) {
      group = { directory: path, name: basename(path), collapsed: false, sessions: [] };
      this.groups.push(group);
    }
    await this.scan(group);
    this.save({ directory: group.directory });
    this.emit("change");
    return group;
  }
  async scan(group: Workspace): Promise<void> {
    const directory = join(group.directory, ".furb/sessions");
    const names = await readdir(directory).catch((error: NodeJS.ErrnoException) => {
      if (error.code === "ENOENT") return [] as string[];
      throw error;
    });
    for (const name of names.sort().reverse()) {
      if (!name.endsWith(".jsonl") || name.endsWith(".changes.jsonl")) continue;
      const path = join(directory, name);
      if (!group.sessions.some((entry) => entry.path === path))
        group.sessions.push({ path, name: basename(name, ".jsonl"), status: "saved", unread: false });
    }
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
      if (this.inspected.get(entry.path) !== key) {
        this.inspected.set(entry.path, key);
        changed.push(entry.path);
      }
    }
    this.inspect(changed);
  }
  /** Find the unfinished work of saved records by their replay, which takes a time that grows with each record, so
   * the rows show their state when it comes and nothing waits for it. */
  private inspect(paths: string[]): void {
    inspectRecords(paths, this.stop.signal).then(
      (states) => {
        for (const group of this.groups)
          for (const entry of group.sessions) {
            const state = states[entry.path];
            if (!state || entry.session || this.opening.has(entry.path)) continue;
            entry.error = state.error;
            entry.status = state.error ? "error" : state.pending ? "paused" : "saved";
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
    const path = session.world.records.path;
    if (!path) throw new Error("A workspace session needs a record.");
    let entry = group.sessions.find((entry) => entry.path === path);
    if (!entry) {
      entry = { path, name: session.sessionName, status: "idle", unread: false };
      group.sessions.unshift(entry);
    }
    if (entry.session && entry.session !== session) throw new Error("This session is already open.");
    entry.session = session;
    const row = entry;
    let completed = session.world.completed;
    const changed = () => {
      const status = session.status();
      const next = session.world.completed;
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
    changed();
    this.save({ directory: group.directory, add: path });
    return row;
  }
  private async load(entry: SessionEntry, group: Workspace): Promise<SessionEntry> {
    if (entry.session) return entry;
    const pending = this.opening.get(entry.path);
    if (pending) return pending;
    const opening = (async () => {
      entry.status = "opening";
      entry.error = undefined;
      this.emit("change");
      let session: Session | undefined;
      let opened: Awaited<ReturnType<typeof openEngine>> | undefined;
      try {
        const demo = savedView(entry.path).view.demo ?? this.options.demo ?? false;
        opened = await openEngine({ ...this.options, cwd: group.directory, record: entry.path, demo });
        const { life, world } = opened;
        session = new Session(life, world, demo, this.preferences);
        await session.refresh();
        if (this.closed) throw new Error("The workspace is closed.");
        return this.adopt(session, group);
      } catch (error) {
        if (session) await session.dispose();
        else await opened?.world.dispose();
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
  async create(group = this.groupOf(), name?: string): Promise<SessionEntry> {
    if (!group) throw new Error("Add a workspace first.");
    const directory = furbDirectory(group.directory, "sessions");
    const path = join(
      directory,
      `${new Date().toISOString().replaceAll(":", "-")}-${randomUUID().slice(0, 8)}.jsonl`,
    );
    const label = name ?? `Session ${group.sessions.length + 1}`;
    const entry: SessionEntry = { path, name: label, status: "opening", unread: false };
    group.sessions.unshift(entry);
    await this.load(entry, group);
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
    for (const next of group.sessions.filter((other) => other !== entry && !other.archived)) {
      if (this.current !== entry) break;
      await this.select(next).catch(() => {});
    }
    if (this.current === entry) await this.create(group);
    if (await this.release(entry)) entry.status = "saved";
    entry.archived = true;
    this.save({ directory: group.directory, add: entry.path, archive: entry.path });
    this.emit("change");
  }
  /** Close the open session of an entry, and tell whether it was open. */
  private async release(entry: SessionEntry): Promise<boolean> {
    const session = entry.session;
    if (!session) return false;
    const listener = this.subscriptions.get(session);
    if (listener) session.off("change", listener);
    this.subscriptions.delete(session);
    entry.session = undefined;
    await session.dispose();
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
    // The current session moves to the first other session that opens, and to a new one when none does. A session
    // that does not open keeps its error on its row.
    for (const next of group.sessions.filter((other) => other !== entry && !other.archived)) {
      if (this.current !== entry) break;
      await this.select(next).catch(() => {});
    }
    if (this.current === entry) await this.create(group);
    await this.release(entry);
    // The lock file moves with the record while this lease holds it, and a process that locked it meanwhile opens
    // the path again.
    const lease = new RecordLock(entry.path);
    const moved: [string, string][] = [];
    try {
      const directory = furbDirectory(group.directory, "trash", randomUUID());
      for (const suffix of ["", ".ui.json", ".world.json", ".changes.jsonl", ".images", ".lock"]) {
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
      if (this.current?.session) this.current.session.notice = `Session moved to ${directory}`;
      return directory;
    } catch (error) {
      for (const [source, target] of moved.reverse()) await rename(target, source);
      throw error;
    } finally {
      lease.dispose();
    }
  }
  async dispose(): Promise<void> {
    this.closed = true;
    this.stop.abort(new Error("The workspaces are closed."));
    await Promise.allSettled([...this.opening.values()]);
    for (const [session, changed] of this.subscriptions) session.off("change", changed);
    await Promise.all([...this.subscriptions.keys()].map((session) => session.dispose()));
    this.subscriptions.clear();
    this.removeAllListeners();
  }
}
