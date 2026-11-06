// A daemon that goes before it answers a request, as at a crash, a kill or a handover: the request that waits fails as
// closed, and the open of a life goes on with the daemon in its place once the connection dialed it again.
import { afterEach, expect, test } from "bun:test";
import { mkdtempSync, rmSync } from "node:fs";
import { createServer, type Server, type Socket } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Closed, Daemon, RemoteLife } from "../src/client.ts";

type Asked = { id: number; verb: string; life?: string };

/** A daemon of a test on a socket of its own: each line that it hears goes to `answer`, which replies or closes. */
function served(answer: (asked: Asked, socket: Socket, connection: number) => void): {
  folder: string;
  place: string;
  server: Server;
} {
  const folder = mkdtempSync(join(tmpdir(), "furb-reconnect-"));
  const place = join(folder, "host.sock");
  let connections = 0;
  const server = createServer((socket) => {
    const connection = connections++;
    let held = "";
    socket.setEncoding("utf8");
    socket.on("error", () => {});
    socket.on("data", (chunk: string) => {
      held += chunk;
      for (let end = held.indexOf("\n"); end >= 0; end = held.indexOf("\n")) {
        const line = held.slice(0, end);
        held = held.slice(end + 1);
        const asked = JSON.parse(line) as Asked;
        if (asked.verb === "hello")
          reply(socket, asked, {
            answer: "welcome",
            data: { peer: { login: "test", role: "write", kind: "window" } },
          });
        else answer(asked, socket, connection);
      }
    });
  });
  server.listen(place);
  return { folder, place, server };
}

function reply(socket: Socket, asked: Asked, answer: unknown): void {
  socket.write(`${JSON.stringify({ frame: "reply", id: asked.id, answer })}\n`);
}

const made: { folder: string; server: Server; daemon: Daemon }[] = [];
afterEach(() => {
  for (const { folder, server, daemon } of made.splice(0)) {
    daemon.close();
    server.close();
    rmSync(folder, { recursive: true, force: true });
  }
});

test("a request that the daemon leaves when it goes fails as closed, with the stack of its caller", async () => {
  const { folder, place, server } = served((_asked, socket) => socket.destroy());
  const daemon = new Daemon(folder, place);
  made.push({ folder, server, daemon });
  await daemon.connect();
  const failed = await daemon.ask({ verb: "list" }).catch((error: unknown) => error);
  expect(failed).toBeInstanceOf(Closed);
  expect((failed as Error).message).toBe("The daemon of this machine closed.");
  expect((failed as Error).stack).toContain("reconnect.test.ts");
});

test("an open that the daemon leaves when it goes ends with the open that the connection makes once it is back", async () => {
  const opens: number[] = [];
  const { folder, place, server } = served((asked, socket, connection) => {
    if (asked.verb === "open") {
      opens.push(connection);
      // The first daemon goes before it answers the open, as a kill of it does.
      if (connection === 0) return void socket.destroy();
      reply(socket, asked, { answer: "done" });
      socket.write(
        `${JSON.stringify({ frame: "event", life: asked.life, seq: 0, event: { event: "changed" } })}\n`,
      );
    } else if (asked.verb === "pending") reply(socket, asked, { answer: "values", data: [] });
    else if (asked.verb === "detach") reply(socket, asked, { answer: "done" });
  });
  const daemon = new Daemon(folder, place);
  made.push({ folder, server, daemon });
  const life = new RemoteLife(daemon, join(folder, "life.jsonl"));
  let reset = 0;
  life.on("reset", () => reset++);
  await life.open();
  expect(opens).toEqual([0, 1]);
  expect(reset).toBe(1);
  expect(life.lost).toBe("");
  await life.detach();
});

test("an open that the daemon leaves fails as closed when the window closed its connection", async () => {
  const { folder, place, server } = served((asked, socket) => {
    if (asked.verb === "open") {
      daemon.close();
      socket.destroy();
    }
  });
  const daemon = new Daemon(folder, place);
  made.push({ folder, server, daemon });
  const life = new RemoteLife(daemon, join(folder, "life.jsonl"));
  expect(await life.open().catch((error: unknown) => error)).toBeInstanceOf(Closed);
});

test("a request in flight when the daemon goes is sent again once the life is open again, and a verb that makes an act is not", async () => {
  const seen: [number, string][] = [];
  const { folder, place, server } = served((asked, socket, connection) => {
    seen.push([connection, asked.verb]);
    if (asked.verb === "open") {
      reply(socket, asked, { answer: "done" });
      socket.write(
        `${JSON.stringify({ frame: "event", life: asked.life, seq: 0, event: { event: "changed" } })}\n`,
      );
    } else if (asked.verb === "pending") reply(socket, asked, { answer: "values", data: [] });
    // The first daemon goes with a peek unanswered, and the second with a run, as a kill of each does.
    else if ((connection === 0 && asked.verb === "peek") || (connection === 1 && asked.verb === "run"))
      socket.destroy();
    else if (asked.verb === "peek") reply(socket, asked, { answer: "value", data: 7 });
    else if (asked.verb === "detach") reply(socket, asked, { answer: "done" });
  });
  const daemon = new Daemon(folder, place);
  made.push({ folder, server, daemon });
  const life = new RemoteLife(daemon, join(folder, "life.jsonl"));
  await life.open();
  expect(await life.peek("bash1")).toBe(7);
  expect(seen.filter(([, verb]) => verb === "peek")).toEqual([
    [0, "peek"],
    [1, "peek"],
  ]);
  const ran = await life.rung({ word: "x = 1" }).catch((error: unknown) => error);
  expect(ran).toBeInstanceOf(Closed);
  expect(seen.filter(([, verb]) => verb === "run")).toHaveLength(1);
  await life.detach();
});

test("a window that reads asks its role in its hello, and the frame stalled shows as a notice of its life", async () => {
  const hellos: unknown[] = [];
  const folder = mkdtempSync(join(tmpdir(), "furb-reconnect-"));
  const place = join(folder, "host.sock");
  const server = createServer((socket) => {
    socket.setEncoding("utf8");
    socket.on("error", () => {});
    socket.on("data", (chunk: string) => {
      const asked = JSON.parse(chunk.split("\n")[0] ?? "{}") as Asked & { role?: string };
      hellos.push(asked.role);
      reply(socket, asked, {
        answer: "welcome",
        data: { peer: { login: "test", role: "read", kind: "window" } },
      });
      socket.write(
        `${JSON.stringify({ frame: "stalled", life: "life.jsonl", acts: [], notice: "The life stalled." })}\n`,
      );
    });
  });
  server.listen(place);
  const daemon = new Daemon(folder, place, "read");
  made.push({ folder, server, daemon });
  const life = new RemoteLife(daemon, "life.jsonl");
  const told = new Promise<string>((resolve) => life.once("log", resolve));
  await daemon.connect();
  expect(hellos).toEqual(["read"]);
  expect(await told).toBe("The life stalled.");
});

test("the frame sign_in of the daemon shows as the event authentication of a life, and the window asks nothing again", async () => {
  const verbs: unknown[] = [];
  const folder = mkdtempSync(join(tmpdir(), "furb-reconnect-"));
  const place = join(folder, "host.sock");
  const server = createServer((socket) => {
    socket.setEncoding("utf8");
    socket.on("error", () => {});
    socket.on("data", (chunk: string) => {
      const asked = JSON.parse(chunk.split("\n")[0] ?? "{}") as Asked & { verb?: string };
      verbs.push(asked.verb);
      if (asked.verb !== "hello") return reply(socket, asked, { answer: "done" });
      reply(socket, asked, {
        answer: "welcome",
        data: { peer: { login: "test", role: "write", kind: "window" } },
      });
      for (const frame of [
        { frame: "sign_in", state: "waiting", url: "https://auth.example/login", message: null },
        { frame: "sign_in", state: "failed", url: null, message: "No network." },
      ])
        socket.write(`${JSON.stringify(frame)}\n`);
    });
  });
  server.listen(place);
  const daemon = new Daemon(folder, place, "write");
  made.push({ folder, server, daemon });
  const life = new RemoteLife(daemon, "life.jsonl");
  const told: unknown[] = [];
  const done = new Promise<void>((resolve) =>
    life.on("authentication", (status: unknown) => {
      told.push(status);
      if (told.length === 2) resolve();
    }),
  );
  await daemon.connect();
  await done;
  expect(told).toEqual([
    { state: "waiting", url: "https://auth.example/login", message: undefined },
    { state: "failed", url: undefined, message: "No network." },
  ]);
  expect(verbs).toEqual(["hello"]);
  await life.cancelSignIn();
  expect(verbs).toEqual(["hello", "cancel_sign_in"]);
});

test("a daemon that closes before its welcome, as one that ends at its grace, is dialed again", async () => {
  let dials = 0;
  const folder = mkdtempSync(join(tmpdir(), "furb-reconnect-"));
  const place = join(folder, "host.sock");
  const server = createServer((socket) => {
    dials += 1;
    socket.setEncoding("utf8");
    socket.on("error", () => {});
    socket.on("data", (chunk: string) => {
      const asked = JSON.parse(chunk.split("\n")[0] ?? "{}") as Asked;
      if (dials === 1) return socket.destroy();
      reply(socket, asked, {
        answer: "welcome",
        data: { peer: { login: "test", role: "write", kind: "window" } },
      });
    });
  });
  server.listen(place);
  const daemon = new Daemon(folder, place, "write");
  made.push({ folder, server, daemon });
  const lost: unknown[] = [];
  daemon.on("lost", (why: unknown) => lost.push(why));
  await daemon.connect();
  expect(dials).toBe(2);
  expect(daemon.peer?.login).toBe("test");
  expect(lost).toEqual([]);
});
