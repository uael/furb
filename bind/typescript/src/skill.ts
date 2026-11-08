import { EventEmitter } from "node:events";
import { createConnection, type Socket } from "node:net";
import { join } from "node:path";
import type {
  Answer,
  Effect,
  Frame,
  Kind,
  Mount,
  Node,
  Peer,
  Slot,
  Request1 as Verb,
  ViewEvent,
} from "./protocol.js";
import { PROTOCOL } from "./protocol.js";

/** The variables of the environment that the daemon gives the process of a skill: the folder where it listens, the
 * record of the life that the process serves, and the name of its skill. */
export const HOST = "FURB_HOST";
export const LIFE = "FURB_LIFE";
export const SKILL = "FURB_SKILL";

/** Where a peer dials the daemon that listens in a folder: its socket, or its named pipe on Windows. */
export function daemonPlace(folder: string): string {
  return process.platform === "win32"
    ? `\\\\.\\pipe\\furb-${folder.replace(/[\\/:]/g, "-")}`
    : join(folder, "host.sock");
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

/** A socket to the daemon at a place, which fails when none listens there. */
function dialing(place: string): Promise<Socket> {
  return new Promise<Socket>((resolve, reject) => {
    const socket = createConnection(place);
    socket.once("connect", () => {
      socket.removeListener("error", reject);
      resolve(socket);
    });
    socket.once("error", reject);
  });
}

/** One connection to the daemon, after its hello. Each verb takes a number and its answer, in any order, and each
 * frame that the daemon pushes goes to each listener of `pushed`. `closed` settles when the stream ends for good, from
 * either side, and each verb that waits when the socket ends fails. A connection dialed with `again` dials the place
 * again when the daemon goes, as at a handover, and tells `reconnected` once the daemon in its place welcomed it, or
 * `lost` with why it did not. */
export class Connection extends EventEmitter {
  /** The peer as the daemon knows it after the hello. */
  peer!: Peer;
  readonly closed: Promise<void>;
  private socket!: Socket;
  private next = 0;
  private readonly waiting = new Map<
    number,
    { resolve(answer: Answer): void; reject(error: Error): void; asked: Error }
  >();
  private readonly frameListeners = new Set<(frame: Frame) => void>();
  /** How many lives hold the connection, which an idle connection keeps the process alive for. */
  private held = 0;
  private ending = false;
  private reaching?: Promise<void>;
  private finish!: () => void;
  private constructor(
    socket: Socket,
    private readonly idle: boolean,
    private readonly route?: { place: string; kind: Kind; name?: string },
  ) {
    super();
    this.closed = new Promise((resolve) => {
      this.finish = resolve;
    });
    this.attach(socket);
  }

  private attach(socket: Socket): void {
    this.socket = socket;
    if (this.idle && this.held === 0) socket.unref();
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
      if (this.ending || !this.route || this.reaching) return this.finish();
      this.reaching = this.hello(this.route.place).then(
        () => {
          this.reaching = undefined;
          this.emit("reconnected");
        },
        (error: unknown) => {
          this.reaching = undefined;
          this.emit("lost", error instanceof Error ? error.message : String(error));
          this.finish();
        },
      );
    });
    // An error of the stream closes it, which `closed` tells.
    socket.on("error", () => {});
  }

  /** The hello on a socket to the place, which makes it the socket of the connection. */
  private async hello(place: string, socket?: Socket): Promise<void> {
    if (!socket) this.attach(await dialing(place));
    const { kind, name } = this.route ?? { kind: "skill" as Kind };
    const welcome = await this.send({ verb: "hello", protocol: PROTOCOL, kind, name });
    if (welcome.answer !== "welcome") throw new Error(`The daemon gave ${welcome.answer}, and no welcome.`);
    this.peer = welcome.data.peer;
  }

  /** A connection to the daemon at a place, as a peer of a kind, with the name of a skill for a skill. */
  static async dial(
    place: string,
    kind: Kind,
    name?: string,
    idle = false,
    again = false,
  ): Promise<Connection> {
    const socket = await dialing(place);
    const connection = new Connection(socket, idle, { place, kind, name });
    if (!again) Reflect.set(connection, "route", undefined);
    await connection.hello(place, socket);
    return connection;
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
    this.socket.ref();
  }

  /** A life let go: with the last life that it held, an idle connection keeps the process alive no more and stays open
   * for the next life. */
  release(): void {
    this.held = Math.max(0, this.held - 1);
    if (this.idle && this.held === 0) this.socket.unref();
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

/** A view of the skill that a window mounted: its viewer and its slot. `tree` serves its tree, each in the place of
 * the one before, and `effect` asks its one window to do something. What a person does to its nodes goes to each
 * listener of `events`, and `disposed` settles when the window or the daemon ends it. */
export class View {
  readonly viewer: string;
  readonly slot: Slot;
  readonly disposed: Promise<void>;
  private version = 0;
  private readonly frameListeners = new Set<(event: ViewEvent) => void>();
  private ended = () => {};
  constructor(
    private readonly connection: Connection,
    mount: Mount,
  ) {
    this.viewer = mount.viewer;
    this.slot = mount.slot;
    this.disposed = new Promise((resolve) => {
      this.ended = resolve;
    });
  }

  async tree(tree: Node): Promise<void> {
    this.version += 1;
    await this.connection.ask({
      verb: "view.tree",
      viewer: this.viewer,
      slot: this.slot,
      version: this.version,
      tree,
    });
  }

  async effect(effect: Effect): Promise<void> {
    await this.connection.ask({ verb: "view.effect", viewer: this.viewer, effect });
  }

  events(listener: (event: ViewEvent) => void): () => void {
    this.frameListeners.add(listener);
    return () => this.frameListeners.delete(listener);
  }

  /** @internal */
  heard(event: ViewEvent): void {
    for (const listener of this.frameListeners) listener(event);
  }

  /** @internal */
  dispose(): void {
    this.ended();
  }
}

/** What the process of a skill asks of its life: the kinds of facts that it hears, each kind when it names none, and
 * where it stands, which the environment that the daemon gave it holds unless a test names it. */
export interface SkillOptions {
  kinds?: readonly string[];
  /** The place in the log of the life that the skill hears from: the start of the log unless it names one. */
  from?: number;
  env?: Record<string, string | undefined>;
}

/** The process of a skill as a peer of the daemon: it attaches the life that the daemon started it for, under the name
 * of its skill, hears the facts of the kinds that it asked, says verbs to the life, which the life takes under that
 * name, and serves a view to each window that mounts one. */
export class Skill {
  readonly views = new Map<string, View>();
  private readonly facts = new Set<(fact: Fact, seq: number) => void>();
  private readonly mounts = new Set<(view: View) => void>();
  private constructor(
    readonly connection: Connection,
    readonly name: string,
    readonly life: string,
    private readonly kinds: ReadonlySet<string> | undefined,
  ) {
    connection.pushed((frame) => this.pushed(frame));
  }

  /** The skill of this process attached to its life, from the variables that the daemon gave it. */
  static async attach(options: SkillOptions = {}): Promise<Skill> {
    const env = options.env ?? process.env;
    const [host, life, name] = [HOST, LIFE, SKILL].map((one) => env[one]);
    if (!host || !life || !name)
      throw new Error(`A process of a skill needs ${HOST}, ${LIFE} and ${SKILL}, which the daemon gives it.`);
    const connection = await Connection.dial(daemonPlace(host), "skill", name);
    const kinds = options.kinds ? new Set(options.kinds) : undefined;
    const skill = new Skill(connection, name, life, kinds);
    await connection.ask({ verb: "attach", life, from: options.from ?? 0 });
    return skill;
  }

  /** Settles when the connection to the daemon ends, as at the end of the life or of the daemon. */
  get closed(): Promise<void> {
    return this.connection.closed;
  }

  /** Each fact of a kind that the skill asked, in the order of the log, from now on. */
  heard(listener: (fact: Fact, seq: number) => void): () => void {
    this.facts.add(listener);
    return () => this.facts.delete(listener);
  }

  /** Each view that a window mounts from now on. */
  served(listener: (view: View) => void): () => void {
    this.mounts.add(listener);
    return () => this.mounts.delete(listener);
  }

  /** The answer of the life to a verb that the skill says, under its name. */
  say(verb: Said): Promise<Answer> {
    return this.connection.ask({ ...verb, life: this.life } as Verb);
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
    if (frame.frame === "event" && frame.life === this.life && frame.event.event === "fact") {
      const fact = frame.event.data as Fact;
      if (this.kinds && !this.kinds.has(fact[0])) return;
      for (const listener of this.facts) listener(fact, frame.seq);
    } else if (frame.frame === "view.mount" && frame.life === this.life) {
      const view = new View(this.connection, frame);
      this.views.set(view.viewer, view);
      for (const listener of this.mounts) listener(view);
    } else if (frame.frame === "view.event") this.views.get(frame.viewer)?.heard(frame);
    else if (frame.frame === "view.dispose") {
      this.views.get(frame.viewer)?.dispose();
      this.views.delete(frame.viewer);
    }
  }
}

/** The act that a verb made, as the reply of the daemon names it. */
function named(answer: Answer): string {
  if (answer.answer !== "act") throw new Error(`The daemon gave ${answer.answer}, and no act.`);
  return answer.data;
}
