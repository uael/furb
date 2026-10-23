import { expect, test } from "bun:test";
import { existsSync, readFileSync, rmSync } from "node:fs";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { openEngine } from "../src/bridge.ts";
import { type About, hostPlace } from "../src/hosting.ts";

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
    if (place && existsSync(place.about)) {
      const { pid } = JSON.parse(readFileSync(place.about, "utf8")) as About;
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
      const about = JSON.parse(readFileSync(place.about, "utf8")) as About;
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
      expect(faults).toEqual(["The life ended in another window."]);
      expect(await two.engine.peek(command).catch((error: Error) => error.message)).toBe(
        "The life ended in another window.",
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
      const { pid } = JSON.parse(readFileSync(place.about, "utf8")) as About;
      const reset = new Promise<void>((resolve) => host.once("reset", resolve));
      process.kill(pid, "SIGKILL");
      await reset;
      const next = JSON.parse(readFileSync(place.about, "utf8")) as About;
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
      const { pid } = JSON.parse(readFileSync(place.about, "utf8")) as About;
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
