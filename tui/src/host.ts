// The host of one life: a process of its own that holds the life in the worker of the TUI, behind a socket, so the life
// outlives each window. A window starts it, detached from the window, with the options of the life as JSON. It loads
// no module that loads OpenTUI, since it draws nothing.
import { rmSync, writeFileSync } from "node:fs";
import { createServer } from "node:net";
import type { HostState } from "./bridge.ts";
import { type About, buildOf, framed, type Hello, hostPlace, PROTOCOL } from "./hosting.ts";
import type { EngineOptions } from "./models.ts";

type Request = { id: number; target: string; method: string; args: unknown[] };
type Send = (message: object) => void;
/** Who waits for an answer of the worker: a window, with the number of its request there, or the host itself. */
type Asker = { send?: Send; id: number; settle?: (value: unknown, error?: string) => void };

const { host: _, ...options } = JSON.parse(process.argv[2] ?? "{}") as EngineOptions;
const record = options.record;
const place = record ? hostPlace(record) : undefined;
if (!record || !place) {
  console.error(
    "furb-host: a host needs the record of a life, on a machine whose cache of furb holds sockets.",
  );
  process.exit(2);
}
const started = Date.now();
const about: About = { pid: process.pid, record, build: "", engine: "", protocol: PROTOCOL, started };
const tell = () => writeFileSync(place.about, `${JSON.stringify(about)}\n`, { mode: 0o600 });
const building = buildOf().catch((error: unknown) => `unknown: ${error}`);

const worker = new Worker(new URL("worker.ts", import.meta.url).href, { type: "module" });
/** Each window that said hello, by its sender. */
const windows = new Set<Send>();
/** Each request that the worker has not answered, under the number that the host gave it: the window that made it and
 * its number there. */
const waiting = new Map<number, Asker>();
let sequence = 0;
/** The last state of the life, which a window that attaches reads first. */
let last: HostState | undefined;
let ending = false;

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

const server = createServer((socket) => {
  const send = framed(socket, (message: Request) => heard(message, send));
  socket.on("close", () => windows.delete(send));
  socket.on("error", () => windows.delete(send));
});

worker.onmessage = ({ data }) => {
  if (data.state || data.log || data.fault) {
    // A state holds the measures of the chains that changed and the compactions since the state before it, so the last
    // state holds them all, which a window that attaches reads first.
    if (data.state)
      last = {
        ...data.state,
        measures: { ...last?.measures, ...data.state.measures },
        compactions: [...(last?.compactions ?? []), ...data.state.compactions],
      };
    for (const send of windows) send(data);
    return;
  }
  const to = waiting.get(data.id);
  waiting.delete(data.id);
  if (to?.settle) to.settle(data.value, data.error);
  to?.send?.({ ...data, id: to.id });
  if (!data.disposed) return;
  // The life ended: each other window hears so, and the host ends once each window has its answer.
  for (const send of windows) if (send !== to?.send) send({ ended: "The life ended in another window." });
  server.close();
  setTimeout(() => close(0), 50);
};
worker.onerror = (event) => {
  // The worker of a life that ended may fail as it closes, which ends nothing more.
  if (ending) return;
  console.error(`furb-host: ${event.message}`);
  for (const send of windows) send({ fault: event.message });
  close(1);
};

/** What a window says: its hello, which attaches it, or a request of the worker, which goes to it. A window that
 * detaches only closes its socket, and the life goes on. */
function heard(message: Request, send: Send): void {
  if (message.target === "host" && message.method === "hello") {
    send({ id: message.id, value: { ...about, root, seq: last?.seq ?? 0 } satisfies Hello });
    // A window that attaches draws what the life holds, so the first state tells it that facts came.
    if (last) send({ state: { ...last, facts: Math.max(1, last.facts) } });
    windows.add(send);
    return;
  }
  if (message.target === "session" && message.method === "end") ending = true;
  ask(message.target, message.method, message.args, { send, id: message.id });
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
  process.exit(1);
}
about.build = await building;
// The lease of the record is this host's now, so a socket at the place is of a host that ended.
rmSync(place.socket, { force: true });
server.listen(place.socket, tell);
// A signal ends the life as the end of a window does, so the commands and the models end with it.
for (const signal of ["SIGINT", "SIGTERM"] as const)
  process.once(signal, () => {
    if (!ending) {
      ending = true;
      ask("session", "end", [], { id: 0 });
    }
    setTimeout(() => close(1), 10000).unref();
  });
