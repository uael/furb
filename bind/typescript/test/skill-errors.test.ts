import { expect, test } from "bun:test";
import { createServer, Socket } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Closed, Connection, Refused } from "../src/skill.ts";

test("a refusal of the daemon is named by its kind and a request that its socket left is Closed, the two that the window and a skill share", () => {
  expect(new Refused("role", "No role.").name).toBe("role");
  expect(new Refused("refused", "No.").name).toBe("Refused");
  expect(new Closed("gone")).toMatchObject({ name: "Closed", message: "gone" });
  expect(new Closed("gone")).toBeInstanceOf(Error);
});

test("an error that comes on the socket takes the stack of the verb that asked it, which names its caller", async () => {
  const place = join(tmpdir(), `furb-stack-${process.pid}.sock`);
  const server = createServer((socket) => {
    socket.on("data", (chunk) => {
      for (const line of String(chunk).split("\n").filter(Boolean)) {
        const { id, verb } = JSON.parse(line) as { id: number; verb: string };
        const frame =
          verb === "hello"
            ? { frame: "reply", id, answer: { answer: "welcome", data: { peer: {} } } }
            : verb === "list"
              ? { frame: "refused", id, error: { error: "role" }, why: "No role." }
              : undefined;
        if (frame) socket.write(`${JSON.stringify(frame)}\n`);
        else socket.end();
      }
    });
  });
  await new Promise<void>((resolve) => server.listen(place, resolve));
  try {
    const connection = await Connection.dial(place, "window");
    const fromHere = async (verb: "list" | "get") =>
      connection
        .ask(verb === "list" ? { verb: "list" } : { verb: "get", life: "l", act: "a" })
        .catch((e: Error) => e) as Promise<Error>;
    const refused = await fromHere("list");
    expect(refused).toBeInstanceOf(Refused);
    expect(refused.stack).toContain("fromHere");
    const closed = await fromHere("get");
    expect(closed).toBeInstanceOf(Closed);
    expect(closed.stack).toContain("fromHere");
  } finally {
    server.close();
  }
});

test("an idle connection keeps the process alive only while a life holds it, and one that is not idle always does", async () => {
  const place = join(tmpdir(), `furb-hold-${process.pid}.sock`);
  const server = createServer((socket) => {
    socket.on("data", (chunk) => {
      const { id } = JSON.parse(String(chunk).split("\n")[0] ?? "{}") as { id: number };
      socket.write(
        `${JSON.stringify({ frame: "reply", id, answer: { answer: "welcome", data: { peer: {} } } })}\n`,
      );
    });
  });
  await new Promise<void>((resolve) => server.listen(place, resolve));
  // The runtime gives no way to read whether a socket keeps the process alive, so the test records each ref and unref.
  const state = new WeakMap<object, boolean>();
  const [ref, unref] = [Socket.prototype.ref, Socket.prototype.unref];
  Socket.prototype.ref = function (this: Socket) {
    state.set(this, true);
    return ref.call(this);
  };
  Socket.prototype.unref = function (this: Socket) {
    state.set(this, false);
    return unref.call(this);
  };
  const refs = (connection: Connection) =>
    state.get((connection as unknown as { socket: Socket }).socket) ?? true;
  try {
    const idle = await Connection.dial(place, "window", undefined, true);
    expect(refs(idle)).toBe(false);
    idle.hold();
    expect(refs(idle)).toBe(true);
    idle.hold();
    idle.release();
    expect(refs(idle)).toBe(true);
    idle.release();
    expect(refs(idle)).toBe(false);
    idle.close();
    const plain = await Connection.dial(place, "skill", "k");
    expect(refs(plain)).toBe(true);
    plain.hold();
    plain.release();
    expect(refs(plain)).toBe(true);
    plain.close();
  } finally {
    [Socket.prototype.ref, Socket.prototype.unref] = [ref, unref];
    server.close();
  }
});
