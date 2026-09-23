import { randomUUID } from "node:crypto";
import { EventEmitter } from "node:events";
import { existsSync, mkdirSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { mkdir, readdir, readFile, realpath, rename, stat, writeFile } from "node:fs/promises";
import { homedir } from "node:os";
import { basename, dirname, join, resolve } from "node:path";
import { RecordLock, type WorldOptions } from "@furb/engine";
import { openEngine } from "./bridge.ts";
import { Preferences } from "./preferences.ts";
import { inspectRecords } from "./records.ts";
import { Session } from "./session.ts";

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
export interface SessionEntry {
  path: string;
  name: string;
  session?: Session;
  status: SessionStatus;
  unread: boolean;
  error?: string;
  held?: number;
}
export interface Workspace {
  directory: string;
  name: string;
  collapsed: boolean;
  sessions: SessionEntry[];
}

/** Status comes from live acts. A saved record is not presented as a running session. */
export function activityStatus(session: Session): SessionStatus {
  if (Object.values(session.errors).some(Boolean)) return "error";
  if (session.queueHeld && session.queued.length) return "blocked";
  if (session.world.held.size) return "paused";
  if (session.world.prompts.size) return "blocked";
  const acts = session.acts.filter((act) => !["chain", "grant"].includes(act.kind));
  const pending = acts.filter((act) => !act.done);
  if (pending.some((act) => !act.paused)) return "working";
  if (pending.length || session.paused) return "paused";
  const latest = acts.at(-1);
  if (latest?.run?.status === "failed") return "error";
  const last = latest?.value;
  if (last && typeof last === "object" && "is" in last && "args" in last && last.is !== "CancelledError")
    return "error";
  return "idle";
}

export class Workspaces extends EventEmitter {
  readonly groups: Workspace[] = [];
  current?: SessionEntry;
  notice = "";
  private readonly opening = new Map<string, Promise<SessionEntry>>();
  private readonly subscriptions = new Map<Session, () => void>();
  private readonly inspected = new Map<string, string>();
  private selection = 0;
  private closed = false;
  readonly path: string;
  constructor(
    readonly preferences = new Preferences(),
    readonly options: WorldOptions & { demo?: boolean } = {},
    path = join(dirname(preferences.path), "workspaces.json"),
  ) {
    super();
    this.path = path;
    try {
      const data = JSON.parse(readFileSync(this.path, "utf8")) as { workspaces?: unknown };
      if (Array.isArray(data?.workspaces))
        for (const item of data.workspaces) {
          if (!item || typeof item.directory !== "string") continue;
          this.groups.push({
            directory: item.directory,
            name: typeof item.name === "string" ? item.name : basename(item.directory),
            collapsed: item.collapsed === true,
            sessions: Array.isArray(item.records)
              ? item.records
                  .filter((path: unknown): path is string => typeof path === "string")
                  .map((path: string) => ({
                    path,
                    name: basename(path, ".jsonl"),
                    status: "saved",
                    unread: false,
                  }))
              : [],
          });
        }
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== "ENOENT")
        this.notice = "Could not read the workspace list. Add a project folder to restore it.";
    }
  }
  save(): void {
    mkdirSync(dirname(this.path), { recursive: true });
    writeFileSync(
      `${this.path}.tmp`,
      JSON.stringify({
        workspaces: this.groups.map((group) => ({
          directory: group.directory,
          name: group.name,
          collapsed: group.collapsed,
          records: group.sessions.map((session) => session.path),
        })),
      }),
      { mode: 0o600 },
    );
    renameSync(`${this.path}.tmp`, this.path);
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
    const expanded =
      directory === "~"
        ? homedir()
        : directory.startsWith("~/")
          ? join(homedir(), directory.slice(2))
          : directory;
    const path = await realpath(resolve(expanded));
    if (!(await stat(path)).isDirectory()) throw new Error("A workspace must be a project folder.");
    let group = this.groups.find((group) => group.directory === path);
    if (!group) {
      group = { directory: path, name: basename(path), collapsed: false, sessions: [] };
      this.groups.push(group);
    }
    await this.scan(group);
    this.save();
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
    group.sessions = group.sessions.filter((entry) => entry.session || existsSync(entry.path));
    await Promise.all(
      group.sessions
        .filter((entry) => !entry.session)
        .map(async (entry) => {
          const metadata = await readFile(`${entry.path}.ui.json`, "utf8")
            .then((text) => JSON.parse(text))
            .catch(() => ({}));
          if (typeof metadata?.sessionName === "string") entry.name = metadata.sessionName;
        }),
    );
    const changed: string[] = [];
    for (const entry of group.sessions.filter((entry) => !entry.session)) {
      const info = await stat(entry.path);
      const key = `${info.mtimeMs}:${info.size}`;
      if (this.inspected.get(entry.path) !== key) {
        this.inspected.set(entry.path, key);
        changed.push(entry.path);
      }
    }
    const states = await inspectRecords(changed);
    for (const entry of group.sessions) {
      const state = states[entry.path];
      if (!state || entry.session) continue;
      entry.held = state.held;
      entry.error = state.error;
      entry.status = state.error ? "error" : state.held ? "paused" : "saved";
    }
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
    let completed = session.world.facts.filter(
      ([kind, id]) => kind === "done" && /^(prompt|rung|bash|wait):/.test(id),
    ).length;
    const changed = () => {
      const status = activityStatus(session);
      const next = session.world.facts.filter(
        ([kind, id]) => kind === "done" && /^(prompt|rung|bash|wait):/.test(id),
      ).length;
      if (row !== this.current && next > completed) row.unread = true;
      completed = next;
      if (row === this.current) row.unread = false;
      row.status = row.unread && status === "idle" ? "done" : status;
      row.name = session.sessionName;
      this.emit("change");
    };
    this.subscriptions.set(session, changed);
    session.on("change", changed);
    changed();
    this.save();
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
        const metadata = await readFile(`${entry.path}.ui.json`, "utf8")
          .then((text) => JSON.parse(text))
          .catch(() => ({}));
        opened = await openEngine({
          ...this.options,
          cwd: group.directory,
          record: entry.path,
          demo: metadata.demo ?? this.options.demo,
        });
        const { life, world } = opened;
        session = new Session(life, world, metadata.demo ?? this.options.demo ?? false, this.preferences);
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
    const selection = ++this.selection;
    await this.load(entry, group);
    if (selection !== this.selection || this.closed) return;
    this.current = entry;
    entry.unread = false;
    entry.status = activityStatus(entry.session as Session);
    group.collapsed = false;
    this.save();
    this.emit("select", entry.session);
    this.emit("change");
  }
  async create(group = this.groupOf(), name?: string): Promise<SessionEntry> {
    if (!group) throw new Error("Add a workspace first.");
    const directory = join(group.directory, ".furb/sessions");
    await mkdir(directory, { recursive: true });
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
      this.save();
    } else {
      this.preferences.sidebar = !this.preferences.sidebar;
      this.preferences.save();
    }
    this.emit("change");
  }
  async delete(entry: SessionEntry): Promise<string> {
    await this.opening.get(entry.path);
    const group = this.groupOf(entry);
    if (!group) throw new Error("This session has no workspace.");
    if (this.current === entry) {
      const next = group.sessions.find((next) => next !== entry);
      if (next) await this.select(next);
      else await this.create(group);
    }
    if (entry.session) {
      const listener = this.subscriptions.get(entry.session);
      if (listener) entry.session.off("change", listener);
      this.subscriptions.delete(entry.session);
      await entry.session.dispose();
      entry.session = undefined;
    }
    const lease = new RecordLock(entry.path);
    const directory = join(group.directory, ".furb/trash", randomUUID());
    const moved: [string, string][] = [];
    try {
      await mkdir(directory, { recursive: true });
      for (const suffix of ["", ".ui.json", ".world.json", ".changes.jsonl", ".images"]) {
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
      this.save();
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
    await Promise.allSettled([...this.opening.values()]);
    for (const [session, changed] of this.subscriptions) session.off("change", changed);
    await Promise.all([...this.subscriptions.keys()].map((session) => session.dispose()));
    this.subscriptions.clear();
    this.removeAllListeners();
  }
}
