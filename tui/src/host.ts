// The host of one life: a process of its own that holds the life in the worker of the TUI, behind a socket, so the life
// outlives each window. A window starts it, detached from the window, with the options of the life as JSON. It loads
// no module that loads OpenTUI, since it draws nothing.
import { randomUUID, timingSafeEqual } from "node:crypto";
import { rmSync, writeFileSync } from "node:fs";
import { type AddressInfo, createServer, type Socket } from "node:net";
import type { HostState } from "./bridge.ts";
import {
  type About,
  aboutOf,
  addressOf,
  buildOf,
  framed,
  HELLO,
  type Hello,
  hostPlace,
  isHost,
  listening,
  PROTOCOL,
  REQUEST,
  RIVAL,
  type Role,
  reads,
  type Served,
  tellAbout,
} from "./hosting.ts";
import type { EngineOptions } from "./models.ts";

type Request = { id: number; target: string; method: string; args: unknown[] };
type Send = (message: object) => void;
/** Who waits for an answer of the worker: a window, with the number of its request there, or the host itself. */
type Asker = { send?: Send; id: number; settle?: (value: unknown, error?: string) => void };

const {
  host: _,
  from,
  ...options
} = JSON.parse(process.argv[2] ?? "{}") as EngineOptions & { from?: string };
const record = options.record;
const place = record ? hostPlace(record) : undefined;
if (!record || !place) {
  console.error(
    "furb-host: a host needs the record of a life, on a machine whose cache of furb holds sockets.",
  );
  process.exit(2);
}
const started = Date.now();
const about: About = {
  pid: process.pid,
  record,
  build: "",
  engine: "",
  protocol: PROTOCOL,
  started,
  ...(from ? { from } : {}),
};
const tell = () => tellAbout(place, about);
// Another host of the life that runs owns it, so this one ends before it opens a second life on the record. The host
// says of itself at once, so a window knows who opens the life while it opens.
const rival = aboutOf(place);
if (rival && rival.pid !== process.pid && isHost(rival.pid)) {
  const said = `${RIVAL}, the process ${rival.pid}.`;
  writeFileSync(place.failed, `${JSON.stringify({ pid: process.pid, error: said })}\n`, { mode: 0o600 });
  console.error(`furb-host: ${said}`);
  process.exit(1);
}
tell();
const building = buildOf().catch((error: unknown) => `unknown: ${error}`);

const worker = new Worker(new URL("worker.ts", import.meta.url).href, { type: "module" });
/** Each window that said hello, by its sender, with its role. */
const windows = new Map<Send, Role>();
/** Each request that the worker has not answered, under the number that the host gave it: the window that made it and
 * its number there. */
const waiting = new Map<number, Asker>();
let sequence = 0;
/** The last state of the life, which a window that attaches reads first. */
let last: HostState | undefined;
let ending = false;
/** What each other window hears when the life ends. */
let why = "The life was ended from another window.";
/** Whether a signal ends the host: no window ended the life, so no window is told that one did, and each window
 * that stays reaches a new host, as it does when a kill ends the host. */
let signalled = false;
/** Why the life stopped, when a fault cut a run of its sandbox, which each window that attaches later hears. */
let stopped: string | undefined;
/** How long a host that hands its life over waits for the commands and the replies of the models that run, so that
 * each that ends in time lands in the record. Each other ends as interrupted in the life of the next host, as a
 * command that a crash cut does. */
const HANDOVER = 10000;
let handing = false;

function ask(target: string, method: string, args: unknown[], to: Asker): void {
  const id = ++sequence;
  waiting.set(id, to);
  worker.postMessage({ id, target, method, args });
}
const asked = (target: string, method: string, ...args: unknown[]) =>
  new Promise<unknown>((resolve, reject) =>
    ask(target, method, args, {
      id: 0,
      settle: (value: unknown, error?: string) =>
        error === undefined ? resolve(value) : reject(new Error(error)),
    }),
  );

/** The end of the host: the socket and what the host said of itself go, and the process ends. */
function close(code: number): never {
  rmSync(place?.socket ?? "", { force: true });
  rmSync(place?.about ?? "", { force: true });
  process.exit(code);
}

// The socket of Unix stands in a folder that only the user opens, so each window that comes through it writes.
const server = createServer((socket) => {
  const send = framed(
    socket,
    (message: Request) => heard(message, send, "write"),
    () => REQUEST,
  );
  socket.on("close", () => windows.delete(send));
  socket.on("error", () => windows.delete(send));
});

/** The door of the life on the network, which a window that writes opens: the addresses that it gives each role. */
let door: Promise<Served> | undefined;

/** The door of the life on the network at an address, or the door that stands already, as it is open once for the
 * life of the host. Each role takes a key of its own, which only the address of that role holds. */
function serve(address: string): Promise<Served> {
  door ??= (async () => {
    const { name, port } = listening(address);
    const keys: Record<Role, string> = { read: randomUUID(), write: randomUUID() };
    const network = createServer((socket) => welcome(socket, keys));
    await new Promise<void>((resolve, reject) => {
      network.once("error", reject);
      network.listen(port, name, () => resolve());
    });
    const at = (network.address() as AddressInfo).port;
    return { read: addressOf(name, at, keys.read), write: addressOf(name, at, keys.write) };
  })();
  // A door that did not open, as on a port that another process holds, can open later at another address.
  door.catch(() => {
    door = undefined;
  });
  return door;
}

/** The role that a key gives, and nothing for a key of no role. */
function roleOf(key: unknown, keys: Record<Role, string>): Role | undefined {
  const given = Buffer.from(typeof key === "string" ? key : "");
  return (["write", "read"] as const).find((role) => {
    const own = Buffer.from(keys[role]);
    return own.length === given.length && timingSafeEqual(own, given);
  });
}

/** A peer of the network: its first message is a hello with the key of a role, in a short message and in ten seconds,
 * or the host ends the socket, so a stranger reaches nothing of the life. */
function welcome(socket: Socket, keys: Record<Role, string>): void {
  let role: Role | undefined;
  const timer = setTimeout(() => socket.destroy(), 10000);
  const send = framed(
    socket,
    (message: Request) => {
      if (role) return heard(message, send, role);
      const said = typeof message === "object" && message ? message : ({} as Request);
      const hello = said.target === "host" && said.method === "hello";
      role = hello ? roleOf((said.args?.[0] as { key?: unknown } | undefined)?.key, keys) : undefined;
      if (!role) {
        send({ id: said.id ?? 0, error: "This key opens no window of the life." });
        socket.end();
        return;
      }
      clearTimeout(timer);
      heard(said, send, role);
    },
    () => (role ? REQUEST : HELLO),
  );
  socket.on("close", () => {
    clearTimeout(timer);
    windows.delete(send);
  });
  socket.on("error", () => windows.delete(send));
}

worker.onmessage = ({ data }) => {
  if (data.authentication) {
    for (const [send, role] of windows) if (role === "write") send(data);
    return;
  }
  if (data.state || data.log || data.fault) {
    // The log of the host keeps each fault, so a life that stopped with no window attached says why.
    if (data.fault) console.error(`furb-host: ${data.fault}`);
    if (data.stopped) stopped = data.fault;
    // A state holds the measures of the chains that changed and the compactions since the state before it, so the last
    // state holds them all, which a window that attaches reads first.
    if (data.state)
      last = {
        ...data.state,
        measures: { ...last?.measures, ...data.state.measures },
        compactions: [...(last?.compactions ?? []), ...data.state.compactions],
      };
    for (const send of windows.keys()) send(data);
    return;
  }
  const to = waiting.get(data.id);
  waiting.delete(data.id);
  if (to?.settle) to.settle(data.value, data.error);
  to?.send?.({ ...data, id: to.id });
  if (!data.disposed) return;
  // The life ended: each other window hears so, and the host ends once each window has its answer.
  if (!signalled) for (const send of windows.keys()) if (send !== to?.send) send({ ended: why });
  server.close();
  setTimeout(() => close(0), 50);
};
worker.onerror = (event) => {
  // The worker of a life that ended may fail as it closes, which ends nothing more.
  if (ending) return;
  console.error(`furb-host: ${event.message}`);
  for (const send of windows.keys()) send({ fault: event.message });
  close(1);
};

/** What a window says: its hello, which attaches it with its role, or a request, which goes to the worker when the role
 * of the window makes it. A window that detaches only closes its socket, and the life goes on. */
function heard(message: Request, send: Send, role: Role): void {
  if (message.target === "host" && message.method === "hello") {
    send({ id: message.id, value: { ...about, root, seq: last?.seq ?? 0, role } satisfies Hello });
    // A window that attaches draws what the life holds, so the first state tells it that facts came.
    if (last) send({ state: { ...last, facts: Math.max(1, last.facts) } });
    if (stopped) send({ fault: stopped });
    windows.set(send, role);
    return;
  }
  // The first run checks the machine of the window, and a window that reads checks nothing of the host.
  if (role === "read" && message.target === "library" && message.method === "setup") {
    send({ id: message.id, value: null });
    return;
  }
  if (role === "read" && !reads(message.target, message.method)) {
    send({ id: message.id, error: `This window reads the life only, so it cannot ${message.method}.` });
    return;
  }
  if (message.target === "host" && message.method === "serve") {
    serve(String(message.args[0] ?? "")).then(
      (value) => send({ id: message.id, value }),
      (error: unknown) =>
        send({ id: message.id, error: error instanceof Error ? error.message : String(error) }),
    );
    return;
  }
  if (message.target === "session" && message.method === "end") {
    const taker = (message.args[0] as { handover?: { build?: string } } | undefined)?.handover;
    if (taker) {
      if (!handing && !ending) void handOver(String(taker.build ?? ""));
      return;
    }
    ending = true;
  }
  ask(message.target, message.method, message.args, { send, id: message.id });
}

/** The ids of the commands and of the replies of the models that run, or none when the life cannot say. */
const running = () =>
  asked("library", "running").then(
    (ids) => new Set(Array.isArray(ids) ? ids.map(String) : []),
    () => new Set<string>(),
  );

/** The hand over of the life to a window of another build: the host waits a bounded time for what runs, then ends
 * the life, so its record and its lease are free for the host that the window starts. */
async function handOver(build: string): Promise<void> {
  handing = true;
  const left = await running();
  for (const deadline = Date.now() + HANDOVER; left.size && Date.now() < deadline; ) {
    await Bun.sleep(50);
    const now = await running();
    for (const id of left) if (!now.has(id)) left.delete(id);
  }
  if (ending) return;
  ending = true;
  why = `A window of the build ${build} took the life over, and a host of that build goes on with it. Open the life again in a window of that build.`;
  ask("session", "end", [], { id: 0 });
}

let root: string;
try {
  root = String(await asked("session", "open", options));
  about.engine = String((await asked("library", "made")) ?? "");
} catch (error) {
  // The reason goes to a file of its own, so a host that listens keeps what it said of itself.
  const why = error instanceof Error ? error.message : String(error);
  writeFileSync(place.failed, `${JSON.stringify({ pid: process.pid, error: why })}\n`, { mode: 0o600 });
  console.error(`furb-host: ${why}`);
  rmSync(place.about, { force: true });
  process.exit(1);
}
about.build = await building;
// The lease of the record is this host's now, so a socket at the place is of a host that ended.
rmSync(place.socket, { force: true });
server.listen(place.socket, tell);
// A signal ends the life as the end of a window does, so the commands and the models end with it.
for (const signal of ["SIGINT", "SIGTERM"] as const)
  process.once(signal, () => {
    signalled = true;
    if (!ending) {
      ending = true;
      ask("session", "end", [], { id: 0 });
    }
    setTimeout(() => close(1), 10000).unref();
  });
