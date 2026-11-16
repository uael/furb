import { EventEmitter } from "node:events";
import { createConnection, type Socket } from "node:net";
import { Duplex, type Readable, type Writable } from "node:stream";
import type { Answer, Frame, Kind, Peer, Role, Request1 as Verb } from "./protocol.js";
import { PROTOCOL } from "./protocol.js";

/** The variables of the environment that a life gives the process of a kit: the record of the life that the process
 * serves, and the name of its kit. The life speaks to the process on its stdin and its stdout. */
export const LIFE = "FURB_LIFE";
export const SKILL = "FURB_SKILL";

/** What a connection reads its frames from and writes its verbs to: a socket to the daemon, or the stdin and the
 * stdout of the process of a kit, which hold no ref of the process. */
export type Link = Pick<Socket, "write" | "on" | "setEncoding" | "end" | "destroyed"> &
  Partial<Pick<Socket, "ref" | "unref">>;

/** The link of a stream to read and a stream to write, as the stdin and the stdout of a process: it closes when the
 * stream to read ends. */
export function linked(input: Readable, output: Writable): Link {
  const link = new Duplex({
    read() {},
    write(chunk, encoding, done) {
      output.write(chunk, encoding, done);
    },
    final(done) {
      output.end(done);
    },
  });
  input.on("data", (chunk) => link.push(chunk));
  input.on("end", () => link.push(null));
  return link as unknown as Link;
}

/** A refusal of the daemon, named by its kind, as `closed`, `role` or `no_skill`. Its name is its kind, and
 * `Refused` for the kind that says no more. */
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
 * build. */
export class Closed extends Error {
  override name = "Closed";
}

/** An error that comes on the socket, with the stack of the verb that asked it, which names its caller. */
function askedBy<E extends Error>(error: E, asked: Error): E {
  error.stack = `${error.name}: ${error.message}\n${asked.stack?.split("\n").slice(1).join("\n") ?? ""}`;
  return error;
}

/** How long a dial to the address of a machine waits for it, in milliseconds. */
const ADDRESS_PATIENCE = 5000;
/** After how long a quiet connection to a machine is probed, in milliseconds. */
const KEEP_ALIVE = 10000;

/** The host and the port of a place that names a machine, as `100.64.0.2:7431` or `[fd7a::1]:7431`, and nothing for a
 * place that is a socket or a pipe of this machine: the one dial reaches a daemon by either. */
export function addressOf(place: string): { host: string; port: number } | undefined {
  const found = /^(?:\[([0-9a-fA-F:.]+)\]|([A-Za-z0-9][A-Za-z0-9.-]*)):(\d{1,5})$/.exec(place);
  const port = Number(found?.[3]);
  return found && port > 0 && port < 65536 ? { host: found[1] ?? found[2] ?? "", port } : undefined;
}

/** A socket to the daemon at a place, or nothing when no daemon listens there: the one dial of a window, a skill and
 * each script. */
export function dialed(place: string): Promise<Socket | undefined> {
  return new Promise((resolve) => {
    const at = addressOf(place);
    const socket = at ? createConnection(at) : createConnection(place);
    // A machine that is off gives no answer, where a socket of this machine fails at once: a dial to an address gives
    // up after a few seconds, and a connection to it that goes quiet is noticed by a probe of the transport.
    if (at)
      socket.setTimeout(ADDRESS_PATIENCE, () => socket.destroy(new Error("The machine did not answer.")));
    socket.once("connect", () => {
      socket.removeAllListeners("error");
      if (at) {
        socket.setTimeout(0);
        socket.setKeepAlive(true, KEEP_ALIVE);
      }
      resolve(socket);
    });
    socket.once("error", () => resolve(undefined));
  });
}

/** A socket to the daemon at a place, which fails when none listens there. */
async function dialing(place: string): Promise<Socket> {
  const socket = await dialed(place);
  if (!socket) throw new Error(`No daemon of furb listens at ${place}.`);
  return socket;
}

/** Where a connection dials and who it says it is: the place, the kind and name of its peer, the role that its hello
 * asks (none to hold the role that its transport proved), and how it gets a socket when the place alone is not
 * enough, as a window that starts a daemon where none listens. */
interface Route {
  place: string;
  kind: Kind;
  name?: string;
  role?: Role;
  reach?: () => Promise<Socket>;
  /** A listener of each frame that the daemon pushes, added before the first hello so that no frame is missed. */
  heard?: (frame: Frame) => void;
  /** Says in the words of its caller why a hello failed, as a daemon of another build. */
  explain?: (error: unknown) => unknown;
}

/** How a connection is dialed beside its route: whether it is idle, which keeps the process alive only while a life
 * holds it, and whether it dials again when the daemon goes. */
interface Way extends Pick<Route, "role" | "reach" | "heard" | "explain"> {
  idle?: boolean;
  again?: boolean;
}

/** One connection to the daemon, after its hello. Each verb takes a number and its answer, in any order, and each
 * frame that the daemon pushes goes to each listener of `pushed`. `closed` settles when the stream ends for good, from
 * either side, and each verb that waits when the socket ends fails. A connection dialed with `again` dials the place
 * again when the daemon goes, as at a handover, and tells `reconnected` once the daemon in its place welcomed it, or
 * `lost` with why it did not. */
export class Connection extends EventEmitter {
  /** The peer as the daemon knows it after the hello. */
  peer!: Peer;
  /** What the welcome gave: the folders of the life of a kit, and each verb that it takes past the first protocol of a
   * kit. */
  welcome!: Welcome;
  readonly closed: Promise<void>;
  private socket!: Link;
  private next = 0;
  private readonly waiting = new Map<
    number,
    { resolve(answer: Answer): void; reject(error: Error): void; asked: Error }
  >();
  private readonly frameListeners = new Set<(frame: Frame) => void>();
  /** How many lives hold the connection, which an idle connection keeps the process alive for. */
  private held = 0;
  private ending = false;
  /** Whether the first hello is over: a socket that closes before it fails the dial and is not dialed again here. */
  private ready = false;
  private reaching?: Promise<void>;
  private finish!: () => void;
  private constructor(
    socket: Link,
    private readonly idle: boolean,
    private readonly route: Route,
    private readonly again: boolean,
  ) {
    super();
    if (route.heard) this.frameListeners.add(route.heard);
    this.closed = new Promise((resolve) => {
      this.finish = resolve;
    });
    this.attach(socket);
  }

  private attach(socket: Link): void {
    this.socket = socket;
    if (this.idle && this.held === 0) socket.unref?.();
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
    socket.on("close", () => {
      if (this.socket !== socket) return;
      for (const waits of this.waiting.values())
        waits.reject(askedBy(new Closed("The daemon closed the connection."), waits.asked));
      this.waiting.clear();
      if (this.reaching) return;
      if (this.ending || !this.again || !this.ready) return this.finish();
      // A daemon that closes before its welcome, as one that ends at its grace in the instant that this dials it, is
      // dialed once more.
      this.reaching = this.hello()
        .catch((error: unknown) => (error instanceof Closed ? this.hello() : Promise.reject(error)))
        .then(
          () => {
            this.reaching = undefined;
            this.emit("reconnected");
          },
          (error: unknown) => {
            this.reaching = undefined;
            this.ending = true;
            const why = this.route.explain?.(error) ?? error;
            this.emit("lost", why instanceof Error ? why.message : String(why));
            this.finish();
          },
        );
    });
    // An error of the stream closes it, which `closed` tells.
    socket.on("error", () => {});
  }

  /** The hello on a socket to the place, which makes it the socket of the connection. */
  private async hello(socket?: Link): Promise<void> {
    if (!socket) this.attach(await (this.route.reach?.() ?? dialing(this.route.place)));
    const { kind, name, role } = this.route;
    const welcome = await this.send({ verb: "hello", protocol: PROTOCOL, kind, name, role });
    if (welcome.answer !== "welcome") throw new Error(`The daemon gave ${welcome.answer}, and no welcome.`);
    this.welcome = welcome.data;
    this.peer = welcome.data.peer;
  }

  /** A connection over a link that is open already, as the stdin and the stdout of the process of a kit, after its
   * hello as a peer of a kind, with the name of a kit. */
  static async over(link: Link, kind: Kind, name?: string): Promise<Connection> {
    const connection = new Connection(link, false, { place: "", kind, name }, false);
    await connection.hello(link);
    connection.ready = true;
    return connection;
  }

  /** A connection to the daemon at a place, as a peer of a kind, with the name of a skill for a skill. */
  static async dial(place: string, kind: Kind, name?: string, way: Way = {}): Promise<Connection> {
    const { idle = false, again = false, ...via } = way;
    // A daemon that closes before its welcome, as one that ends at its grace in the instant that this dials it, is
    // dialed once more, which starts a new daemon when none listens.
    for (let attempt = 0; ; attempt++) {
      const socket = await (via.reach?.() ?? dialing(place));
      const connection = new Connection(socket, idle, { place, kind, name, ...via }, again);
      try {
        await connection.hello(socket);
      } catch (error) {
        if (error instanceof Closed && attempt === 0) continue;
        throw via.explain?.(error) ?? error;
      }
      connection.ready = true;
      return connection;
    }
  }

  /** The answer of the daemon to a verb, or its refusal as a `Refused`. A verb waits for the dial that follows a
   * handover. */
  async ask(verb: Verb): Promise<Answer> {
    const asked = new Error(verb.verb);
    if (this.reaching) await this.reaching;
    return this.send(verb, asked);
  }

  private send(verb: Verb, asked = new Error(verb.verb)): Promise<Answer> {
    const id = this.next++;
    return new Promise<Answer>((resolve, reject) => {
      if (this.ending || this.socket.destroyed) {
        reject(askedBy(new Closed("This connection holds no socket to the daemon."), asked));
        return;
      }
      this.waiting.set(id, { resolve, reject, asked });
      this.socket.write(`${JSON.stringify({ id, ...verb })}\n`);
    });
  }

  /** Each frame that the daemon pushes from now on, heard until the call that this gives. */
  pushed(listener: (frame: Frame) => void): () => void {
    this.frameListeners.add(listener);
    return () => this.frameListeners.delete(listener);
  }

  /** A life that the connection serves. An idle connection, as the one of a window, keeps the process alive only
   * while a life holds it, and a connection that is not idle keeps it alive always. */
  hold(): void {
    this.held++;
    this.socket.ref?.();
  }

  /** A life let go: with the last life that it held, an idle connection keeps the process alive no more and stays open
   * for the next life. */
  release(): void {
    this.held = Math.max(0, this.held - 1);
    if (this.idle && this.held === 0) this.socket.unref?.();
  }

  close(): void {
    this.ending = true;
    this.socket.end();
  }

  private heard(frame: Frame): void {
    if (frame.frame === "reply" || frame.frame === "refused") {
      const waits = this.waiting.get(frame.id ?? -1);
      if (!waits) return;
      this.waiting.delete(frame.id ?? -1);
      if (frame.frame === "reply") waits.resolve(frame.answer);
      else waits.reject(askedBy(new Refused(frame.error.error, frame.why), waits.asked));
      return;
    }
    for (const listener of this.frameListeners) listener(frame);
  }
}

/** A fact of the life as the daemon tells it: its kind, the act that it is about, who said it, and its words. */
export type Fact = [kind: string, about: string, by: string, ...words: unknown[]];

/** A verb that the skill says to its life, with no `life`, which the skill fills in. */
export type Said = Verb extends infer One
  ? One extends { life: string }
    ? Omit<One, "life">
    : never
  : never;

/** What the process of a kit asks of its life: the kinds of facts that it hears, each kind when it names none, and
 * where it stands, which the environment that the life gave it holds unless a test names it, and the link to its life,
 * its stdin and its stdout unless a test names another. */
export interface SkillOptions {
  kinds?: readonly string[];
  /** The place in the life that the kit hears from: the start unless it names one. */
  from?: number;
  env?: Record<string, string | undefined>;
  link?: Link;
  /** The answer of each kind of act that the kit answers, as `answers` takes it, made from the welcome of the life.
   * The life gives the acts that wait for the kit as soon as it attaches, so a kit answers each of them only when its
   * answers stand before it attaches. */
  answering?: (welcome: Welcome) => Record<string, (fact: Fact, signal: AbortSignal) => unknown>;
}

/** The process of a kit as a peer of its life: it attaches the life under the name of its kit, hears the facts of the
 * kinds that it asked, answers the acts of its kinds, and says verbs to the life, which the life takes. A kit draws
 * nothing: its word draws its views. */
export class Skill {
  private readonly facts = new Set<(fact: Fact, seq: number) => void>();
  private readonly answering = new Map<string, (fact: Fact, signal: AbortSignal) => unknown>();
  private readonly running = new Map<string, AbortController>();
  private constructor(
    readonly connection: Connection,
    readonly name: string,
    readonly life: string,
    private readonly kinds: ReadonlySet<string> | undefined,
  ) {
    connection.pushed((frame) => this.pushed(frame));
  }

  /** The kit of this process attached to its life, from the variables that the life gave it. */
  static async attach(options: SkillOptions = {}): Promise<Skill> {
    const env = options.env ?? process.env;
    const [life, name] = [LIFE, SKILL].map((one) => env[one]);
    if (life === undefined || !name)
      throw new Error(`A process of a kit needs ${LIFE} and ${SKILL}, which its life gives it.`);
    const link = options.link ?? linked(process.stdin, process.stdout);
    const connection = await Connection.over(link, "skill", name);
    const kinds = options.kinds ? new Set(options.kinds) : undefined;
    const skill = new Skill(connection, name, life, kinds);
    for (const [kind, answer] of Object.entries(options.answering?.(connection.welcome) ?? {}))
      skill.answers(kind, answer);
    await connection.ask({ verb: "attach", life, from: options.from ?? 0 });
    return skill;
  }

  /** Settles when the link to the life ends, as at the end of the life. */
  get closed(): Promise<void> {
    return this.connection.closed;
  }

  /** Each fact of a kind that the skill asked, in the order of the log, from now on. */
  heard(listener: (fact: Fact, seq: number) => void): () => void {
    this.facts.add(listener);
    return () => this.facts.delete(listener);
  }

  /** Each act of a kind that the life gives the kit, and that is not done, answered from now on, whatever kinds the
   * kit hears: the kit closes it with what `answer` gives for its fact and the signal that the done of the act aborts,
   * awaited when it is a promise, or with a refusal that names what it threw. An act that is done already gets no
   * answer, and one whose done comes first gets none either. */
  answers(kind: string, answer: (fact: Fact, signal: AbortSignal) => unknown): () => void {
    this.answering.set(kind, answer);
    return () => this.answering.delete(kind);
  }

  private async answered(fact: Fact, answer: (fact: Fact, signal: AbortSignal) => unknown): Promise<void> {
    const act = fact[1];
    const aborted = new AbortController();
    this.running.set(act, aborted);
    try {
      const open = await this.say({ verb: "peek", act }).then(
        () => false,
        (no: unknown) => no instanceof Refused,
      );
      if (!open) return;
      let value: string;
      try {
        value = JSON.stringify(await answer(fact, aborted.signal));
      } catch (no) {
        value = JSON.stringify({ is: "Refused", args: [no instanceof Error ? no.message : String(no)] });
      }
      if (!aborted.signal.aborted) await this.say({ verb: "close", act, value });
    } finally {
      this.running.delete(act);
    }
  }

  /** The answer of the life to a verb that the kit says. */
  say(verb: Said): Promise<Answer> {
    return this.connection.ask({ ...verb, life: this.life } as Verb);
  }

  /** A fact said to the life of its own accord, of a kind, about a chain or an act, with words of JSON, which a life
   * that names the capability `fact` in its welcome takes. */
  async fact(kind: string, about: string, ...words: unknown[]): Promise<void> {
    if (!this.connection.welcome.capabilities?.includes("fact"))
      throw new Error("This life takes no fact from a kit: it is older than the capability fact.");
    await this.say({ verb: "fact", kind, about, words: words.map((one) => JSON.stringify(one)) });
  }

  /** The value of each key of the kit, as the configs give it, else the default of its file of keys, which a life
   * that names the capability `config` in its welcome gives. The life says a fact `configured` at each change of a key
   * through the configs of its window, with the key and the value that wins for it, which a kit hears. */
  async settings(): Promise<Record<string, unknown>> {
    if (!this.connection.welcome.capabilities?.includes("config"))
      throw new Error("This life gives no config to a kit: it is older than the capability config.");
    const answer = await this.say({ verb: "config", config: { action: "get" } });
    const values = answer.answer === "values" ? answer.data : [];
    const keys = values.filter((one) => (one as { skill?: string }).skill === this.name) as {
      key: string[];
      value: unknown;
    }[];
    return Object.fromEntries(keys.map((one) => [one.key[1], one.value]));
  }

  /** A word run on a chain, the root unless it names one: the name of its rung. */
  async run(word: string, on?: string): Promise<string> {
    return named(await this.say({ verb: "run", word, on }));
  }

  /** A prompt on a chain, of a shape, with a message, to an actor: the name of the prompt. */
  async prompt(
    shape: string,
    message: string,
    { to, on }: { to?: string; on?: string } = {},
  ): Promise<string> {
    return named(await this.say({ verb: "prompt", shape, message, to, on }));
  }

  close(): void {
    this.connection.close();
  }

  private pushed(frame: Frame): void {
    if (frame.frame !== "event" || frame.life !== this.life || frame.event.event !== "fact") return;
    const fact = frame.event.data as Fact;
    const answer = this.answering.get(fact[0]);
    if (answer) void this.answered(fact, answer);
    else if (fact[0] === "done") this.running.get(fact[1])?.abort();
    if (this.kinds && !this.kinds.has(fact[0])) return;
    for (const listener of this.facts) listener(fact, frame.seq);
  }
}

/** What a welcome gives. */
type Welcome = Extract<Answer, { answer: "welcome" }>["data"];

/** The act that a verb made, as the reply of the life names it. */
function named(answer: Answer): string {
  if (answer.answer !== "act") throw new Error(`The life gave ${answer.answer}, and no act.`);
  return answer.data;
}
