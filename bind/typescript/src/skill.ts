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

/** One connection to the daemon, after its hello. Each verb takes a number and its answer, in any order, and each
 * frame that the daemon pushes goes to each listener of `pushed`. `closed` settles when the stream ends, from either
 * side, and each verb that waits then fails. */
export class Connection {
  /** The peer as the daemon knows it after the hello. */
  peer!: Peer;
  readonly closed: Promise<void>;
  private next = 0;
  private readonly waiting = new Map<number, { resolve(answer: Answer): void; reject(error: Error): void }>();
  private readonly listeners = new Set<(frame: Frame) => void>();
  private constructor(private readonly socket: Socket) {
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
    this.closed = new Promise((resolve) => {
      socket.on("close", () => {
        for (const waits of this.waiting.values())
          waits.reject(new Closed("The daemon closed the connection."));
        this.waiting.clear();
        resolve();
      });
    });
    // An error of the stream closes it, which `closed` tells.
    socket.on("error", () => {});
  }

  /** A connection to the daemon at a place, as a peer of a kind, with the name of a skill for a skill. */
  static async dial(place: string, kind: Kind, name?: string): Promise<Connection> {
    const socket = await new Promise<Socket>((resolve, reject) => {
      const dialing = createConnection(place);
      dialing.once("connect", () => {
        dialing.removeListener("error", reject);
        resolve(dialing);
      });
      dialing.once("error", reject);
    });
    const connection = new Connection(socket);
    const welcome = await connection.ask({ verb: "hello", protocol: PROTOCOL, kind, name });
    if (welcome.answer !== "welcome") throw new Error(`The daemon gave ${welcome.answer}, and no welcome.`);
    connection.peer = welcome.data.peer;
    return connection;
  }

  /** The answer of the daemon to a verb, or its refusal as a `Refused`. */
  ask(verb: Verb): Promise<Answer> {
    const id = this.next++;
    return new Promise<Answer>((resolve, reject) => {
      this.waiting.set(id, { resolve, reject });
      this.socket.write(`${JSON.stringify({ id, ...verb })}\n`);
    });
  }

  /** Each frame that the daemon pushes from now on, heard until the call that this gives. */
  pushed(listener: (frame: Frame) => void): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  close(): void {
    this.socket.end();
  }

  private heard(frame: Frame): void {
    if (frame.frame === "reply" || frame.frame === "refused") {
      const waits = this.waiting.get(frame.id ?? -1);
      if (!waits) return;
      this.waiting.delete(frame.id ?? -1);
      if (frame.frame === "reply") waits.resolve(frame.answer);
      else waits.reject(new Refused(frame.error.error, frame.why));
      return;
    }
    for (const listener of this.listeners) listener(frame);
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
  private readonly listeners = new Set<(event: ViewEvent) => void>();
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
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  /** @internal */
  heard(event: ViewEvent): void {
    for (const listener of this.listeners) listener(event);
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
