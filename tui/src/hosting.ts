// What a window and the host of a life share: where the host listens, how a message crosses, and what the host says
// of itself. It loads no module that loads OpenTUI, since the host draws nothing.

import { deserialize, serialize } from "bun:jsc";
import { chmodSync, mkdirSync, readdirSync } from "node:fs";
import type { Socket } from "node:net";
import { tmpdir, userInfo } from "node:os";
import { basename, dirname, join } from "node:path";
import { scratch } from "furb";

/** The version of the protocol between a window and the host of a life, which the hello of each says. A host and a
 * window of two versions do not talk. */
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
}

/** What the hello of the host gives a window: what it says of itself, the root of its life, and the count of facts
 * that the life said. */
export interface Hello extends About {
  root: string;
  seq: number;
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
 * value that JSON drops cross whole. */
export function framed(socket: Socket, heard: (message: never) => void): (message: object) => void {
  let held: Buffer = Buffer.alloc(0);
  socket.on("data", (chunk: Buffer) => {
    held = held.length ? Buffer.concat([held, chunk]) : chunk;
    while (held.length >= 4) {
      const size = held.readUInt32LE(0);
      if (held.length < 4 + size) break;
      const body = held.subarray(4, 4 + size);
      held = held.subarray(4 + size);
      heard(deserialize(new Uint8Array(body)) as never);
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
 * engine, the Kernel, the interpreter and the ears of the crate, so a host of a new build knows the life of an old one. */
export async function buildOf(): Promise<string> {
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
