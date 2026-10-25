import { spawn } from "node:child_process";
import { EventEmitter } from "node:events";
import { closeSync, openSync, readFileSync } from "node:fs";
import { createConnection, type Socket } from "node:net";
import type {
  Act,
  FileChange,
  ImageAttachment,
  LiveAct,
  Engine as Native,
  Repaired,
  SetupInfo,
  Stream,
  Turn,
} from "furb";
import type { Measure } from "furb/life";
import {
  buildOf,
  framed,
  type Hello,
  hostPlace,
  type Place,
  PROTOCOL,
  type Role,
  reachOf,
  type Served,
} from "./hosting.ts";
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
  update({ facts, prompts, notices, streams, pending, measures, ...plain }: HostState): void {
    Object.assign(this, plain);
    if (Object.keys(measures).length) this.measures = { ...this.measures, ...measures };
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
class Link {
  closed = false;
  /** Why the link ended, which each later request fails with. */
  private why = "The session is disposed.";
  readonly host: HostView;
  private sequence = 0;
  private readonly waiting = new Map<number, { resolve(value: unknown): void; reject(error: Error): void }>();
  /** The requests that wait for a transport, as a window that attaches again holds them. */
  private held: Request[] = [];
  private sender?: (message: Request) => void;
  constructor(local?: (target: string, method: string) => Promise<unknown> | undefined) {
    this.host = new HostView(
      (target, method, args) => local?.(target, method) ?? this.request(target, method, args),
    );
  }
  set send(sender: ((message: Request) => void) | undefined) {
    this.sender = sender;
    if (sender) for (const message of this.held.splice(0)) sender(message);
  }
  request(target: string, method: string, args: unknown[]): Promise<unknown> {
    return new Promise<unknown>((resolve, reject) => {
      if (this.closed) {
        reject(new Error(this.why));
        return;
      }
      const message = { id: ++this.sequence, target, method, args };
      this.waiting.set(message.id, { resolve, reject });
      if (this.sender) this.sender(message);
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
export async function openEngine(options: EngineOptions): Promise<{ engine: Engine; host: HostView }> {
  if (options.attach) {
    const { host, port, key } = reachOf(options.attach);
    return attach(options, () => dial(host, port), key);
  }
  const place =
    options.host && options.record && !isRecorded(options.record) ? hostPlace(options.record) : undefined;
  return place ? attach(options, () => reach(options, place), "", true) : inWindow(options);
}

/** The life in a worker of the window, which ends with the window. */
async function inWindow(options: EngineOptions): Promise<{ engine: Engine; host: HostView }> {
  const worker = new Worker(new URL("worker.ts", import.meta.url).href, { type: "module" });
  const link = new Link((target) =>
    target === "host"
      ? Promise.reject(new Error("This life runs in its window, so no other operator reaches it."))
      : undefined,
  );
  link.send = (message) => worker.postMessage(message);
  worker.onmessage = ({ data }) => link.receive(data);
  worker.onerror = (event) => {
    link.end(event.message);
    link.host.emit("fault", new Error(event.message));
  };
  let root: string;
  try {
    root = (await link.request("session", "open", [options])) as string;
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

/** A connection to the host of a life: the host that answers at its place, or a host that the window starts as a
 * process of its own, detached from the window, which it waits for. A host that ends before it listens says why, as
 * on a record whose lease another process holds: unless another window started a host meanwhile, the window then
 * fails with that reason. */
async function reach(options: EngineOptions, place: Place): Promise<Socket> {
  const found = await connect(place.socket);
  if (found) return found;
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
    if (ended !== undefined) {
      const again = await connect(place.socket);
      if (again) return again;
      let failed: { pid?: number; error?: string } = {};
      try {
        failed = JSON.parse(readFileSync(place.failed, "utf8"));
      } catch {}
      throw new Error(
        failed.pid === child.pid && failed.error
          ? failed.error
          : `The host of the life ended with ${ended} before it listened: ${place.log} says why.`,
      );
    }
    await Bun.sleep(20);
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
  reachHost: () => Promise<Socket>,
  key = "",
  takes = false,
): Promise<{ engine: Engine; host: HostView }> {
  let socket: Socket | undefined;
  const link: Link = new Link((target, method) => {
    if (target !== "session" || method !== "dispose") return undefined;
    link.end("The session is disposed.");
    socket?.end();
    return Promise.resolve();
  });
  const lost = "The host of the life ended before it answered.";
  const mine = buildOf().catch((error: unknown) => `unknown: ${error}`);
  let first: Hello | undefined;
  let reaching: Promise<void> | undefined;
  const hello = async (): Promise<Hello> => {
    for (let took = false; ; took = true) {
      const held = await reachHost();
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
      const said = (await link.request("host", "hello", [
        { window: process.pid, protocol: PROTOCOL, build, key },
      ])) as Hello;
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
        return;
      } catch (error) {
        if (error instanceof Error && error.message === lost && tries < 5 && !link.closed) continue;
        link.end(error instanceof Error ? error.message : String(error));
        link.host.emit("fault", error instanceof Error ? error : new Error(String(error)));
        return;
      }
    }
  };
  first = await hello();
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
