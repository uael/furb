import { spawn } from "node:child_process";
import { EventEmitter } from "node:events";
import { closeSync, existsSync, mkdirSync, openSync, readFileSync, statSync } from "node:fs";
import { createConnection, type Socket } from "node:net";
import { homedir } from "node:os";
import { join } from "node:path";
import type { Fact, Standing, Turn } from "furb";
import { type Compaction, followed, isDone, operated, type Spend, WORK } from "furb/life";
import type {
  Act,
  Answer,
  Config,
  Event,
  FollowUp,
  Frame,
  Listed,
  Peer,
  Role,
  Request1 as Verb,
} from "furb/protocol";
import { PROTOCOL } from "furb/protocol";
import type { Setting } from "./configs.ts";

export type { Verb };

/** The folder of the daemon of this machine in the cache of furb, as `furb host` names it. */
export function daemonFolder(env: Record<string, string | undefined> = process.env): string {
  const named = (name: string) => env[name] || undefined;
  const base =
    named("XDG_CACHE_HOME") ??
    (process.platform === "win32"
      ? named("LOCALAPPDATA")
      : process.platform === "darwin"
        ? join(homedir(), "Library", "Caches")
        : join(homedir(), ".cache"));
  if (!base) throw new Error("This machine has no cache of furb, so no daemon has a place on it.");
  return join(base, "furb", "host");
}

/** Where a window dials the daemon of a folder: its socket, or its named pipe on Windows. */
export function daemonPlace(folder = daemonFolder()): string {
  return process.platform === "win32"
    ? `\\\\.\\pipe\\furb-${folder.replace(/[\\/:]/g, "-")}`
    : join(folder, "host.sock");
}

/** The program furb that serves the daemon: the one that FURB_BIN names, else the newest that `bun run build` made in
 * the target folder of this clone, else furb on the PATH. */
export function furbProgram(env: Record<string, string | undefined> = process.env): string {
  if (env.FURB_BIN) return env.FURB_BIN;
  const target = env.CARGO_TARGET_DIR || join(import.meta.dir, "..", "..", "target");
  const name = process.platform === "win32" ? "furb.exe" : "furb";
  const triple = env.CARGO_BUILD_TARGET ? [env.CARGO_BUILD_TARGET] : [];
  const built = ["debug", "release"]
    .map((profile) => join(target, ...triple, profile, name))
    .filter((path) => existsSync(path))
    .sort((one, other) => statSync(other).mtimeMs - statSync(one).mtimeMs);
  return built[0] ?? "furb";
}

/** How long a window waits for the end of a life that it let go, in milliseconds: the grace of the daemon, 30 s, and
 * the time that the life takes to write its checkpoint. */
const ENDING = 60000;

/** How long a window waits for a daemon that it started to listen, in milliseconds. */
const STARTING = Number(process.env.FURB_HOST_STARTING ?? 60000);

/** A daemon started as a process of its own, detached from the window, once it listens: it writes what it says in
 * host.log beside its socket, which says why when it ends before it listens. A daemon that another window started at
 * the same moment takes the place, and this one ends, so the window dials the place again until one answers. */
async function started(folder: string, place: string): Promise<Socket> {
  mkdirSync(folder, { recursive: true, mode: 0o700 });
  const logged = join(folder, "host.log");
  const log = openSync(logged, "a", 0o600);
  const child = spawn(furbProgram(), ["host"], { detached: true, stdio: ["ignore", log, log] });
  closeSync(log);
  child.unref();
  let ended: string | undefined;
  child.once("error", (error) => {
    ended = error.message;
  });
  child.once("exit", (code, signal) => {
    ended = `exit ${code ?? signal}`;
  });
  const until = Date.now() + STARTING;
  for (;;) {
    const socket = await dialed(place);
    if (socket) return socket;
    if (ended !== undefined && !/already listens/.test(tail(logged))) {
      const again = await dialed(place);
      if (again) return again;
      throw new Error(`The daemon of this machine ended (${ended}) before it listened: ${tail(logged)}`);
    }
    if (Date.now() > until)
      throw new Error(`The daemon of this machine did not listen in ${STARTING / 1000} s.`);
    await Bun.sleep(25);
  }
}

/** The last line that a daemon wrote in its log. */
function tail(path: string): string {
  try {
    return readFileSync(path, "utf8").trim().split("\n").at(-1) ?? "";
  } catch {
    return "";
  }
}

/** A connection to a place, or nothing when no daemon answers there. */
export function dialed(place: string): Promise<Socket | undefined> {
  return new Promise((resolve) => {
    const socket = createConnection(place);
    socket.once("connect", () => {
      socket.removeAllListeners("error");
      resolve(socket);
    });
    socket.once("error", () => resolve(undefined));
  });
}

/** A refusal of the daemon, named by its kind, as `closed` or `role`. */
export class Refused extends Error {
  constructor(
    readonly kind: string,
    why: string,
  ) {
    super(why);
    this.name = kind === "refused" ? "Refused" : kind;
  }
}

/** A request that the daemon left with no answer when its socket closed, as when it ends or hands over to another
 * build: it takes the stack of the verb that asked it, which names its caller. */
export class Closed extends Error {
  override name = "Closed";
}

/** The connection of a window to the daemon of its machine, which it starts when none answers. Each request takes a
 * number and its answer, and each frame that the daemon pushes is an event of the connection: `event` with the life,
 * the place and the event, `presence` with the life and the peers, and `ask` with the life, the number and the request
 * of a model that the window answers. When the daemon goes, as at a handover, the connection dials again, and tells
 * `reconnected` once the daemon in its place welcomed it, or `lost` with why it did not. */
export class Daemon extends EventEmitter {
  /** The peer of the window as the daemon knows it after its hello. */
  peer?: Peer;
  private socket?: Socket;
  private next = 0;
  private readonly waiting = new Map<
    number,
    { resolve(answer: Answer): void; reject(error: Error): void; asked: Error }
  >();
  private closing = false;
  private reaching?: Promise<void>;
  constructor(
    readonly folder = daemonFolder(),
    readonly place = daemonPlace(folder),
    /** The role that the hello of the window asks, none to hold the role that its transport proved. */
    readonly role?: Role,
  ) {
    super();
  }

  /** The daemon reached, or started and reached, and welcomed, once for each connection. */
  connect(): Promise<void> {
    if (this.socket && !this.reaching) return Promise.resolve();
    this.reaching ??= this.reach().finally(() => {
      this.reaching = undefined;
    });
    return this.reaching;
  }

  /** A daemon that closes before its welcome, as one that ends at its grace in the instant that this window dials it,
   * is dialed again once, which starts a new daemon when none listens. */
  private async reach(): Promise<void> {
    try {
      await this.reached();
    } catch (error) {
      if (!(error instanceof Closed)) throw error;
      await this.reached();
    }
  }

  private async reached(): Promise<void> {
    const socket = (await dialed(this.place)) ?? (await started(this.folder, this.place));
    this.socket = socket;
    let held = "";
    socket.setEncoding("utf8");
    socket.on("data", (chunk: string) => {
      held += chunk;
      for (let end = held.indexOf("\n"); end >= 0; end = held.indexOf("\n")) {
        const line = held.slice(0, end);
        held = held.slice(end + 1);
        if (line.trim()) this.heard(JSON.parse(line) as Frame);
      }
    });
    socket.on("error", () => {});
    socket.on("close", () => this.gone(socket));
    if (this.held === 0) socket.unref();
    const welcome = await this.ask({
      verb: "hello",
      protocol: PROTOCOL,
      kind: "window",
      role: this.role,
    }).catch((error: unknown) => {
      // A refusal of the version is told as a daemon of another build.
      if (!(error instanceof Refused) || error.kind !== "protocol") throw error;
      throw new Error(
        `The daemon of this machine is of another build (${error.message}). It ends once its windows quit and its grace passes, and the next window starts one of this build.`,
      );
    });
    if (welcome.answer !== "welcome") throw new Error("The daemon gave no welcome.");
    this.peer = welcome.data.peer;
  }

  private heard(frame: Frame): void {
    if (frame.frame === "reply" || frame.frame === "refused") {
      const waits = this.waiting.get(frame.id ?? -1);
      if (!waits) return;
      this.waiting.delete(frame.id ?? -1);
      if (frame.frame === "reply") waits.resolve(frame.answer);
      else {
        // A refusal comes on the socket, so it takes the stack of the verb that asked it, which names its caller.
        const refused = new Refused(frame.error.error, frame.why);
        refused.stack = `${refused.name}: ${refused.message}\n${waits.asked.stack?.split("\n").slice(1).join("\n") ?? ""}`;
        waits.reject(refused);
      }
    } else if (frame.frame === "event") this.emit("event", frame.life, frame.seq, frame.event);
    else if (frame.frame === "presence") this.emit("presence", frame.life, frame.peers);
    else if (frame.frame === "stalled") this.emit("stalled", frame.life, frame.notice);
    else if (frame.frame === "sign_in") this.emit("authentication", frame);
    else if (frame.frame === "ask") this.emit("ask", frame.life, frame.ask, frame.request);
  }

  /** The socket closed: each request that waits fails, and a connection that the window did not close dials again. */
  private gone(socket: Socket): void {
    if (this.socket !== socket) return;
    this.socket = undefined;
    for (const waits of this.waiting.values()) {
      const closed = new Closed("The daemon of this machine closed.");
      closed.stack = `${closed.name}: ${closed.message}\n${waits.asked.stack?.split("\n").slice(1).join("\n") ?? ""}`;
      waits.reject(closed);
    }
    this.waiting.clear();
    // A socket that closes before its welcome fails the reach that dialed it, which dials again.
    if (this.closing || this.reaching) return;
    this.connect().then(
      () => this.emit("reconnected"),
      (error: unknown) => this.emit("lost", error instanceof Error ? error.message : String(error)),
    );
  }

  /** The answer of the daemon to a verb. */
  async ask(verb: Verb): Promise<Answer> {
    if (verb.verb !== "hello") {
      if (this.reaching) await this.reaching;
      else if (!this.socket && !this.closing) await this.connect();
    }
    const socket = this.socket;
    if (!socket || this.closing) throw new Error("This window holds no connection to the daemon.");
    const id = this.next++;
    const asked = new Error(verb.verb);
    return new Promise<Answer>((resolve, reject) => {
      this.waiting.set(id, { resolve, reject, asked });
      socket.write(`${JSON.stringify({ id, ...verb } satisfies Request)}\n`);
    });
  }

  /** The records of the sessions of each workspace of the list in a config folder, as the daemon reads them, and each
   * record whose life the daemon holds. */
  async records(config: string): Promise<Listed[]> {
    const listed = await this.ask({ verb: "list", config });
    if (listed.answer !== "lives") throw new Error(`The daemon gave ${listed.answer}, and no list of lives.`);
    return listed.data;
  }

  /** The records whose life a window holds in the daemon now, of those given: a window of any connection, this one
   * too, and no pin. */
  async holding(lives: readonly string[]): Promise<Set<string>> {
    const listed = await this.ask({ verb: "list" });
    if (listed.answer !== "lives") throw new Error(`The daemon gave ${listed.answer}, and no list of lives.`);
    return new Set(
      listed.data.filter((one) => one.watched && lives.includes(one.life)).map((one) => one.life),
    );
  }

  /** The life of a record gone from the daemon, which ends it once no window and no pin held it for its grace. It is
   * refused at once while a window holds the life, and when the life lives longer than a window waits, as under a
   * pin. A window waits for that end before it takes the record. */
  async ended(life: string, patience = ENDING): Promise<void> {
    for (const until = Date.now() + patience; ; await Bun.sleep(20)) {
      const listed = await this.ask({ verb: "list" });
      if (listed.answer !== "lives")
        throw new Error(`The daemon gave ${listed.answer}, and no list of lives.`);
      if (!listed.data.some((one) => one.life === life && one.held)) return;
      if ((await this.holding([life])).size > 0) throw new Error(`Another window holds the life of ${life}.`);
      if (Date.now() > until)
        throw new Error(`The life of ${life} did not end in ${patience / 1000} s: a pin holds it.`);
    }
  }

  /** Whether the window closed the connection, so that it dials no more. */
  get closed(): boolean {
    return this.closing;
  }

  /** The connection closed by the window: it dials no more. */
  close(): void {
    this.closing = true;
    this.socket?.end();
  }

  /** How many lives of the window the connection holds. */
  private held = 0;
  hold(): void {
    this.held++;
    this.socket?.ref();
  }
  /** A life let go: with the last life that it held, the connection keeps the process of the window alive no more,
   * and stays open for the next life. */
  release(): void {
    this.held = Math.max(0, this.held - 1);
    if (this.held === 0) this.socket?.unref();
  }
}

type Request = { id: number } & Verb;

/** What hears an event of an emitter. */
export type Listener = Parameters<EventEmitter["on"]>[1];

/** The one connection of this window to the daemon of its machine, made at its first use. A cache of furb named
 * again, as a scenario of the tests names one of its own, takes a connection to the daemon of that cache. */
let shared: Daemon | undefined;
/** A role given makes this window a reader on any transport: its hello asks that role, and the daemon gives the
 * lesser of it and the role that the transport proved. */
export function daemon(role?: Role): Daemon {
  const folder = daemonFolder();
  if (shared?.folder !== folder || (role && shared.role !== role)) {
    shared?.close();
    shared = new Daemon(folder, daemonPlace(folder), role ?? shared?.role);
  }
  return shared;
}

/** A request of a model of a life, as the daemon asks the window that answers it. */
type Asked = { actor: string; chain: string; messages: unknown[] };

/** A request of a model that the window answers, as the demo answers it, and what the turn writes as it is written. */
export type Models = (
  asked: Asked,
  write: (part: { text?: string; thinking?: string }) => void,
) => Promise<unknown>;

/** What a window chooses of a life that its open makes, which a life that lives keeps as it is. */
export interface Choices {
  cwd?: string;
  actor?: string;
  roster?: string[];
  extensions?: boolean;
  /** The folder of the settings of the user and the claude command line that the window reads, which the life reads
   * too. */
  config?: string;
  claude?: string;
  /** The models of the life, which the window answers in their place, as the demo does. */
  models?: Models;
}

/** The verbs that a window sends again when the daemon went before it answered: each reads, or says what a second time
 * says again. */
const AGAIN = new Set<Verb["verb"]>([
  "peek",
  "get",
  "transcript",
  "turns",
  "program",
  "cwd",
  "standing",
  "filled",
  "names",
  "inspect",
  "pending",
  "queued",
  "seen",
  "queue",
  "pause",
  "wake",
]);

/** The value of an act done with a fault, as an error with the name of its class. */
function faulted(value: unknown): Error | undefined {
  const fault = value as { is?: unknown; args?: unknown } | null;
  if (!fault || typeof fault !== "object" || typeof fault.is !== "string" || !Array.isArray(fault.args))
    return undefined;
  if (!/(Error|Refused|Exception|Drift|Exit)$/.test(fault.is) || fault.is === "Exit") return undefined;
  const said = fault.args.map(String).join(" ");
  return new Error(said ? `${fault.is}: ${said}` : fault.is);
}

/** The act that a verb made, as the reply of the daemon names it. */
function named(answer: Answer): string {
  if (answer.answer !== "act") throw new Error(`The daemon gave ${answer.answer}, and no act.`);
  return answer.data;
}
function valued(answer: Answer): unknown {
  if (answer.answer === "value") return answer.data;
  if (answer.answer === "values") return answer.data;
  if (answer.answer === "done") return null;
  throw new Error(`The daemon gave ${answer.answer}, and no value.`);
}

/** One life of the daemon, as a window holds it: the named verbs that the window says to it, and each event of it as
 * the window hears it. The window holds each act as the daemon told it last, with its meaning, and reads nothing of the
 * meaning of an act from the facts. It tells `facts` after the facts of a change, `change` after a text, `log` at a
 * notice, and `ended` with why when the life ends; after a handover it attaches again, and tells `reset`, since the
 * life that the new daemon opened tells each fact again. */
export class RemoteLife extends EventEmitter {
  readonly root = "chain1";
  disposed = false;
  readonly facts: Fact[] = [];
  /** Each act as the daemon told it last. */
  readonly acts = new Map<string, Act>();
  /** The place up to which the operator read each chain or thread, by its name: the place of the last fact seen that
   * he said of it, which covers each change that the record holds before it. */
  readonly seenAt = new Map<string, number>();
  /** Each compaction of a chain, in the order of the record. */
  readonly compactions: Compaction[] = [];
  /** What each command that is not done printed on each stream so far, by the command. */
  readonly outputs = new Map<string, { stdout: string; stderr: string }>();
  /** How many acts of work are done, which grows as the daemon tells each end. */
  completed = 0;
  /** What the answers of the models cost each chain, by the chain, and all chains together, in dollars. */
  readonly spends = new Map<string, Spend>();
  cost = 0;
  /** How many times a spend changed, which tells a view when to read the spends again. */
  spendChanges = 0;
  /** How many changes of an act the window heard, and the count at the last change of each act that a person
   * follows, so that a reader takes only the acts that changed since it last read. */
  private changes = 0;
  private readonly changed = new Map<string, number>();
  /** Each change of an act in order, as the count of the change and the name of the act. An act that changed again
   * leaves its earlier entries, which the log drops once it holds twice as many entries as acts. */
  private log: { count: number[]; name: string[] } = { count: [], name: [] };
  /** The follow-ups that wait on each chain, in order, as the life told them last. */
  readonly queues = new Map<string, FollowUp[]>();
  /** What each rung that asked a model streams while the model writes, by that rung. */
  readonly streams = new Map<string, { chain: string; text: string; thinking: string }>();
  /** Why the life is not open in this window, and nothing while it is. */
  lost = "";
  private batch?: { fact: Fact; acts: Act[] };
  private readonly listening: [string, Listener][];
  constructor(
    readonly daemon: Daemon,
    readonly record: string,
    private readonly choices: Choices = {},
  ) {
    super();
    const event = (life: string, _seq: number, told: Event) => life === this.record && this.heard(told);
    const ask = (life: string, number: number, request: Asked) =>
      life === this.record && void this.answered(number, request);
    const again = () => {
      this.opening = undefined;
      void this.open().then(
        () => this.emit("reset"),
        (error: unknown) => this.lose(error),
      );
    };
    const lost = (why: string) => this.lose(new Error(why));
    const stalled = (life: string, notice: string) => life === this.record && this.emit("log", notice);
    const authentication = ({ state, url, message }: Extract<Frame, { frame: "sign_in" }>) =>
      this.emit("authentication", { state, url: url ?? undefined, message: message ?? undefined });
    this.listening = [
      ["event", event],
      ["ask", ask],
      ["stalled", stalled],
      ["authentication", authentication],
      ["reconnected", again],
      ["lost", lost],
    ];
    for (const [name, listener] of this.listening) daemon.on(name, listener);
  }

  /** The open that each verb waits for. */
  private opening?: Promise<void>;
  /** The life opened in the daemon, on the choices of the window when this open makes it, and attached, once the
   * daemon told the facts that the life opened on, or why it ended. */
  open(): Promise<void> {
    this.opening ??= this.opened();
    return this.opening;
  }
  private held = false;
  private async opened(): Promise<void> {
    if (!this.held) this.daemon.hold();
    this.held = true;
    await this.daemon.connect();
    const { cwd, actor, roster, extensions, config, claude, models } = this.choices;
    const told = new Promise<void>((resolve) => {
      const heard = () => {
        this.off("facts", heard);
        this.off("ended", heard);
        resolve();
      };
      this.on("facts", heard);
      this.on("ended", heard);
    });
    // The open attaches the window in the same step, from the start of the log, so the life has its holder before
    // the grace of the daemon passes, and the window lets go of what it held of an earlier birth.
    this.facts.length = 0;
    this.acts.clear();
    this.seenAt.clear();
    this.compactions.length = 0;
    this.outputs.clear();
    this.completed = 0;
    this.spends.clear();
    this.spendChanges++;
    this.cost = 0;
    this.changed.clear();
    this.log = { count: [], name: [] };
    this.streams.clear();
    this.queues.clear();
    this.batch = undefined;
    this.lost = "";
    const answers = Boolean(models);
    const life = this.record;
    try {
      await this.daemon.ask({
        verb: "open",
        life,
        cwd,
        actor,
        roster,
        extensions,
        config,
        claude,
        answers,
        attach: 0,
      });
    } catch (error) {
      if (error instanceof Closed) return await this.reopened(error);
      throw error;
    }
    await told;
    // A verb of a life waits for the open of the life to end, as its replay, and fails when the life ended there, as
    // on a drift: the window holds the life once a verb was answered, or knows why it ended.
    const ended = new Promise<void>((resolve) => this.once("ended", () => resolve()));
    await this.daemon
      .ask({ verb: "pending", life })
      .catch((error: unknown) =>
        error instanceof Closed ? this.reopened(error) : this.lost ? undefined : ended,
      );
    if (this.lost) throw new Error(this.lost);
  }

  /** The open that the connection makes again once the daemon that went before it answered this open is back, as
   * after a crash, a kill or a handover: the connection dials again, and its open stands for this one, which ends with
   * it. A window that closed the connection dials no more, so the open fails with the request that the daemon left. */
  private reopened(closed: Closed): Promise<void> {
    if (this.daemon.closed) return Promise.reject(closed);
    return new Promise<void>((resolve, reject) => {
      const heard = () => {
        this.off("reset", heard);
        this.off("ended", heard);
        if (this.lost) reject(new Error(this.lost));
        else resolve();
      };
      this.on("reset", heard);
      this.on("ended", heard);
    });
  }

  private lose(error: unknown): void {
    this.lost = error instanceof Error ? error.message : String(error);
    for (const heard of this.known.values()) for (const one of heard) one();
    this.known.clear();
    for (const waits of this.waits.values()) for (const one of waits) one.reject(new Error(this.lost));
    this.waits.clear();
    this.emit("ended", this.lost);
  }

  private heard(told: Event): void {
    switch (told.event) {
      case "fact":
        this.flush();
        this.batch = { fact: told.data as Fact, acts: [] };
        return;
      case "act":
        if (this.batch) this.batch.acts.push(told.data);
        else this.told([told.data]);
        return;
      case "text": {
        const { rung, chain, text, thinking } = told.data;
        const held = this.streams.get(rung) ?? { chain, text: "", thinking: "" };
        this.streams.set(rung, { chain, text: held.text + text, thinking: held.thinking + thinking });
        this.emit("change");
        return;
      }
      case "queued":
        this.queues.set(told.data.chain, told.data.waiting);
        this.emit("change");
        return;
      case "notice":
        this.emit("log", told.data);
        return;
      case "changed":
        this.flush();
        this.emit("facts");
        return;
      case "reborn":
        this.flush();
        if (told.data) this.emit("log", `The life opened again from its checkpoint after ${told.data}.`);
        return;
      case "ended":
        this.flush();
        this.emit("facts");
        this.lose(new Error(told.data ? `The life ended: ${told.data}` : "The life ended."));
        return;
    }
  }

  /** The fact that waits for its acts, with the acts that the daemon told with it, each in the state that the fact
   * left it. A fact says alone what the window keeps of it: a fact seen, a compaction, and the lines that a command
   * prints. */
  private flush(): void {
    const batch = this.batch;
    if (!batch) return;
    this.batch = undefined;
    const fact = batch.fact;
    this.facts.push(fact);
    const [kind, id, by] = fact;
    const at = this.facts.length;
    // A later life says each fact of its record again as the journal, which names no speaker, and only the operator
    // says seen. A chain says its own prefix at its birth, as a chain with a source does, and that prefix cuts nothing.
    if (kind === "seen" && (operated(by) || by === "journal")) this.seenAt.set(id, at);
    else if (kind === "prefix" && by !== id) this.compactions.push({ chain: id, by, at });
    else if (kind === "started" && this.outputs.has(id)) this.outputs.set(id, { stdout: "", stderr: "" });
    else if (kind === "out" && this.acts.get(id)?.kind === "bash" && !isDone(this.acts.get(id) as Act)) {
      const output = this.outputs.get(id) ?? { stdout: "", stderr: "" };
      if (fact[4] === "stderr") output.stderr += String(fact[3]);
      else output.stdout += String(fact[3]);
      this.outputs.set(id, output);
      this.mark(id);
    }
    this.told(batch.acts);
  }

  /** The acts that changed after a count of changes, each that a person follows, in the order they began, and the
   * count now. */
  since(count: number): { count: number; acts: Act[] } {
    const { log } = this;
    let low = 0;
    for (let high = log.count.length; low < high; ) {
      const middle = (low + high) >> 1;
      if ((log.count[middle] ?? 0) <= count) low = middle + 1;
      else high = middle;
    }
    const acts: Act[] = [];
    for (let at = low; at < log.count.length; at++) {
      const name = log.name[at] ?? "";
      const act = this.acts.get(name);
      if (act && this.changed.get(name) === log.count[at]) acts.push(act);
    }
    acts.sort((one, other) => one.began - other.began);
    return { count: this.changes, acts };
  }
  /** One more change of an act that a person follows. */
  private mark(id: string): void {
    this.changed.set(id, ++this.changes);
    this.log.count.push(this.changes);
    this.log.name.push(id);
    if (this.log.count.length <= 2 * this.changed.size + 64) return;
    const kept: { count: number[]; name: string[] } = { count: [], name: [] };
    for (const [at, name] of this.log.name.entries())
      if (this.changed.get(name) === this.log.count[at]) {
        kept.count.push(this.log.count[at] ?? 0);
        kept.name.push(name);
      }
    this.log = kept;
  }
  /** An act as the daemon told it last. */
  act(id: string): Act | undefined {
    return this.acts.get(id);
  }
  /** What a command that is not done printed so far on each stream. */
  output(id: string): { stdout: string; stderr: string } | undefined {
    return this.outputs.get(id);
  }
  /** The chain that an act stands in, and nothing for a name of no act. */
  scope(id: string): string {
    return this.acts.get(id)?.chain ?? "";
  }

  /** The acts that the daemon told with a fact: a verb that waits for an act, or for its end, reads the acts as they
   * stand after them. */
  private told(acts: Act[]): void {
    for (const act of acts) {
      const was = this.acts.get(act.id);
      this.acts.set(act.id, act);
      if (followed(act.kind)) this.mark(act.id);
      if (act.kind === "bash" && !isDone(act) && !this.outputs.has(act.id))
        this.outputs.set(act.id, { stdout: "", stderr: "" });
      if (act.kind === "chain" && act.spend && act.spend !== was?.spend) {
        this.cost += act.spend.dollars - (this.spends.get(act.id)?.dollars ?? 0);
        this.spends.set(act.id, act.spend);
        this.spendChanges++;
      }
      if (isDone(act) && !(was && isDone(was))) {
        this.outputs.delete(act.id);
        if (WORK.includes(act.kind)) this.completed++;
      }
      for (const heard of this.known.get(act.id) ?? []) heard();
      this.known.delete(act.id);
      if (act.phase.phase !== "done") continue;
      if (act.kind === "reply") this.streams.delete(act.by);
      this.settled(act.id, act.phase.value);
    }
  }

  /** The verbs that wait for the first fact of the act that they made, by the act. */
  private readonly known = new Map<string, (() => void)[]>();
  /** The act that a verb made, once the window holds it: the daemon replies to the verb before it
   * tells the facts that the verb said, and the window reads its own acts. */
  private async made(answer: Answer): Promise<string> {
    const id = named(answer);
    // The daemon may tell the facts of the act before its reply to the verb comes, and the acts that it told stand in
    // the acts of the window: the wait is for an act unheard.
    if (this.acts.has(id) || this.lost) return id;
    await new Promise<void>((resolve) => {
      const heard = this.known.get(id) ?? [];
      heard.push(resolve);
      this.known.set(id, heard);
    });
    return id;
  }

  /** The waits on the end of each act. */
  private readonly waits = new Map<string, { resolve(value: unknown): void; reject(error: Error): void }[]>();
  private settled(id: string, value: unknown): void {
    const waits = this.waits.get(id);
    if (!waits) return;
    this.waits.delete(id);
    const fault = faulted(value);
    for (const one of waits) fault ? one.reject(fault) : one.resolve(value);
  }

  /** A turn that the window gives for a request of a model of its life. */
  private async answered(ask: number, request: Asked): Promise<void> {
    const models = this.choices.models;
    if (!models) return;
    const life = this.record;
    const write = (part: { text?: string; thinking?: string }) =>
      void this.daemon
        .ask({ verb: "wrote", life, ask, text: part.text ?? "", thinking: part.thinking ?? "" })
        .catch(() => {});
    try {
      const turn = await models(request, write);
      await this.daemon.ask({ verb: "answered", life, ask, turn, why: null });
    } catch (error) {
      const why = error instanceof Error ? error.message : String(error);
      await this.daemon.ask({ verb: "answered", life, ask, turn: null, why }).catch(() => {});
    }
  }

  private async verb(verb: Verb): Promise<Answer> {
    if (this.disposed) throw new Error("The session is disposed.");
    await this.open();
    // A life that ended takes no verb, until the window opens it again.
    if (this.lost) throw new Error(this.lost);
    try {
      return await this.daemon.ask(verb);
    } catch (error) {
      // A verb that the daemon left when it went goes again once the life is open again, if a second time changes
      // nothing: a verb that makes an act may have made it before the daemon went, and fails with the request.
      if (!(error instanceof Closed) || !AGAIN.has(verb.verb)) throw error;
      await this.reopened(error);
      return await this.daemon.ask(verb);
    }
  }

  async prompt(
    shape: string,
    { message = "", on, to }: { message?: string; on?: string; to?: string } = {},
  ): Promise<string> {
    return await this.made(await this.verb({ verb: "prompt", life: this.record, shape, message, to, on }));
  }
  async rung({ word, on }: { word: string; on?: string }): Promise<string> {
    return await this.made(await this.verb({ verb: "run", life: this.record, word, on }));
  }
  /** A chain under a label, or a fork of the chain `source`, with its transcript and its program. */
  async chain({ label, source, on }: { label?: string; source?: string; on?: string } = {}): Promise<string> {
    return await this.made(await this.verb({ verb: "chain", life: this.record, label, source, on }));
  }
  /** A command on a chain, with the text of its stdin, or with its stdin open for a feed when it is null. */
  async bash(command: string, { stdin, on }: { stdin?: string | null; on?: string } = {}): Promise<string> {
    const feed = stdin === null;
    return await this.made(
      await this.verb({ verb: "bash", life: this.record, command, stdin: stdin ?? undefined, feed, on }),
    );
  }
  async grant({ usd, share, on }: { usd?: number; share?: number; on?: string } = {}): Promise<string> {
    return await this.made(await this.verb({ verb: "grant", life: this.record, usd, share, on }));
  }
  async stand(): Promise<string> {
    return await this.made(await this.verb({ verb: "stand", life: this.record }));
  }
  /** What an act came to once it is done, which throws the fault that ended it. */
  async config(config: Config): Promise<string> {
    return await this.made(await this.verb({ verb: "config", life: this.record, config }));
  }
  /** A cancel of the sign-in of codex that the daemon runs for the life, which refuses the config that asked it. */
  async cancelSignIn(): Promise<void> {
    await this.daemon.ask({ verb: "cancel_sign_in", life: this.record });
  }
  /** Each key of the configs of the folder of the life, as the host joins them with their schema. */
  async settings(): Promise<Setting[]> {
    return valued(
      await this.verb({ verb: "config", life: this.record, config: { action: "get" } }),
    ) as Setting[];
  }
  async result(id: string): Promise<unknown> {
    const act = this.acts.get(id);
    if (act?.phase.phase === "done") {
      const fault = faulted(act.phase.value);
      if (fault) throw fault;
      return act.phase.value;
    }
    const done = new Promise<unknown>((resolve, reject) => {
      const waits = this.waits.get(id) ?? [];
      waits.push({ resolve, reject });
      this.waits.set(id, waits);
    });
    // The end of the window can come before the read of the act, and the caller then hears it from the await below.
    done.catch(() => {});
    if (!act && (await this.get(id)) === null) {
      this.waits.delete(id);
      throw new Error(`No act is named ${id}.`);
    }
    return await done;
  }
  async outcome(id: string): Promise<{ done: boolean; value: unknown }> {
    const act = this.acts.get(id);
    if (act)
      return { done: act.phase.phase === "done", value: act.phase.phase === "done" ? act.phase.value : null };
    try {
      return { done: true, value: await this.peek(id) };
    } catch {
      return { done: false, value: null };
    }
  }
  async peek(id: string): Promise<unknown> {
    return valued(await this.verb({ verb: "peek", life: this.record, act: id }));
  }
  async get(id: string): Promise<unknown> {
    return valued(await this.verb({ verb: "get", life: this.record, act: id }));
  }
  /** A control of an act, once the window heard its fact: the daemon may reply to the verb before it
   * tells that fact, and the window reads the act after the verb as the control left it. */
  private async control(
    verb: Verb & { verb: "pause" | "wake" | "cancel" | "close"; act: string },
  ): Promise<void> {
    const start = this.facts.length;
    await this.verb(verb);
    const heard = () =>
      Boolean(this.lost) ||
      this.facts.slice(start).some(([kind, about]) => kind === verb.verb && about === verb.act);
    while (!heard() && this.facts.length >= start)
      await new Promise<void>((resolve) => {
        const told = () => {
          for (const name of ["facts", "ended", "reset"]) this.off(name, told);
          resolve();
        };
        for (const name of ["facts", "ended", "reset"]) this.on(name, told);
      });
  }
  async pause(id: string, { why }: { why?: string } = {}): Promise<void> {
    await this.control({ verb: "pause", life: this.record, act: id, why });
  }
  async wake(id: string): Promise<void> {
    await this.control({ verb: "wake", life: this.record, act: id });
  }
  async cancel(id: string, { why }: { why?: string } = {}): Promise<void> {
    await this.control({ verb: "cancel", life: this.record, act: id, why });
  }
  /** The close of an act with a value written as the record writes a value. */
  async close(value: string, { id }: { id: string }): Promise<void> {
    await this.control({ verb: "close", life: this.record, act: id, value });
  }
  async answer(id: string, line: string): Promise<void> {
    await this.verb({ verb: "answer", life: this.record, act: id, line });
  }
  async read(path: string, { on }: { on?: string } = {}): Promise<{ path: string; content: string }> {
    return valued(await this.verb({ verb: "read", life: this.record, path, on })) as {
      path: string;
      content: string;
    };
  }
  async write(
    { path, content }: { path: string; content: string },
    { on }: { on?: string } = {},
  ): Promise<void> {
    await this.verb({ verb: "write", life: this.record, path, content, on });
  }
  async transcript({ on, of }: { on?: string; of?: string } = {}): Promise<Fact[]> {
    return valued(await this.verb({ verb: "transcript", life: this.record, on, of })) as Fact[];
  }
  async turns({ on }: { on?: string } = {}): Promise<Turn[]> {
    return valued(await this.verb({ verb: "turns", life: this.record, on })) as Turn[];
  }
  async program({ on }: { on?: string } = {}): Promise<Record<string, string>> {
    return valued(await this.verb({ verb: "program", life: this.record, on })) as Record<string, string>;
  }
  async cwd({ on }: { on?: string } = {}): Promise<string> {
    return String(valued(await this.verb({ verb: "cwd", life: this.record, on })));
  }
  async standing(): Promise<Standing> {
    return valued(await this.verb({ verb: "standing", life: this.record })) as Standing;
  }
  /** The share of the window of its model that an answer filled, as filled of the engine gives it. */
  async filled(act: string): Promise<number | undefined> {
    const got = valued(await this.verb({ verb: "filled", life: this.record, act }));
    return typeof got === "number" ? got : undefined;
  }
  async names(chain?: string): Promise<string[]> {
    return valued(await this.verb({ verb: "names", life: this.record, on: chain })) as string[];
  }
  async inspect(
    name: string,
    chain?: string,
  ): Promise<{ kind: string; representation: string; value: unknown }> {
    const got = await this.verb({ verb: "inspect", life: this.record, name, on: chain });
    if (got.answer !== "inspected") throw new Error(`The daemon gave ${got.answer}, and no name.`);
    return { kind: got.data.kind, representation: got.data.repr, value: got.data.value };
  }
  async pending(): Promise<Map<string, string>> {
    const got = valued(await this.verb({ verb: "pending", life: this.record })) as [string, string][];
    return new Map(got);
  }
  async seen(about: string): Promise<void> {
    await this.verb({ verb: "seen", life: this.record, about });
  }
  /** A follow-up put in the queue of a chain, which the life sends once the chain rests. */
  async queue(entry: string, shape: string, message: string, on: string): Promise<void> {
    await this.verb({ verb: "queue", life: this.record, entry, shape, message, on });
  }
  /** A follow-up taken out of the queue of its chain before the life sent it. */
  async unqueue(entry: string): Promise<void> {
    await this.verb({ verb: "unqueue", life: this.record, entry });
  }
  async pin(holder: string): Promise<void> {
    await this.verb({ verb: "pin", life: this.record, holder });
  }
  async unpin(holder: string): Promise<void> {
    await this.verb({ verb: "unpin", life: this.record, holder });
  }
  /** The window lets the life go: it hears no event of it more, and the life ends with its last holder. */
  async detach(): Promise<void> {
    if (this.disposed) return;
    this.disposed = true;
    for (const [name, listener] of this.listening) this.daemon.off(name, listener);
    for (const waits of this.waits.values())
      for (const one of waits) one.reject(new Error("The session is disposed."));
    this.waits.clear();
    await this.daemon.ask({ verb: "detach", life: this.record }).catch(() => {});
    if (this.held) this.daemon.release();
    this.held = false;
  }
}
