import { expect, test } from "bun:test";
import { existsSync, readFileSync, rmSync } from "node:fs";
import { mkdtemp, readdir, rm, writeFile } from "node:fs/promises";
import { createConnection, createServer } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { openEngine } from "../src/bridge.ts";
import { type About, aboutOf, hostPlace, listening, type Place, reachOf, tellAbout } from "../src/hosting.ts";

/** The moment a condition holds, read again every 50 ms. */
async function eventually(ready: () => boolean | Promise<boolean>, what: string): Promise<void> {
  for (const deadline = Date.now() + 20000; !(await ready()); await Bun.sleep(50))
    if (Date.now() > deadline) throw new Error(`Waited 20 seconds for ${what}.`);
}

/** Whether a process runs. */
function alive(pid: number): boolean {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

/** What the host at a place says of itself, which a test reads while the host runs. */
function said(place: Place): About {
  const about = aboutOf(place);
  if (!about) throw new Error("The host said nothing of itself.");
  return about;
}

/** A life of the demo in a folder of its own, whose host each window of the test reaches, and which ends with the
 * test. */
async function demoLife(use: (record: string, directory: string) => Promise<void>): Promise<void> {
  const directory = await mkdtemp(join(tmpdir(), "furb-host-"));
  const record = join(directory, "life.jsonl");
  const place = hostPlace(record);
  try {
    await use(record, directory);
  } finally {
    // A host that a failed test left ends here, with its life.
    const about = place && aboutOf(place);
    if (about) {
      const { pid } = about;
      if (alive(pid)) process.kill(pid, "SIGTERM");
      await eventually(() => !alive(pid), "the host to end").catch(() => process.kill(pid, "SIGKILL"));
    }
    await rm(directory, { recursive: true, force: true });
  }
}

test.skipIf(process.platform === "win32")(
  "a command goes on in the host after its window ends, and a new window attaches and sees it done",
  () =>
    demoLife(async (record, directory) => {
      const place = hostPlace(record);
      if (!place) throw new Error("No socket serves here.");
      // The first window starts the host, starts a command, and ends while the command runs.
      const script = join(directory, "window.ts");
      await writeFile(
        script,
        `import { openEngine } from ${JSON.stringify(join(import.meta.dir, "../src/bridge.ts"))};
const { engine, host } = await openEngine({ demo: true, host: true, cwd: ${JSON.stringify(directory)}, record: ${JSON.stringify(record)} });
const command = await engine.bash("sleep 2; echo landed", { on: engine.root });
console.log(JSON.stringify({ command, root: engine.root, done: await engine.peek(command) }));
await host.dispose();
`,
      );
      const window = Bun.spawn([process.execPath, script], { stdout: "pipe", stderr: "inherit" });
      expect(await window.exited).toBe(0);
      const { command, root, done } = JSON.parse(await new Response(window.stdout).text()) as {
        command: string;
        root: string;
        done: unknown;
      };
      expect(done).toBeNull();
      const about = said(place);
      expect(about).toMatchObject({ record, protocol: 1 });
      expect(about.pid).not.toBe(window.pid);
      expect(about.engine).toMatch(/^[0-9a-f]{64}$/);
      expect(about.build).toMatch(/\+[0-9a-f]{12}$/);
      expect(alive(about.pid)).toBe(true);
      // The window ended before the command did, and the host runs it on.
      const { engine, host } = await openEngine({ demo: true, host: true, cwd: directory, record });
      expect(engine.root).toBe(root);
      await eventually(async () => (await engine.peek(command)) !== null, "the command to end in the host");
      expect(await engine.peek(command)).toMatchObject({ code: 0, stdout: { content: "landed\n" } });
      expect(host.seq).toBeGreaterThan(0);
      // The window that attached ends the life, so the host ends and its socket goes.
      await host.end();
      await eventually(() => !alive(about.pid), "the host to end");
      expect(existsSync(place.socket)).toBe(false);
      expect(existsSync(place.about)).toBe(false);
    }),
);

test.skipIf(process.platform === "win32")(
  "a life that a fault stops tells the window that is attached, and the log of the host keeps why",
  () =>
    demoLife(async (record, directory) => {
      const place = hostPlace(record);
      if (!place) throw new Error("No socket serves here.");
      const { engine, host } = await openEngine({ demo: true, host: true, cwd: directory, record });
      const faults: string[] = [];
      host.on("fault", (error: Error) => faults.push(error.message));
      const cut = await Promise.resolve()
        .then(() => engine.rung({ word: "x = 'a' * (5 << 30)", on: engine.root }))
        .then(
          () => "",
          (error: Error) => error.message,
        );
      expect(cut).toContain("memory limit exceeded");
      await eventually(() => faults.length > 0, "the window to hear that the life stopped");
      expect(faults[0]).toStartWith("The life stopped at MemoryError: memory limit exceeded");
      expect(readFileSync(place.log, "utf8")).toContain(`furb-host: ${faults[0]}`);
      await host.end();
    }),
);

test.skipIf(process.platform === "win32")(
  "two windows attach to one host, and the end of the life in one tells the other",
  () =>
    demoLife(async (record, directory) => {
      const options = { demo: true, host: true, cwd: directory, record };
      const one = await openEngine(options);
      const two = await openEngine(options);
      expect(two.engine.root).toBe(one.engine.root);
      // A command that one window runs, the other sees in the life.
      const command = await one.engine.bash("echo seen", { on: one.engine.root });
      await eventually(async () => (await two.engine.peek(command)) !== null, "the command to end");
      const faults: string[] = [];
      two.host.on("fault", (error: Error) => faults.push(error.message));
      await one.host.end();
      await eventually(() => faults.length > 0, "the other window to hear the end");
      expect(faults).toEqual(["The life was ended from another window."]);
      expect(await two.engine.peek(command).catch((error: Error) => error.message)).toBe(
        "The life was ended from another window.",
      );
    }),
);

test.skipIf(process.platform === "win32")(
  "a window whose host ends with no end of the life starts a host again, and reads the acts of the life from the start",
  () =>
    demoLife(async (record, directory) => {
      const place = hostPlace(record);
      if (!place) throw new Error("No socket serves here.");
      const { engine, host } = await openEngine({ demo: true, host: true, cwd: directory, record });
      const command = await engine.bash("echo kept", { on: engine.root });
      await eventually(async () => (await engine.peek(command)) !== null, "the command to end");
      const { pid } = said(place);
      const reset = new Promise<void>((resolve) => host.once("reset", resolve));
      process.kill(pid, "SIGKILL");
      await reset;
      const next = said(place);
      expect(next.pid).not.toBe(pid);
      expect(await engine.peek(command)).toMatchObject({ code: 0, stdout: { content: "kept\n" } });
      await host.end();
    }),
);

test.skipIf(process.platform === "win32")(
  "a window whose host takes a connection as its process ends and drops it reaches the place again, and starts a host",
  () =>
    demoLife(async (record, directory) => {
      const place = hostPlace(record);
      if (!place) throw new Error("No socket serves here.");
      const { engine, host } = await openEngine({ demo: true, host: true, cwd: directory, record });
      const command = await engine.bash("echo kept", { on: engine.root });
      await eventually(async () => (await engine.peek(command)) !== null, "the command to end");
      const { pid } = said(place);
      // The host stops, and its place takes one connection and drops it with no answer, as the socket of a host does
      // while a kill ends its process.
      process.kill(pid, "SIGSTOP");
      rmSync(place.socket, { force: true });
      let taken = 0;
      const ending = createServer((socket) => {
        taken++;
        socket.destroy();
        ending.close();
      });
      await new Promise<void>((resolve) => ending.listen(place.socket, resolve));
      const faults: string[] = [];
      host.on("fault", (error: Error) => faults.push(error.message));
      const reset = new Promise<void>((resolve) => host.once("reset", resolve));
      process.kill(pid, "SIGKILL");
      await reset;
      expect([taken, faults]).toEqual([1, []]);
      expect(await engine.peek(command)).toMatchObject({ code: 0, stdout: { content: "kept\n" } });
      await host.end();
    }),
);

test.skipIf(process.platform === "win32")(
  "a host that cannot open its life ends, and the window that started it fails with its reason",
  () =>
    demoLife(async (record, directory) => {
      await writeFile(record, "not json\n");
      const failure = await openEngine({ demo: true, host: true, cwd: directory, record }).then(
        () => "opened",
        (error: Error) => error.message,
      );
      expect(failure).toContain("Invalid record entry");
      expect(existsSync(hostPlace(record)?.socket ?? "")).toBe(false);
    }),
);

test.skipIf(process.platform === "win32")(
  "a window of a new build takes over a host of another build, the life goes on in a host of its build, and a window of the old host hears the end",
  () =>
    demoLife(async (record, directory) => {
      const place = hostPlace(record);
      if (!place) throw new Error("No socket serves here.");
      const options = { demo: true, host: true, cwd: directory, record };
      // A window of the old build starts a host of that build, which takes it from the window.
      process.env.FURB_BUILD = "0.0.0+0ld0b0i1d000";
      let old: Awaited<ReturnType<typeof openEngine>>;
      try {
        old = await openEngine(options);
      } finally {
        delete process.env.FURB_BUILD;
      }
      const before = said(place);
      expect(before.build).toBe("0.0.0+0ld0b0i1d000");
      const kept = await old.engine.bash("echo kept", { on: old.engine.root });
      await eventually(async () => (await old.engine.peek(kept)) !== null, "the first command to end");
      const faults: string[] = [];
      old.host.on("fault", (error: Error) => faults.push(error.message));
      // A command that runs as the window takes the host over lands before the old host ends its life.
      const late = await old.engine.bash("sleep 1; echo late", { on: old.engine.root });
      const start = performance.now();
      const { engine, host } = await openEngine(options);
      const took = performance.now() - start;
      console.log(`The takeover took ${Math.round(took)} ms.`);
      const after = said(place);
      expect(after.pid).not.toBe(before.pid);
      expect(after.build).toMatch(/\+[0-9a-f]{12}$/);
      expect(alive(before.pid)).toBe(false);
      expect(engine.root).toBe(old.engine.root);
      expect(await engine.peek(kept)).toMatchObject({ code: 0, stdout: { content: "kept\n" } });
      expect(await engine.peek(late)).toMatchObject({ code: 0, stdout: { content: "late\n" } });
      await eventually(() => faults.length > 0, "the window of the old host to hear the end");
      expect(faults[0]).toStartWith(`A window of the build ${after.build} took the life over`);
      expect(await old.engine.peek(kept).catch((error: Error) => error.message)).toBe(faults[0]);
      await host.end();
      await eventually(() => !alive(after.pid), "the new host to end");
    }),
);

test.skipIf(process.platform === "win32")(
  "a window of the old build that finds the host that took its build attaches to that host and takes the life back from no one",
  () =>
    demoLife(async (record, directory) => {
      const place = hostPlace(record);
      if (!place) throw new Error("No socket serves here.");
      const options = { demo: true, host: true, cwd: directory, record };
      const asOld = async () => {
        process.env.FURB_BUILD = "0.0.0+0ld0b0i1d000";
        try {
          return await openEngine(options);
        } finally {
          delete process.env.FURB_BUILD;
        }
      };
      await asOld();
      const { host } = await openEngine(options);
      const after = said(place);
      expect(after.from).toBe("0.0.0+0ld0b0i1d000");
      const again = await asOld();
      expect(said(place)).toMatchObject({ pid: after.pid, build: after.build });
      expect(alive(after.pid)).toBe(true);
      expect(again.engine.root).toBeString();
      await host.end();
      await eventually(() => !alive(after.pid), "the host to end");
    }),
);

test.skipIf(process.platform === "win32")(
  "a window that claims a message of more than 256 MiB loses its socket, and the host and the life go on",
  () =>
    demoLife(async (record, directory) => {
      const place = hostPlace(record);
      if (!place) throw new Error("No socket serves here.");
      const { engine, host } = await openEngine({ demo: true, host: true, cwd: directory, record });
      const socket = createConnection(place.socket);
      await new Promise<void>((resolve) => socket.once("connect", () => resolve()));
      const closed = new Promise<void>((resolve) => socket.once("close", () => resolve()));
      const head = Buffer.alloc(4);
      head.writeUInt32LE(0xffffffff, 0);
      socket.write(head);
      await closed;
      const about = said(place);
      expect(alive(about.pid)).toBe(true);
      const command = await engine.bash("echo up", { on: engine.root });
      await eventually(async () => (await engine.peek(command)) !== null, "the command to end");
      expect(await engine.peek(command)).toMatchObject({ code: 0, stdout: { content: "up\n" } });
      await host.end();
      await eventually(() => !alive(about.pid), "the host to end");
    }),
);

test.skipIf(process.platform === "win32")(
  "a command that runs past the wait of a host that hands its life over waits in the new host, and a wake ends it as interrupted",
  () =>
    demoLife(async (record, directory) => {
      const options = { demo: true, host: true, cwd: directory, record };
      process.env.FURB_BUILD = "0.0.0+0ld0b0i1d000";
      let old: Awaited<ReturnType<typeof openEngine>>;
      try {
        old = await openEngine(options);
      } finally {
        delete process.env.FURB_BUILD;
      }
      const cut = await old.engine.bash("sleep 60; echo never", { on: old.engine.root });
      await eventually(
        async () => (await old.engine.peek(cut)) === null && old.host.seq > 0,
        "the command to start",
      );
      const start = performance.now();
      const { engine, host } = await openEngine(options);
      expect(performance.now() - start).toBeGreaterThan(10000);
      await eventually(() => host.pending.has(cut), "the new host to hold the command as pending");
      expect(await engine.peek(cut)).toBeNull();
      await host.resume();
      await eventually(async () => (await engine.peek(cut)) !== null, "the wake to end the command");
      expect(await engine.peek(cut)).toMatchObject({
        is: "Refused",
        args: ['"sleep 60; echo never" was interrupted: the life that ran it ended before it did'],
      });
      await host.end();
    }),
);

test.skipIf(process.platform === "win32")(
  "a window over the network with the key to read sees the life and changes nothing in it",
  () =>
    demoLife(async (record, directory) => {
      const own = await openEngine({ demo: true, host: true, cwd: directory, record });
      const served = await own.host.serve();
      // The door opens once for the life of the host, so a second serve gives the same addresses.
      expect(await own.host.serve("0.0.0.0:1")).toEqual(served);
      expect(served.read).toMatch(/^furb:\/\/127\.0\.0\.1:\d+\/[0-9a-f-]{36}$/);
      expect(served.write).not.toBe(served.read);
      const command = await own.engine.bash("echo seen", { on: own.engine.root });
      await eventually(async () => (await own.engine.peek(command)) !== null, "the command to end");
      const reader = await openEngine({ attach: served.read, record: join(directory, "reader.jsonl") });
      expect([reader.engine.root, reader.host.role, own.host.role]).toEqual([
        own.engine.root,
        "read",
        "write",
      ]);
      expect(await reader.engine.peek(command)).toMatchObject({ code: 0, stdout: { content: "seen\n" } });
      await eventually(() => reader.host.seq > 0, "the state of the life");
      // The window keeps its view at its own record, and not at the record of the host.
      expect(reader.host.record).toBe(join(directory, "reader.jsonl"));
      const refused = (work: Promise<unknown>) => work.then(String, (error: Error) => error.message);
      expect(await refused(reader.engine.bash("echo no", { on: reader.engine.root }))).toBe(
        "This window reads the life only, so it cannot bash.",
      );
      expect(await refused(reader.host.look("life.jsonl", reader.engine.root))).toBe(
        "This window reads the life only, so it cannot look.",
      );
      expect(await refused(reader.host.serve())).toBe("This window reads the life only, so it cannot serve.");
      expect(await refused(reader.host.end())).toBe("This window reads the life only, so it cannot end.");
      expect(await reader.host.setup()).toBeNull();
      // The window that writes sees that no command came from the window that reads.
      const commands = (await own.engine.transcript({ on: own.engine.root })).filter(
        (fact) => fact[0] === "bash",
      );
      expect(commands.map((fact) => fact[1])).toEqual([command]);
      await reader.host.dispose();
      await own.host.end();
    }),
);

test.skipIf(process.platform === "win32")(
  "a window over the network with the key to write drives the life, and a host refuses a key of no role",
  () =>
    demoLife(async (record, directory) => {
      const own = await openEngine({ demo: true, host: true, cwd: directory, record });
      const served = await own.host.serve();
      const writer = await openEngine({ attach: served.write, record: join(directory, "writer.jsonl") });
      expect(writer.host.role).toBe("write");
      const command = await writer.engine.bash("echo driven", { on: writer.engine.root });
      await eventually(async () => (await own.engine.peek(command)) !== null, "the command to end");
      expect(await own.engine.peek(command)).toMatchObject({ code: 0, stdout: { content: "driven\n" } });
      const { host, port } = reachOf(served.read);
      const wrong = await openEngine({ attach: `furb://${host}:${port}/not-a-key` }).then(
        () => "opened",
        (error: Error) => error.message,
      );
      expect(wrong).toBe("This key opens no window of the life.");
      // A stranger that says it sends a long message before any hello loses its socket, and the life goes on.
      const stranger = createConnection({ host, port });
      await new Promise<void>((resolve) => stranger.once("connect", () => resolve()));
      const closed = new Promise<void>((resolve) => stranger.once("close", () => resolve()));
      stranger.on("error", () => {});
      const head = Buffer.alloc(4);
      head.writeUInt32LE(2 ** 30, 0);
      stranger.write(head);
      await closed;
      expect(await writer.engine.peek(command)).toMatchObject({ code: 0 });
      await writer.host.dispose();
      await own.host.end();
    }),
);

test("an address of a host reads as its host, its port and its key, and an address to listen at as a host and a port", () => {
  expect(reachOf("furb://[::1]:7700/k%2F1")).toEqual({ host: "::1", port: 7700, key: "k/1" });
  expect(() => reachOf("http://h:1/k")).toThrow(
    "http://h:1/k is no address of a host: it reads furb://host:port/key.",
  );
  expect(() => reachOf("furb://h:1/")).toThrow("is no address of a host");
  expect([
    listening(""),
    listening("7700"),
    listening(":7700"),
    listening("0.0.0.0:7700"),
    listening("[::]:1"),
  ]).toEqual([
    { name: "127.0.0.1", port: 0 },
    { name: "127.0.0.1", port: 7700 },
    { name: "127.0.0.1", port: 7700 },
    { name: "0.0.0.0", port: 7700 },
    { name: "::", port: 1 },
  ]);
  expect(() => listening("a:b:c")).toThrow("a:b:c is no address: give host:port, as 0.0.0.0:7700.");
});

test.skipIf(process.platform === "win32")(
  "a host that a signal ends tells no window that another window ended the life, and the window reaches a new host",
  () =>
    demoLife(async (record, directory) => {
      const place = hostPlace(record);
      if (!place) throw new Error("No socket serves here.");
      const { engine, host } = await openEngine({ demo: true, host: true, cwd: directory, record });
      const command = await engine.bash("echo kept", { on: engine.root });
      await eventually(async () => (await engine.peek(command)) !== null, "the command to end");
      const { pid } = said(place);
      const faults: string[] = [];
      host.on("fault", (error: Error) => faults.push(error.message));
      const reset = new Promise<void>((resolve) => host.once("reset", resolve));
      process.kill(pid, "SIGTERM");
      await reset;
      expect(faults).toEqual([]);
      expect(host.lost).toBe("");
      expect(said(place).pid).not.toBe(pid);
      expect(await engine.peek(command)).toMatchObject({ code: 0, stdout: { content: "kept\n" } });
      await host.end();
    }),
);

test.skipIf(process.platform === "win32")(
  "a window whose life another window ended says so, and opens the life again with reopen",
  () =>
    demoLife(async (record, directory) => {
      const options = { demo: true, host: true, cwd: directory, record };
      const one = await openEngine(options);
      const two = await openEngine(options);
      const command = await one.engine.bash("echo kept", { on: one.engine.root });
      await eventually(async () => (await two.engine.peek(command)) !== null, "the command to end");
      const heard = new Promise<void>((resolve) => two.host.once("fault", () => resolve()));
      await one.host.end();
      await heard;
      expect(two.host.lost).toBe("The life was ended from another window.");
      await two.host.reopen();
      expect(two.host.lost).toBe("");
      expect(await two.engine.peek(command)).toMatchObject({ code: 0, stdout: { content: "kept\n" } });
      await two.host.end();
    }),
);

test.skipIf(process.platform === "win32")(
  "a window stops a host that does not answer the end of the life, and opens the life again in a host of its own",
  () =>
    demoLife(async (record, directory) => {
      const place = hostPlace(record);
      if (!place) throw new Error("No socket serves here.");
      const { engine, host } = await openEngine({ demo: true, host: true, cwd: directory, record });
      const command = await engine.bash("echo kept", { on: engine.root });
      await eventually(async () => (await engine.peek(command)) !== null, "the command to end");
      const { pid } = said(place);
      const kept = [process.env.FURB_HOST_ENDING, process.env.FURB_HOST_GRACE];
      process.env.FURB_HOST_ENDING = "300";
      process.env.FURB_HOST_GRACE = "300";
      try {
        // A host that a signal froze hears nothing, so the end of the life never gets its answer.
        process.kill(pid, "SIGSTOP");
        await host.end();
        await eventually(() => !alive(pid), "the host to end");
        expect(host.lost).toBe("The host of the life was stopped from this window.");
        expect(existsSync(place.socket)).toBe(false);
        await host.reopen();
        expect(host.lost).toBe("");
        expect(said(place).pid).not.toBe(pid);
        expect(await engine.peek(command)).toMatchObject({ code: 0, stdout: { content: "kept\n" } });
        await host.end();
      } finally {
        for (const [name, value] of [
          ["FURB_HOST_ENDING", kept[0]],
          ["FURB_HOST_GRACE", kept[1]],
        ] as const)
          if (value === undefined) delete process.env[name];
          else process.env[name] = value;
      }
    }),
);

test.skipIf(process.platform === "win32")(
  "a window that finds the host of its life still opening waits for that host, and starts no second one",
  () =>
    demoLife(async (record, directory) => {
      const place = hostPlace(record);
      if (!place) throw new Error("No socket serves here.");
      const options = { demo: true, host: true, cwd: directory, record };
      const first = Bun.spawn(
        [process.execPath, join(import.meta.dir, "../src/host.ts"), JSON.stringify(options)],
        { stdout: "ignore", stderr: "ignore" },
      );
      await eventually(() => aboutOf(place) !== undefined, "the host to say of itself");
      const { engine, host } = await openEngine(options);
      expect(engine.root).toBeTruthy();
      expect(said(place).pid).toBe(first.pid);
      expect(alive(first.pid)).toBe(true);
      await host.end();
      await first.exited;
    }),
);

/** A folder of its own for the place of a host that no process serves. */
async function quietPlace(use: (place: Place, directory: string) => Promise<void>): Promise<void> {
  const directory = await mkdtemp(join(tmpdir(), "furb-about-"));
  try {
    const place = hostPlace(join(directory, "life.jsonl"));
    if (!place) throw new Error("No socket serves here.");
    await use(place, directory);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}

const SAID = { pid: 4242, record: "life.jsonl", build: "b", engine: "e", protocol: 1, started: 7 };

test.skipIf(process.platform === "win32")(
  "a reader of what a host says of itself gets nothing, and raises nothing, for a file that is gone or not whole",
  () =>
    quietPlace(async (place) => {
      expect(aboutOf(place)).toBeUndefined();
      for (const part of ["", "{", '{"pid":4242,"rec', "null", "[]", '"pid"', '{"pid":"4242"}']) {
        await writeFile(place.about, part);
        expect(aboutOf(place)).toBeUndefined();
      }
      tellAbout(place, SAID);
      expect(aboutOf(place)).toEqual(SAID);
      rmSync(place.about);
      expect(aboutOf(place)).toBeUndefined();
    }),
);

test.skipIf(process.platform === "win32")(
  "a host that says of itself again and again shows a reader a whole file at each moment, and leaves no other file",
  () =>
    quietPlace(async (place, directory) => {
      tellAbout(place, SAID);
      const script = join(directory, "say.ts");
      await writeFile(
        script,
        `import { tellAbout } from ${JSON.stringify(join(import.meta.dir, "../src/hosting.ts"))};
const place = JSON.parse(process.argv[2] ?? "{}");
for (let n = 1; n <= 4000; n++) tellAbout(place, { pid: n, record: "life.jsonl", build: "b", engine: "e", protocol: 1, started: n });
`,
      );
      const writer = Bun.spawn([process.execPath, script, JSON.stringify(place)], {
        stdout: "inherit",
        stderr: "inherit",
      });
      let reads = 0;
      let torn = 0;
      while (writer.exitCode === null) {
        for (let n = 0; n < 100; n++) {
          reads++;
          if (!aboutOf(place)) torn++;
        }
        await Bun.sleep(0);
      }
      expect(await writer.exited).toBe(0);
      expect(reads).toBeGreaterThan(100);
      expect(torn).toBe(0);
      expect(aboutOf(place)?.pid).toBe(4000);
      const key = place.about.slice(place.about.lastIndexOf("/") + 1, -".json".length);
      expect(
        (await readdir(join(place.about, ".."))).filter((name) => name.startsWith(`${key}.`)).sort(),
      ).toEqual([`${key}.json`]);
    }),
);
