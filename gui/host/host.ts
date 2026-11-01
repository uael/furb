// The host of a life for the app. It opens the projects and the lives as the TUI opens them, with the Workspaces and
// the Session of the TUI, which drive the Session of bind/typescript in the worker of the TUI, so the app and the TUI
// have one host of a life. It sends each window what changed in the life that it shows, and it does what the window
// calls for, each by the method of the TUI that does it.

import { existsSync, readFileSync } from "node:fs";
import { basename, dirname, resolve } from "node:path";
import { type Line, Parts, type Seen } from "@furb/tui/src/parts.ts";
import type { Cancel, Session } from "@furb/tui/src/session.ts";
import type { Workspace, Workspaces } from "@furb/tui/src/workspaces.ts";
import type { LiveAct, Stream } from "furb";
import type { Life, Mark, Part, Project, Said, StreamPart } from "../src/wire.ts";

/** How many milliseconds at least stand between two parts that the host sends while the life changes. */
const PACE = 50;

/** The project of a record: the directory that its session saved, the project whose .furb/sessions holds it, or
 * else the directory of the record. */
export function projectOf(record: string): string {
  try {
    const saved = JSON.parse(readFileSync(`${record}.session.json`, "utf8"))?.options?.cwd;
    if (typeof saved === "string" && existsSync(saved)) return saved;
  } catch {
    // A record with no saved session, or one that does not read, stands in the project that holds it.
  }
  const folder = dirname(resolve(record));
  return basename(folder) === "sessions" && basename(dirname(folder)) === ".furb"
    ? dirname(dirname(folder))
    : folder;
}

/** What the host holds of a stream that it sent: how much of its text and of its thought, and its retry. */
interface Sent {
  text: number;
  thinking: number;
  retry: string;
}

export class Host {
  private readonly windows = new Set<(said: Said) => void>();
  /** The session that the windows show, and the count of the lives that they showed. */
  private shown?: Session;
  private generation = 0;
  /** What the host last sent of the acts, of the streams, of the life and of the projects. */
  private acts = new Map<string, LiveAct>();
  private streams = new Map<string, Sent>();
  private life = "";
  private projects = "";
  private timer?: ReturnType<typeof setTimeout>;
  private last = 0;
  /** The cancel that waits for the operator to confirm it, since it ends running work. */
  private cancelling?: Cancel;
  /** The parts of the extensions of the project of the life shown, as the TUI loads them, and the marks that they
   * gave last, with what the life held then. */
  private parts?: Parts;
  private marked = { held: "", marks: [] as Mark[] };
  constructor(readonly workspaces: Workspaces) {
    workspaces.on("change", this.changed);
    workspaces.on("select", this.changed);
  }

  /** A window that connects: it takes the whole life at once, then each part that the host sends. */
  connect(send: (said: Said) => void): () => void {
    this.follow();
    this.windows.add(send);
    send({ part: this.whole() });
    return () => this.windows.delete(send);
  }

  /** The session that the workspaces show, followed: a new one is a new generation, which the windows take whole. */
  private follow(): void {
    const session = this.workspaces.current?.session;
    if (session === this.shown) return;
    this.shown?.off("change", this.changed);
    this.shown?.off("confirm", this.confirming);
    this.shown?.off("compose", this.composing);
    this.shown = session;
    this.cancelling = undefined;
    this.parts?.dispose();
    this.parts = session ? new Parts(session.host.directory, this.changed) : undefined;
    void this.parts?.load();
    this.marked = { held: "", marks: [] };
    session?.on("change", this.changed);
    session?.on("confirm", this.confirming);
    session?.on("compose", this.composing);
    this.generation++;
    this.acts = new Map();
    this.streams = new Map();
    this.life = "";
    for (const send of this.windows) send({ part: this.whole() });
  }

  /** A word for each window. */
  private tell(said: Said): void {
    for (const send of this.windows) send(said);
  }

  /** A cancel that ends running work, which the session asks the operator to confirm first, as the TUI does. */
  private confirming = (cancel: Cancel): void => {
    this.cancelling = cancel;
    const chains = this.shown?.chainsOf(cancel.reach) ?? "";
    this.tell({ confirm: { act: cancel.act.id, acts: cancel.reach.acts.map((act) => act.id), chains } });
  };

  /** A message that the operator took back, which goes back to the composer of its chain. */
  private composing = (text: string): void => {
    this.tell({ compose: { chain: this.shown?.selected ?? "", text } });
  };

  /** A change of the life or of the list: a part goes out once PACE passed since the last one. */
  private changed = (): void => {
    if (this.timer) return;
    this.timer = setTimeout(
      () => {
        this.timer = undefined;
        this.last = Date.now();
        this.follow();
        const part = this.part();
        if (part) for (const send of this.windows) send({ part });
      },
      Math.max(0, this.last + PACE - Date.now()),
    );
  };

  /** The life that a session holds, as the window shows it. */
  private lifeOf(session: Session): Life {
    const { host } = session;
    return {
      record: host.record ?? "",
      root: session.engine.root,
      directory: host.directory,
      place: basename(host.directory),
      name: session.sessionName,
      selected: session.selected,
      actor: session.actor,
      status: session.status(),
      loading: session.loading,
      notice: session.notice,
      error: session.error,
      ...(host.reading ? { reading: host.reading } : {}),
      recorded: host.recorded ?? false,
      pending: host.pending.size,
      cost: host.cost,
      measures: host.measures,
      asks: session.desk.asks,
      ...(session.aimed ? { aimed: session.aimed.id } : {}),
      compactions: host.compactions,
      marks: this.marksOf(session),
    };
  }

  /** The marks that the parts give for each chain that a compaction holds, as the feed of the TUI shows them, given
   * again only when the parts, the compactions or the acts changed. */
  private marksOf(session: Session): Mark[] {
    const { compactions } = session.host;
    const parts = this.parts;
    if (!parts || !compactions.length) return [];
    const held = `${parts.version} ${compactions.length} ${session.acts.length} ${session.acts.at(-1)?.id}`;
    if (held === this.marked.held) return this.marked.marks;
    const seen = (chain: string): Seen => ({
      chain,
      acts: session.acts,
      compactions,
      of: (kind) => session.acts.filter((act) => act.kind === kind),
      run: (word) => session.runWord(word),
      say: (text) => {
        session.notice = text;
      },
      send: (text) => session.submit(text),
      // The app draws only the marks of a part, which read the life, so nothing here asks the operator.
      draft: () => {
        throw new Error("The app has no input that a part fills.");
      },
      open: (url) => parts.opener(url),
      choose: () => Promise.reject(new Error("The app opens no dialog of a part.")),
      form: () => Promise.reject(new Error("The app opens no dialog of a part.")),
      confirm: () => Promise.reject(new Error("The app opens no dialog of a part.")),
    });
    const runs = (line: Line) =>
      typeof line === "string"
        ? [{ text: line, tone: "text" }]
        : line.map(([text, tone]) => ({ text, tone: tone ?? "text" }));
    const marks = new Map<string, Mark>();
    for (const chain of new Set(compactions.map((one) => one.chain)))
      for (const { chain: on, at, title, lines } of parts.marks(seen(chain)))
        marks.set(`${on} ${at} ${title}`, { chain: on, at, title, lines: (lines ?? []).map(runs) });
    this.marked = { held, marks: [...marks.values()].sort((one, other) => one.at - other.at) };
    return this.marked.marks;
  }

  /** The projects that the TUI and the app know, each with its records that are not archived. */
  private projectsOf(): Project[] {
    return this.workspaces.groups.map((group: Workspace) => ({
      directory: group.directory,
      name: group.name,
      records: group.sessions
        .filter((entry) => !entry.archived)
        .map(({ path, name, status, error, modified, size }) => ({
          path,
          name,
          status,
          error,
          modified,
          size,
        })),
    }));
  }

  /** What a stream holds after what the host sent of it, or nothing when it holds nothing new. */
  private streamed(id: string, stream: Stream): StreamPart | undefined {
    const sent = this.streams.get(id) ?? { text: 0, thinking: 0, retry: "" };
    const retry = stream.retry ? JSON.stringify(stream.retry) : "";
    // A stream that a retry started again holds less than the host sent, so it goes again from its start.
    const text = stream.text.length < sent.text ? 0 : sent.text;
    const thinking = stream.thinking.length < sent.thinking ? 0 : sent.thinking;
    if (text === stream.text.length && thinking === stream.thinking.length && retry === sent.retry)
      return undefined;
    this.streams.set(id, { text: stream.text.length, thinking: stream.thinking.length, retry });
    return {
      id,
      chain: stream.chain,
      at: text,
      text: stream.text.slice(text),
      thinkingAt: thinking,
      thinking: stream.thinking.slice(thinking),
      ...(stream.retry ? { retry: stream.retry } : {}),
    };
  }

  /** The whole life, for a window that connects: it leaves what the host sent as it is, so the other windows miss no
   * change. A part that comes after it may hold again what it holds, which changes nothing in a window. */
  private whole(): Part {
    const session = this.shown;
    const streams: StreamPart[] = [...(session?.host.streams ?? [])].map(([id, stream]) => ({
      id,
      chain: stream.chain,
      at: 0,
      text: stream.text,
      thinkingAt: 0,
      thinking: stream.thinking,
      ...(stream.retry ? { retry: stream.retry } : {}),
    }));
    return {
      generation: this.generation,
      whole: true,
      projects: this.projectsOf(),
      life: session ? this.lifeOf(session) : null,
      acts: session ? [...session.acts] : [],
      streams,
      ended: [],
    };
  }

  /** What changed since the last part, or nothing when nothing changed. */
  private part(): Part | undefined {
    const session = this.shown;
    const part: Part = { generation: this.generation, whole: false, acts: [], streams: [], ended: [] };
    const projects = this.projectsOf();
    const listed = JSON.stringify(projects);
    if (listed !== this.projects) {
      this.projects = listed;
      part.projects = projects;
    }
    if (session) {
      const life = this.lifeOf(session);
      const said = JSON.stringify(life);
      if (said !== this.life) {
        this.life = said;
        part.life = life;
      }
      // The session puts a new object in the place of each act that changed, and keeps the others.
      for (const act of session.acts)
        if (this.acts.get(act.id) !== act) {
          this.acts.set(act.id, act);
          part.acts.push(act);
        }
      for (const [id, stream] of session.host.streams) {
        const piece = this.streamed(id, stream);
        if (piece) part.streams.push(piece);
      }
      for (const id of this.streams.keys())
        if (!session.host.streams.has(id)) {
          this.streams.delete(id);
          part.ended.push(id);
        }
    }
    const changed =
      part.projects ||
      part.life !== undefined ||
      part.acts.length ||
      part.streams.length ||
      part.ended.length;
    return changed ? part : undefined;
  }

  /** The session that a call acts on: the one that the windows show. */
  private session(): Session {
    const session = this.shown ?? this.workspaces.current?.session;
    if (!session) throw new Error("No life is open.");
    return session;
  }

  /** The project of a directory, which the list holds from now on. */
  private async project(directory: string): Promise<Workspace> {
    return this.workspaces.add(resolve(directory));
  }

  /** A record that goes on as a life, as the TUI opens it, or opens to read when another engine made it. */
  async open(record: string): Promise<void> {
    const path = resolve(record);
    for (const group of this.workspaces.groups) {
      const entry = group.sessions.find((one) => one.path === path);
      if (entry) return this.workspaces.select(entry);
    }
    await this.workspaces.import(path, await this.project(projectOf(path)));
  }

  /** A new life in a project, on the models of the catalog, as the TUI makes one. */
  async create(directory: string): Promise<void> {
    await this.workspaces.create(await this.project(directory));
  }

  /** A call of a window, by its name, and what it comes to. A failure of the session shows in its bar too. */
  async call(name: string, args: unknown[]): Promise<unknown> {
    const text = (at: number) => String(args[at] ?? "");
    switch (name) {
      case "open":
        return this.open(text(0));
      case "create":
        return this.create(text(0));
      case "add":
        await this.project(text(0));
        return this.workspaces.refresh();
    }
    const session = this.session();
    try {
      switch (name) {
        case "show":
          return await session.select(text(0));
        case "send":
          // The composer sends to the chain that the window shows. A line answers the question that waits on that
          // chain, as the composer of the TUI does, and else goes to its model as a message.
          if (session.selected !== text(0)) await session.select(text(0));
          return await session.submit(text(1));
        case "resume":
          await session.host.resume();
          return await session.refresh();
        case "answer": {
          // A button of a choice answers the question that it names, as a line of the composer answers it.
          const ask = session.desk.asks.find((one) => one.id === text(0));
          if (!ask) throw new Error(`${text(0)} waits for no answer.`);
          return await session.answer(ask, text(1));
        }
        case "pause":
        case "wake":
          // A control names the act that the operator picked, and reaches that act alone with what stands under it.
          if (!text(0)) throw new Error(`A ${name} names the act that it holds.`);
          return await session.command(`/${name} ${text(0)}`);
        case "cancel":
          if (!text(0)) throw new Error("A cancel names the act that it ends.");
          return await session.cancel(text(0));
        case "confirm": {
          const cancel = this.cancelling;
          this.cancelling = undefined;
          if (cancel && args[0] === true) await session.end(cancel);
          return;
        }
        case "takeBack": {
          // The last message of the operator on the chain: one that runs is cancelled and comes back to the
          // composer, and one that is done leaves a new branch of the chain, as the undo of the TUI does.
          if (session.selected !== text(0)) await session.select(text(0));
          const own = session.lastOwn;
          if (own && session.isUserPrompt(own)) await session.cancel();
          else await session.undo();
          return session.selected;
        }
        case "files": {
          const { files, commands } = await session.filesAt(text(0));
          return { files: files.map(({ path, made }) => ({ path, made: made ?? false })), commands };
        }
        case "rewind": {
          // What a rewind gives back, as the tree of the TUI offers it: the conversation, the files, or both.
          const mode = text(1);
          if (mode === "files") await session.restoreAt(text(0));
          else
            await session.rewind(
              text(0),
              undefined,
              mode === "both" ? await session.filesAt(text(0)) : undefined,
            );
          return session.selected;
        }
      }
    } catch (error) {
      session.fail(error);
      throw error;
    }
    throw new Error(`${name} is no call of the host.`);
  }

  /** The end of the host: each life ends, and saves what it holds. */
  async dispose(): Promise<void> {
    clearTimeout(this.timer);
    this.windows.clear();
    this.shown?.off("change", this.changed);
    this.shown?.off("confirm", this.confirming);
    this.shown?.off("compose", this.composing);
    this.parts?.dispose();
    await this.workspaces.dispose();
  }
}
