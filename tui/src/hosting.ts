// What a window and the host of a life share: where the host listens, how a message crosses, and what the host says
// of itself. It loads no module that loads OpenTUI, since the host draws nothing.

import { deserialize, serialize } from "bun:jsc";
import { chmodSync, mkdirSync, readdirSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs";
import type { Socket } from "node:net";
import { hostname, tmpdir, userInfo } from "node:os";
import { basename, dirname, join } from "node:path";
import { scratch } from "furb";

/** The version of the protocol between a window and the host of a life, which the hello of each says. Each version
 * keeps the hello and the end of the life as they are, so a window takes over a host of any build: it ends that host
 * with an end that names the hand over, and a host that does not know the hand over ends as by any end. */
export const PROTOCOL = 1;
/** The longest path of a socket that this module makes: macOS takes 104 bytes with the end of the string. */
const LONGEST = 100;

/** The files of the host of one life: its socket, what it says of itself, why it could not open its life, and its
 * output. */
export interface Place {
  socket: string;
  about: string;
  failed: string;
  log: string;
}

/** What the host says of itself in its hello and in the file beside its socket: its process, the record of its life,
 * its build, the engine of its life, its protocol and the time that it started. The file is a cache: the socket and
 * the lease are the truth. */
export interface About {
  pid: number;
  record: string;
  build: string;
  engine: string;
  protocol: number;
  started: number;
  /** The build that a window of this build took the life from, when a takeover started this host. */
  from?: string;
}

/** What the hello of the host gives a window: what it says of itself, the root of its life, the count of facts that
 * the life said, and the role of the window. */
export interface Hello extends About {
  root: string;
  seq: number;
  /** A window that comes through the socket of Unix writes, and a window that comes over the network holds the role
   * that its key gives. A host of an older build says no role, and each of its windows writes. */
  role?: Role;
}

/** What an operator does in a life: read and write, or read only, which changes nothing in the life and opens no file
 * of the machine of the host by its path. */
export type Role = "read" | "write";

/** The addresses at which other operators reach a life over the network, each with the key of its role. */
export interface Served {
  read: string;
  write: string;
}

/** The longest message that a peer of the network sends before its hello gives it a role: a stranger makes the host
 * hold no more than this. */
export const HELLO = 65536;

/** The longest message that a window sends after it has a role: 256 MiB. A message may hold a file or a long text, but a
 * peer that claims more makes the host hold up to 4 GiB, so the host ends its socket. */
export const REQUEST = 256 * 1024 * 1024;

/** The requests that a window that reads makes: the views of the life and what the host reads of it, which change
 * nothing. It opens no file by its path, so it reads only what the life holds and the changes of its project. */
const READS: Record<string, readonly string[]> = {
  host: ["hello"],
  session: ["dispose"],
  engine: [
    "get",
    "peek",
    "transcript",
    "turns",
    "program",
    "standing",
    "cwd",
    "inspect",
    "names",
    "outcome",
    "pending",
    "parked",
  ],
  library: [
    "snapshot",
    "source",
    "made",
    "catalog",
    "providers",
    "sees",
    "model",
    "changes",
    "net",
    "branch",
    "act",
  ],
};

/** Whether a window that reads may make a request. */
export function reads(target: string, method: string): boolean {
  return READS[target]?.includes(method) ?? false;
}

/** Where the host listens on the network for an address that an operator gives: host:port, [host]:port, :port, a
 * port alone, or nothing, on 127.0.0.1 and a free port by default. */
export function listening(address: string): { name: string; port: number } {
  const text = address.trim();
  if (/^\d+$/.test(text)) return { name: "127.0.0.1", port: Number(text) };
  const found = /^(?:\[([^\]]+)\]|([^:[\]]*))(?::(\d+))?$/.exec(text);
  if (!found) throw new Error(`${text} is no address: give host:port, as 0.0.0.0:7700.`);
  return { name: found[1] ?? (found[2] || "127.0.0.1"), port: Number(found[3] ?? 0) };
}

/** The address that an operator gives a window to reach a host on the network: furb://host:port/key. A host that
 * listens on every interface is named by the name of its machine. */
export function addressOf(name: string, port: number, key: string): string {
  const shown = name === "0.0.0.0" || name === "::" ? hostname() : name;
  return `furb://${shown.includes(":") ? `[${shown}]` : shown}:${port}/${key}`;
}

/** The host and the port of an address of a host on the network, and the key that it holds. */
export function reachOf(address: string): { host: string; port: number; key: string } {
  let url: URL | undefined;
  try {
    url = new URL(address);
  } catch {}
  if (!url || url.protocol !== "furb:" || !url.port || url.pathname.length < 2)
    throw new Error(`${address} is no address of a host: it reads furb://host:port/key.`);
  return {
    host: url.hostname.replace(/^\[|\]$/g, ""),
    port: Number(url.port),
    key: decodeURIComponent(url.pathname.slice(1)),
  };
}

/** Where the host of the life on a record listens: hosts/<key>.sock in the cache of furb, in a folder of mode 0700, so
 * only the user connects. The key is that of the scratch directory of the record. Beside the socket, <key>.json says
 * what the host says of itself, <key>.failed why the last host that failed could not open its life, and <key>.log
 * what the hosts wrote. A path of a socket that is too long for the system goes in a folder of the user in the
 * temporary directory. Nothing on Windows, where no socket of Unix serves yet, or on a machine with no cache of furb:
 * the life then stays in the window. */
export function hostPlace(record: string): Place | undefined {
  if (process.platform === "win32") return undefined;
  const directory = scratch(record);
  if (!directory) return undefined;
  const key = basename(directory);
  let folder = join(dirname(dirname(directory)), "hosts");
  if (Buffer.byteLength(join(folder, `${key}.sock`)) > LONGEST)
    folder = join(tmpdir(), `furb-hosts-${userInfo().uid}`);
  mkdirSync(folder, { recursive: true, mode: 0o700 });
  chmodSync(folder, 0o700);
  return {
    socket: join(folder, `${key}.sock`),
    about: join(folder, `${key}.json`),
    failed: join(folder, `${key}.failed`),
    log: join(folder, `${key}.log`),
  };
}

/** The sender of the messages on a socket, which hears each message that comes on it. A message crosses as the
 * structured clone of the runtime after its length in four bytes, as it crosses to a worker, so an error, a map and a
 * value that JSON drops cross whole. A message longer than most, or one that does not read, ends the socket. */
export function framed(
  socket: Socket,
  heard: (message: never) => void,
  most: () => number = () => Number.POSITIVE_INFINITY,
): (message: object) => void {
  let held: Buffer = Buffer.alloc(0);
  socket.on("data", (chunk: Buffer) => {
    held = held.length ? Buffer.concat([held, chunk]) : chunk;
    while (held.length >= 4 && !socket.destroyed) {
      const size = held.readUInt32LE(0);
      if (size > most()) {
        socket.destroy();
        return;
      }
      if (held.length < 4 + size) break;
      const body = held.subarray(4, 4 + size);
      held = held.subarray(4 + size);
      let message: unknown;
      try {
        message = deserialize(new Uint8Array(body));
      } catch {
        socket.destroy();
        return;
      }
      heard(message as never);
    }
  });
  return (message) => {
    if (socket.destroyed) return;
    const body = Buffer.from(serialize(message));
    const head = Buffer.alloc(4);
    head.writeUInt32LE(body.length, 0);
    socket.write(Buffer.concat([head, body]));
  };
}

/** The build of this program: the version of the package furb and the digest of its native library, which holds the
 * engine, the Kernel, the interpreter and the ears of the crate, so a window of a new build knows a host of an old one
 * and takes it over. FURB_BUILD names another build in its place, as a test of a takeover does. */
export async function buildOf(): Promise<string> {
  if (process.env.FURB_BUILD) return process.env.FURB_BUILD;
  const index = Bun.resolveSync("furb", import.meta.dir);
  const root = dirname(dirname(index));
  const { version } = (await Bun.file(join(root, "package.json")).json()) as { version: string };
  const names = readdirSync(root).filter((name) => /^furb(\..+)?\.node$/.test(name));
  const native =
    process.env.NAPI_RS_NATIVE_LIBRARY_PATH ??
    names.find((name) => name.includes(`${process.platform}-${process.arch}`)) ??
    names.find((name) => name === "furb.node");
  if (!native) return version;
  const hasher = new Bun.CryptoHasher("sha256");
  for await (const chunk of Bun.file(native.includes("/") ? native : join(root, native)).stream())
    hasher.update(chunk);
  return `${version}+${hasher.digest("hex").slice(0, 12)}`;
}

/** What a host that failed to start says when another host of the same life runs: the window then waits for that host. */
export const RIVAL = "Another host of this life runs";

/** What the lease of a record says when a process that is no host holds it. */
export const OWNED = "Another process owns";

/** Why the host with a process number stopped, when it wrote its fault in the file of failures of its place before it
 * ended: nothing when the file is gone, holds the fault of another host, or is not whole. */
export function failureOf(place: Place, pid: number): string | undefined {
  try {
    const said = JSON.parse(readFileSync(place.failed, "utf8")) as { pid?: unknown; error?: unknown };
    return said.pid === pid && typeof said.error === "string" ? said.error : undefined;
  } catch {
    return undefined;
  }
}

/** What the host at a place said of itself, or nothing when it said none, when the file is gone, or when it holds no
 * whole JSON of a host. It reads the file once and looks at nothing before: a host that ends removes the file at any
 * moment, so a look before the read is a race. */
export function aboutOf(place: Place): About | undefined {
  try {
    const said: unknown = JSON.parse(readFileSync(place.about, "utf8"));
    return typeof said === "object" && said !== null && typeof (said as About).pid === "number"
      ? (said as About)
      : undefined;
  } catch {
    return undefined;
  }
}

/** The host says of itself in the file at its place, whole or not at all: a reader never sees a part of it. */
export function tellAbout(place: Place, about: About): void {
  const temporary = `${place.about}.${process.pid}.tmp`;
  try {
    writeFileSync(temporary, `${JSON.stringify(about)}\n`, { mode: 0o600 });
    renameSync(temporary, place.about);
  } catch (error) {
    rmSync(temporary, { force: true });
    throw error;
  }
}

/** Whether a process is a host of a life: it runs, and its command line names the program of the host with the options
 * that a window gives it, as `bun /path/host.ts {...}`. A process that ended, one that waits to be reaped, a process that
 * only names host.ts, as an editor does, and an unrelated process that took the number of a host are no host. */
export function isHost(pid: number): boolean {
  try {
    process.kill(pid, 0);
  } catch {
    return false;
  }
  const got = Bun.spawnSync(["ps", "-o", "command=", "-p", String(pid)]);
  return /(^|\/)host\.ts \{/.test(got.stdout.toString());
}

/** The end of a host that does not answer: the signal that ends a life as the end of a window does, and the kill after
 * a grace in milliseconds. A host that a signal stopped wakes to hear the end first. It gives when the process is gone. */
export async function stopProcess(pid: number, grace = 12000): Promise<void> {
  try {
    process.kill(pid, "SIGTERM");
    process.kill(pid, "SIGCONT");
  } catch {
    return;
  }
  for (const deadline = Date.now() + grace; isHost(pid) && Date.now() < deadline; ) await Bun.sleep(50);
  if (!isHost(pid)) return;
  try {
    process.kill(pid, "SIGKILL");
  } catch {}
  for (const deadline = Date.now() + 5000; isHost(pid) && Date.now() < deadline; ) await Bun.sleep(50);
}
