import { spawn } from "node:child_process";
import { EventEmitter } from "node:events";
import { closeSync, openSync, readFileSync, rmSync } from "node:fs";
import { createConnection, type Socket } from "node:net";
import { dirname, resolve } from "node:path";
import {
  type Act,
  type FileChange,
  type ImageAttachment,
  type LiveAct,
  type Engine as Native,
  type Repaired,
  ROOT,
  type SetupInfo,
  type Stream,
  type Turn,
} from "furb";
import type { Compaction, Measure, PauseCause } from "furb/life";
import {
  aboutOf,
  buildOf,
  framed,
  type Hello,
  hostPlace,
  isHost,
  OWNED,
  type Place,
  PROTOCOL,
  RIVAL,
  type Role,
  reachOf,
  type Served,
  stopProcess,
} from "./hosting.ts";

/** The reason of a link that a window ended by its own hand, which is no loss of the life. */
const DISPOSED = "The session is disposed.";
/** Why a window that stopped its host holds no life. */
const STOPPED = "The host of the life was stopped from this window.";

import type { EngineOptions } from "./models.ts";
import type { ProviderChoice } from "./providers.ts";
import { type Carry, isRecorded, type Reading } from "./reading.ts";
import type { ActRow, FollowUp } from "./session.ts";

/** What one answer of a model read and wrote: its whole prompt, its output, the reads and the writes of the cache,
 * and its dollars. */
export type Usage = NonNullable<Turn[2]>;

/** The models of the roster, each as its name, its efforts and its window. */
export type Roster = [string, string[], number][];
/** The items of a list from a place on, which take the place of the items that the host holds from that place. */
export interface Tail<T> {
  from: number;
  list: T[];
}

/** A view of a chain. A host that gives the mark of the last snapshot it took takes only what changed since then: the
 * roster when it changed, the words that the program changed, and the tails of the turns and of the answers. */
export interface Snapshot {
  /** The mark of this snapshot, which the host gives with its next request once it holds this snapshot. */
  mark: number;
  dispatched: string[];
  paused: boolean;
  /** The cause that the pause over the chain gave, null when none stands or none was given. */
  cause: PauseCause | null;
  roster?: Roster;
  /** The acts that changed after the count the session asked with. */
  acts: ActRow[];
  /** The count of changes of the act table that the acts are read at. */
  count: number;
  selected: string;
  turns: Tail<Turn>;
  /** The usage of each answer of the chain, in order. A chain with a source reads the answers of its origin in its
   * turns, and they are not among these. */
  answers: Tail<Usage>;
  /** The share of the window that the last answer of the chain filled, as filled of the engine gives it. Nothing
   * before the first answer. */
  filled?: number;
  /** The word of each rung of the program, whole, or the words that changed since the snapshot of the mark that the
   * host gave. */
  program?: { whole: boolean; words: Record<string, string> };
  actor: string;
  directory: string;
}

type Returned<T> = T extends Act ? string : Awaited<T>;
/** Async calls are a choice of the TUI. The library's methods remain synchronous. */
export type Engine = {
  [K in keyof Native]: Native[K] extends (...args: infer A) => infer R
    ? (...args: A) => Promise<Returned<R>>
    : Native[K];
};
/** A question that waits for the operator. */
type Prompt = { id: string; shape: string; message: string };
/** A prompt of None to the operator, which asks nothing and was closed as the console took it, until the operator
 * dismisses it. */
type Notice = { id: string; message: string };
/** What the session in the worker holds that the host keeps as it comes. */
interface Plain {
  completed: number;
  cost: number;
  directory: string;
  imageDirectory: string;
  actor: string;
  /** The path of the record, when the session keeps one. */
  record?: string;
  /** How many file changes the session holds. */
  changes: number;
  /** Why the record is open to read, when a life on it was refused for a drift, as on a record of another engine. */
  reading?: Reading;
  /** Whether the life is a recording, which shows its log and runs nothing. */
  recorded?: boolean;
  /** The measure of each chain whose models answered, by the chain. */
  measures: Record<string, Measure>;
  /** Each compaction of a chain, in the order of the record. A state holds those since the state before it. */
  compactions: Compaction[];
  /** The count of facts that the life said, which each state carries, so a window knows what it missed. */
  seq: number;
}
/** What the session in the worker holds, as it sends it to the host, and how many facts the life said since the last
 * state, which the host needs to know of and not to hold. Its measures are those of the chains whose models answered
 * since the last state, which join the measures that the host holds. */
export interface HostState extends Plain {
  facts: number;
  prompts: Prompt[];
  notices: Notice[];
  streams: [string, Stream][];
  pending: [string, string][];
}

/** The session in the worker, as the host sees it: what it holds, and the requests the host makes of it. */
export class HostView extends EventEmitter implements Plain {
  completed = 0;
  cost = 0;
  directory = "";
  imageDirectory = "";
  actor = "";
  record?: string;
  changes = 0;
  reading?: Reading;
  recorded?: boolean;
  measures: Record<string, Measure> = {};
  compactions: Compaction[] = [];
  seq = 0;
  prompts = new Map<string, Prompt>();
  notices = new Map<string, Notice>();
  streams = new Map<string, Stream>();
  pending = new Map<string, string>();
  constructor(private request: (target: string, method: string, args: unknown[]) => Promise<unknown>) {
    super();
  }
  /** The text of each part of the state as the host read it last. A part that did not change keeps its object, which
   * a view that reads the whole life keys on. */
  private read: Record<string, string> = {};
  private changed(part: string, value: unknown): boolean {
    const text = JSON.stringify(value);
    if (this.read[part] === text) return false;
    this.read[part] = text;
    return true;
  }
  // A host of an older build that speaks the same protocol sends a state with no measures and no compactions.
  update({
    facts,
    prompts,
    notices,
    streams,
    pending,
    measures = {},
    compactions = [],
    ...plain
  }: HostState): void {
    Object.assign(this, plain);
    if (Object.keys(measures).length) this.measures = { ...this.measures, ...measures };
    if (compactions.length) this.compactions = [...this.compactions, ...compactions];
    if (this.changed("prompts", prompts))
      this.prompts = new Map(prompts.map((prompt) => [prompt.id, prompt]));
    if (this.changed("notices", notices))
      this.notices = new Map(notices.map((notice) => [notice.id, notice]));
    this.streams = new Map(streams);
    if (this.changed("pending", pending)) this.pending = new Map(pending);
    if (facts) this.emit("facts");
    this.emit("change");
  }
  /** A request of the worker: the method of one of its targets, the session, its console, or the library, which
   * answers what the host reads of the life. */
  private ask<T>(
    target: "session" | "console" | "library" | "host",
    method: string,
    ...args: unknown[]
  ): Promise<T> {
    return this.request(target, method, args) as Promise<T>;
  }
  /** Whether the model of an actor takes an image, as the catalog of the crate says. */
  sees = (actor: string) => this.ask<boolean>("library", "sees", actor);
  /** What this window does in the life, as the hello of its host gave it: a window that reads changes nothing. */
  role: Role = "write";
  /** Why this window holds no life now, in plain words, and nothing while the life is open. */
  lost = "";
  /** Whether the life runs in a host of this machine, which this window can stop and open again. */
  local = false;
  /** The window reaches the life again: the host of the record that answers, or a host that it starts, and the view
   * reads the life from the start. */
  reopen = () => this.ask<void>("host", "reopen");
  /** The end of the host of this life by force, for a host that does not answer the end of the life. */
  stop = () => this.ask<void>("host", "stop");
  /** The addresses at which other operators reach the life over the network, to read or to write. The first serve
   * opens the door of the host at the address that it is given, host:port or a port, on 127.0.0.1 and a free port by
   * default, and each later one gives the same addresses. */
  serve = (address = "") => this.ask<Served>("host", "serve", address);
  /** The models the catalog of the crate offers, each as its name, its efforts and its window. */
  catalog = () => this.ask<[string, string[], number][]>("library", "catalog");
  /** What the first run checks of this machine, and nothing in the demo, which needs no model. */
  setup = () => this.ask<SetupInfo | null>("library", "setup");
  providers = () => this.ask<ProviderChoice[]>("library", "providers");
  provider = (name: string, value: unknown) => this.ask<boolean>("library", "provider", name, value);
  codexAuth = (action = "status") => this.ask<import("furb").CodexAuthInfo>("library", "codexAuth", action);
  reload = () => this.ask<boolean>("library", "reload");
  /** The name of the model that a name gives in the roster or in the catalog, and nothing when it gives none. */
  model = (name: string) => this.ask<string | null>("library", "model", name);
  answer = (id: string, value: string) => this.ask<unknown>("console", "answer", id, value);
  /** The words of the operator in place of each value that a question offers, which close it as Refused. */
  decline = (id: string, words: string) => this.ask<unknown>("console", "decline", id, words);
  /** The notice that the operator read, which the TUI shows no more as unread. */
  dismiss = (id: string) => this.ask<unknown>("console", "dismiss", id);
  attachImage = (path: string) => this.ask<ImageAttachment>("session", "attachImage", path);
  sendQueued = (entry: FollowUp) => this.ask<string>("library", "queue", entry);
  resume = () => this.ask<unknown>("session", "resume");
  source = () => this.ask<string>("library", "source");
  /** What a new life from the record open to read carries, and the first word that binds it. */
  carry = () => this.ask<Carry>("library", "carry");
  snapshot = (chain: string, since = 0, held = 0) =>
    this.ask<Snapshot>("library", "snapshot", chain, since, held);
  readChanges = (start: number, count: number) => this.ask<FileChange[]>("library", "changes", start, count);
  /** The net change of each file across the changes that the writes made, or across every change of the life when
   * no list of writes is given. */
  netChanges = (writes: string[] | null) => this.ask<FileChange[]>("library", "net", writes);
  /** The change of each file of the branch against main, as git holds it in the directory of the session. */
  branchChanges = () => this.ask<FileChange[]>("library", "branch");
  /** The journal of file changes written again with each change that it holds, and the counts of changes kept and of
   * lines removed. */
  repairChanges = () => this.ask<Repaired>("library", "repairChanges");
  /** A new journal of file changes, and the path that the damaged one is kept at. */
  renewChanges = () => this.ask<string>("library", "renewChanges");
  /** An act whole, with all that a command printed, and nothing when the life holds no such act. */
  act = (id: string) => this.ask<LiveAct | undefined>("library", "act", id);
  /** The text the World reads at a path, from where a chain stands, which makes no act and keeps nothing. */
  look = (path: string, chain: string) =>
    this.ask<{ path: string; content: string }>("library", "look", path, chain);
  /** The end of the window's hold on the life: a life in the window ends with it, and a life in a host of its own
   * goes on. */
  dispose = () => this.ask<void>("session", "dispose");
  /** The end of the life, its commands and its models, wherever it runs. */
  end = () => this.ask<void>("session", "end");
}

type Request = { id: number; target: string; method: string; args: unknown[] };
/** What the holder of a life sends a window: an answer to a request, or a push of the life. */
type Message = {
  authentication?: import("furb").CodexAuthInfo;
  id?: number;
  value?: unknown;
  error?: string;
  name?: string;
  disposed?: boolean;
  fault?: string;
  log?: string;
  state?: HostState;
  ended?: string;
};

/** The requests of a window to the holder of its life, and what comes back: each request takes a number, and each
 * answer goes to its request, while each push goes to the host view. */
export class Link {
  closed = false;
  /** Why the link ended, which each later request fails with. */
  private why = DISPOSED;
  readonly host: HostView;
  private sequence = 0;
  private readonly waiting = new Map<number, { resolve(value: unknown): void; reject(error: Error): void }>();
  /** The requests that wait for a transport, as a window that attaches again holds them, or for the open of the life. */
  private held: Request[] = [];
  private sender?: (message: Request) => void;
  /** Whether the life is not open yet, so that a request waits for it, in the order it was made, but for the requests
   * that open the life and for the end of the window's hold on it. */
  opening = false;
  /** The requests that go while the life opens. */
  private readonly early = new WeakSet<Request>();
  constructor(local?: (target: string, method: string) => Promise<unknown> | undefined) {
    this.host = new HostView(
      (target, method, args) => local?.(target, method) ?? this.request(target, method, args),
    );
  }
  set send(sender: ((message: Request) => void) | undefined) {
    this.sender = sender;
    this.flush();
  }
  /** Sends the requests that wait, but for those that wait for the open of the life while it opens. */
  private flush(): void {
    const sender = this.sender;
    if (!sender) return;
    const goes = (message: Request) => !this.opening || this.early.has(message);
    const now = this.held.filter(goes);
    this.held = this.held.filter((message) => !goes(message));
    for (const message of now) sender(message);
  }
  /** The life answered its open: the requests that waited for it go. */
  open(): void {
    this.opening = false;
    this.flush();
  }
  request(target: string, method: string, args: unknown[], first = false): Promise<unknown> {
    return new Promise<unknown>((resolve, reject) => {
      if (this.closed) {
        reject(new Error(this.why));
        return;
      }
      const message = { id: ++this.sequence, target, method, args };
      this.waiting.set(message.id, { resolve, reject });
      if (first || (target === "session" && method === "dispose")) this.early.add(message);
      if (this.sender && (!this.opening || this.early.has(message))) this.sender(message);
      else this.held.push(message);
    });
  }
  receive(data: Message): void {
    if (data.authentication) {
      this.host.emit("authentication", data.authentication);
      return;
    }
    if (data.fault) {
      this.host.emit("fault", new Error(data.fault));
      return;
    }
    if (data.log) {
      this.host.emit("log", data.log);
      return;
    }
    if (data.state) {
      this.host.update(data.state);
      return;
    }
    if (data.ended) {
      this.end(data.ended);
      this.host.emit("fault", new Error(data.ended));
      return;
    }
    const promise = this.waiting.get(data.id ?? 0);
    this.waiting.delete(data.id ?? 0);
    if (data.error)
      promise?.reject(Object.assign(new Error(data.error), data.name ? { name: data.name } : {}));
    else promise?.resolve(data.value);
    if (data.disposed) this.end("The session is disposed.");
  }
  /** Each request that waits fails with a reason, as the holder of the life will not answer it. */
  drop(why: string): void {
    for (const pending of this.waiting.values()) pending.reject(new Error(why));
    this.waiting.clear();
    this.held = [];
  }
  /** The end of the link: no request goes any more. */
  end(why: string): void {
    this.closed = true;
    this.why = why;
    this.drop(why);
    if (why === DISPOSED) return;
    this.host.lost = why;
    this.host.emit("change");
  }
  /** The link takes requests again, as the window reaches a host again. */
  revive(): void {
    this.closed = false;
    this.why = DISPOSED;
    this.host.lost = "";
  }
  /** The engine of the life, as a proxy whose methods are requests. */
  engine(root: string): Engine {
    return new Proxy(
      { root },
      {
        get: (target, key) => {
          if (key === "root") return target.root;
          if (key === "disposed") return this.closed;
          if (key === "then") return undefined;
          return (...args: unknown[]) => this.request("engine", String(key), args);
        },
      },
    ) as Engine;
  }
}

/** The life of a record, in a host of its own when the options ask it and a socket serves, or in a worker of the
 * window. A recorded life runs nothing, so it stays in the window. A window that names the address of a host on the
 * network attaches to that host, and keeps its view at its own record. */
export async function openEngine(
  options: EngineOptions,
  made?: (link: Link) => void,
): Promise<{ engine: Engine; host: HostView }> {
  if (options.attach) {
    const { host, port, key } = reachOf(options.attach);
    return attach(options, () => dial(host, port), key, false, made);
  }
  const place =
    options.host && options.record && !isRecorded(options.record) ? hostPlace(options.record) : undefined;
  return place
    ? attach(options, (stopped) => reach(options, place, stopped), "", true, made)
    : inWindow(options, made);
}

/** A life that opens in the background: its engine and its host stand at once, so a session can draw its view before
 * the life is open. Each request of the view waits for the open, in the order it was made, and ready tells when the
 * life is open, or why it is not. The root of a life is the same for each life, and so is its record. */
export function openingEngine(options: EngineOptions): {
  engine: Engine;
  host: HostView;
  ready: Promise<void>;
} {
  const made: { link?: Link } = {};
  const opened = openEngine(options, (link) => {
    made.link = link;
    link.opening = true;
    link.host.directory = resolve(options.cwd ?? (options.record ? dirname(options.record) : "."));
    link.host.record = options.record && resolve(options.record);
  });
  // The open is begun by now, and it made the link before its first wait.
  const held = made.link;
  if (!held) throw new Error("The life did not begin to open.");
  const ready = opened.then(({ engine }) => {
    if (engine.root !== ROOT)
      throw new Error(`The life opened on the chain ${engine.root}, and not on ${ROOT}.`);
  });
  return { engine: held.engine(ROOT), host: held.host, ready };
}

/** The life in a worker of the window, which ends with the window. */
async function inWindow(
  options: EngineOptions,
  made?: (link: Link) => void,
): Promise<{ engine: Engine; host: HostView }> {
  const worker = new Worker(new URL("worker.ts", import.meta.url).href, { type: "module" });
  const link = new Link((target, method) =>
    target === "host"
      ? Promise.reject(
          new Error(
            method === "stop" || method === "reopen"
              ? "This life runs in its window, so it has no host to stop or to open again."
              : "This life runs in its window, so no other operator reaches it.",
          ),
        )
      : undefined,
  );
  made?.(link);
  link.send = (message) => worker.postMessage(message);
  worker.onmessage = ({ data }) => link.receive(data);
  worker.onerror = (event) => {
    link.end(event.message);
    link.host.emit("fault", new Error(event.message));
  };
  let root: string;
  try {
    root = (await link.request("session", "open", [options], true)) as string;
    link.open();
  } catch (error) {
    // No caller holds a worker whose life did not open, so it ends here.
    link.end("The session is disposed.");
    worker.terminate();
    throw error;
  }
  return { engine: link.engine(root), host: link.host };
}

/** A connection to a host on the network, which fails with the reason when no host answers there. */
function dial(host: string, port: number): Promise<Socket> {
  return new Promise((resolve, reject) => {
    const socket = createConnection({ host, port });
    socket.once("connect", () => {
      socket.removeAllListeners("error");
      resolve(socket);
    });
    socket.once("error", (error) =>
      reject(new Error(`No host of a life answers at ${host}:${port}: ${error.message}`)),
    );
  });
}

/** A connection to the socket at a path, and nothing when no host answers there. */
function connect(path: string): Promise<Socket | undefined> {
  return new Promise((resolve) => {
    const socket = createConnection(path);
    socket.once("connect", () => {
      socket.removeAllListeners("error");
      resolve(socket);
    });
    socket.once("error", () => resolve(undefined));
  });
}

/** How long a window waits for a sign of a host before it starts one again: a host that ends frees the lease at its
 * end, so a lease that no host of furb holds is held by another program or is about to go. */
const QUIET = 1000;

/** The socket of a host that another window started, or that is about to end, while its process runs: a host that
 * opens a long record takes minutes, so the wait has no end while the process lives. Nothing when no host lives for
 * the time of QUIET. */
async function waiting(place: Place, stopped: () => boolean): Promise<Socket | undefined> {
  let seen = Date.now();
  for (;;) {
    const socket = await connect(place.socket);
    if (socket) return socket;
    if (stopped()) throw new Error(STOPPED);
    const about = aboutOf(place);
    if (about && isHost(about.pid)) seen = Date.now();
    else if (Date.now() - seen > QUIET) return undefined;
    await Bun.sleep(250);
  }
}

/** A host that this window starts as a process of its own, detached from the window: its socket once it listens, or
 * why it ended before. A window that stops its host while the host starts ends it. */
async function launch(
  options: EngineOptions,
  place: Place,
  stopped: () => boolean,
): Promise<Socket | string> {
  const log = openSync(place.log, "a", 0o600);
  const child = spawn(
    process.execPath,
    [new URL("host.ts", import.meta.url).pathname, JSON.stringify(options)],
    {
      detached: true,
      stdio: ["ignore", log, log],
    },
  );
  closeSync(log);
  child.unref();
  let ended: number | undefined;
  child.once("exit", (code, signal) => {
    ended = code ?? (signal ? 128 : 1);
  });
  for (;;) {
    const socket = await connect(place.socket);
    if (socket) return socket;
    if (stopped()) {
      if (child.pid) await stopProcess(child.pid, graceOf());
      throw new Error(STOPPED);
    }
    if (ended !== undefined) {
      const again = await connect(place.socket);
      if (again) return again;
      let failed: { pid?: number; error?: string } = {};
      try {
        failed = JSON.parse(readFileSync(place.failed, "utf8"));
      } catch {}
      return failed.pid === child.pid && failed.error
        ? failed.error
        : `The host of the life ended with ${ended} before it listened: ${place.log} says why.`;
    }
    await Bun.sleep(20);
  }
}

/** How long a host that a window stops has to end the life before the window kills it, in milliseconds. */
const graceOf = () => Number(process.env.FURB_HOST_GRACE ?? 12000);
/** How long a host has to answer the end of the life before the window stops it, in milliseconds. */
const endingOf = () => Number(process.env.FURB_HOST_ENDING ?? 15000);

/** A connection to the host of a life: the host that answers at its place, or a host that the window starts as a
 * process of its own, detached from the window, which it waits for. A host that ends before it listens says why. A
 * host that finds the life owned waits for that owner while it runs: the host of another window that opens a long
 * record, or one that ends. A lease that no host of furb holds fails with the words to end that program. */
async function reach(
  options: EngineOptions,
  place: Place,
  stopped: () => boolean = () => false,
): Promise<Socket> {
  for (let tries = 1; ; tries++) {
    const found = await connect(place.socket);
    if (found) return found;
    if (stopped()) throw new Error(STOPPED);
    const got = await launch(options, place, stopped);
    if (typeof got !== "string") return got;
    if (!got.includes(RIVAL) && !got.includes(OWNED)) throw new Error(got);
    const host = await waiting(place, stopped);
    if (host) return host;
    if (tries >= 3)
      throw new Error(
        "Another program holds the life, and it is no host that this window can reach. End that program, then open the life again.",
      );
  }
}

/** How long a window waits for a host of another build to end once it asked it to hand the life over: the host waits
 * up to 10 seconds for what runs, then ends the life. */
const TAKEOVER = 60000;

/** The life in a host of its own, which outlives the window. A dispose of the window detaches it, and the life goes
 * on; an end ends the life. When the host goes away with the life open, the window attaches again, to a host that it
 * starts again when none answers, and reads the acts from the start when it is another host. Its hello gives the key
 * of the window, which a host on the network asks. A window of a host of this machine, which takes, takes over a host
 * of another build: the host ends the life, and the window starts a host of its own build, which opens the life again
 * on the same record. */
async function attach(
  options: EngineOptions,
  reachHost: (stopped: () => boolean) => Promise<Socket>,
  key = "",
  takes = false,
  made?: (link: Link) => void,
): Promise<{ engine: Engine; host: HostView }> {
  let socket: Socket | undefined;
  /** Whether this window stopped its host, which ends each reach of it. */
  let stopping = false;
  const place = takes && options.record ? hostPlace(options.record) : undefined;
  const link: Link = new Link((target, method) => {
    if (target === "host" && (method === "stop" || method === "reopen"))
      return method === "stop" ? stop() : reopen();
    if (target === "session" && method === "end" && place) return ending();
    if (target !== "session" || method !== "dispose") return undefined;
    link.end(DISPOSED);
    socket?.end();
    return Promise.resolve();
  });
  link.host.local = Boolean(place);
  made?.(link);
  const lost = "The host of the life ended before it answered.";
  const mine = buildOf().catch((error: unknown) => `unknown: ${error}`);
  let first: Hello | undefined;
  let reaching: Promise<void> | undefined;
  const hello = async (): Promise<Hello> => {
    for (let took = false; ; took = true) {
      const held = await reachHost(() => stopping);
      socket = held;
      // The record of a host on the network stands on another machine, or is the record of another operator, so the
      // window keeps its view at its own record.
      const send = framed(held, (data: Message) => {
        if (socket !== held) return;
        link.receive(
          options.attach && data.state ? { ...data, state: { ...data.state, record: options.record } } : data,
        );
      });
      link.send = send;
      held.on("close", () => {
        if (socket !== held) return;
        link.send = undefined;
        link.drop(lost);
        if (!link.closed && first && !reaching)
          reaching = again().finally(() => {
            reaching = undefined;
          });
      });
      held.on("error", () => {});
      // A host that ends its side of the socket ends it whole: bun closes no socket whose peer ended while a write of
      // the window waits on it, so the socket would wait with no close.
      held.on("end", () => held.destroy());
      const build = await mine;
      const said = (await link.request(
        "host",
        "hello",
        [{ window: process.pid, protocol: PROTOCOL, build, key }],
        true,
      )) as Hello;
      // A host that this window started after a takeover keeps its build, so two windows of two builds that open
      // the life at once do not take it over from each other with no end.
      if (takes && !took && said.build !== build) {
        socket = undefined;
        link.send = undefined;
        await takeOver(held, send, said, build);
        continue;
      }
      if (said.protocol !== PROTOCOL) {
        held.end();
        throw new Error(
          `The host of ${said.record} speaks the protocol ${said.protocol}, and this window ${PROTOCOL}. End the life in a window of its build first.`,
        );
      }
      link.host.role = said.role ?? "write";
      return said;
    }
  };
  // A host that a kill ends can take a connection while its process ends, and then drops it before it answers the
  // hello: the window reaches the place again, where no host answers any more, and starts a new one.
  const again = async () => {
    for (let tries = 1; ; tries++) {
      try {
        const next = await hello();
        if (next.pid !== first?.pid || next.started !== first?.started) link.host.emit("reset");
        first = next;
        return;
      } catch (error) {
        if (error instanceof Error && error.message === lost && tries < 5 && !link.closed) continue;
        link.end(error instanceof Error ? error.message : String(error));
        link.host.emit("fault", error instanceof Error ? error : new Error(String(error)));
        return;
      }
    }
  };
  /** The end of the host by force: the window lets the life go, ends the process of the host, and removes what the
   * host left. The window does this when the host does not answer, and no end of the life reaches it. */
  const stop = async (): Promise<void> => {
    if (!place) throw new Error("This window reaches no host of this machine, so it cannot stop one.");
    stopping = true;
    const pid = aboutOf(place)?.pid ?? first?.pid;
    link.end(STOPPED);
    socket?.destroy();
    if (pid && isHost(pid)) await stopProcess(pid, graceOf());
    rmSync(place.socket, { force: true });
    rmSync(place.about, { force: true });
  };
  /** The end of the life, which waits for the host a bounded time: a host that does not answer is stopped. */
  const ending = async (): Promise<void> => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    const answered = link.request("session", "end", [], true).then(
      () => true,
      () => true,
    );
    const late = new Promise<boolean>((resolve) => {
      timer = setTimeout(() => resolve(false), endingOf());
    });
    const done = await Promise.race([answered, late]);
    clearTimeout(timer);
    if (!done) await stop();
  };
  /** The life reached again, whatever ended the link: a host that answers, or a host that the window starts. */
  const reopen = async (): Promise<void> => {
    if (!place)
      throw new Error("This window reaches no host of this machine, so it cannot open the life again.");
    stopping = false;
    const old = socket;
    socket = undefined;
    link.send = undefined;
    old?.destroy();
    link.revive();
    await again();
    if (!link.closed) link.host.emit("reset");
  };
  try {
    first = await hello();
  } catch (error) {
    // The requests that wait for the open of the life fail with the reason it did not open.
    link.end(error instanceof Error ? error.message : String(error));
    throw error;
  }
  link.open();
  return { engine: link.engine(first.root), host: link.host };
}

/** The end of a host of another build, which hands the life over: the window asks it to end the life, and waits until
 * it closes the socket. The ask is an end of the session, so a host that does not know the hand over ends the life
 * as at any end. */
async function takeOver(
  held: Socket,
  send: (message: object) => void,
  said: Hello,
  build: string,
): Promise<void> {
  let timer: Timer | undefined;
  const closed = new Promise<boolean>((resolve) => {
    held.once("close", () => resolve(true));
    timer = setTimeout(() => resolve(false), TAKEOVER);
  });
  send({ id: 0, target: "session", method: "end", args: [{ handover: { build, protocol: PROTOCOL } }] });
  const ended = await closed;
  clearTimeout(timer);
  if (ended) return;
  held.destroy();
  throw new Error(
    `The host of ${said.record}, of the build ${said.build}, did not hand the life over to the build ${build} in ${TAKEOVER / 1000} seconds.`,
  );
}
