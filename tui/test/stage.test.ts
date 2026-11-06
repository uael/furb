import { expect, test } from "bun:test";
import { mkdtemp, readdir, readFile, rm, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

test.skipIf(process.platform === "win32")(
  "two runs of the gallery with two roots hold their homes at once",
  async () => {
    const directory = await mkdtemp(join(tmpdir(), "furb-stage-"));
    try {
      const marks = join(directory, "marks");
      const script = join(directory, "run.ts");
      // Each run says that it holds its home, then ends only once the other run holds its own home too.
      await writeFile(
        script,
        `import { mkdirSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { home } from ${JSON.stringify(join(import.meta.dir, "../script/stage.ts"))};
const place = await home("furb-stage-test");
const marks = ${JSON.stringify(marks)};
mkdirSync(marks, { recursive: true });
writeFileSync(join(marks, String(process.pid)), place);
for (const deadline = Date.now() + 20000; readdirSync(marks).length < 2; await Bun.sleep(50))
  if (Date.now() > deadline) process.exit(3);
process.exit(0);
`,
      );
      const roots = [join(directory, "one"), join(directory, "two")];
      const env = { ...process.env };
      delete env.FURB_GALLERY_HOME;
      delete env.FURB_GALLERY_PARENT;
      const runs = roots.map((root) =>
        Bun.spawn([process.execPath, script], {
          env: { ...env, FURB_GALLERY_ROOT: root },
          stderr: "inherit",
        }),
      );
      expect(await Promise.all(runs.map((run) => run.exited))).toEqual([0, 0]);
      const homes = await Promise.all(
        (await readdir(marks)).map((one) => readFile(join(marks, one), "utf8")),
      );
      expect(homes.map((one) => one.split("/").slice(-2).join("/")).sort()).toEqual([
        "one/furb-stage-test",
        "two/furb-stage-test",
      ]);
      expect((await readdir(roots[0] ?? "")).length + (await readdir(roots[1] ?? "")).length).toBe(0);
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  },
);

test.skipIf(process.platform === "win32")(
  "a gallery root of 90 characters still reaches its daemon on a socket that fits",
  async () => {
    const directory = await mkdtemp("/tmp/furb-stage-");
    try {
      const marks = join(directory, "marks");
      const script = join(directory, "run.ts");
      // The child binds the socket where the daemon of furb listens, in the cache that the home gives it, and dials it.
      await writeFile(
        script,
        `import { mkdirSync, writeFileSync } from "node:fs";
import { createConnection, createServer } from "node:net";
import { join } from "node:path";
import { daemonFolder, daemonPlace } from ${JSON.stringify(join(import.meta.dir, "../src/client.ts"))};
import { home } from ${JSON.stringify(join(import.meta.dir, "../script/stage.ts"))};
await home("furb-stage-test");
const place = daemonPlace(daemonFolder());
mkdirSync(join(place, ".."), { recursive: true, mode: 0o700 });
const server = createServer((socket) => socket.end("hello"));
await new Promise<void>((done, fail) => server.once("error", fail).listen(place, done));
const said = await new Promise<string>((done, fail) => {
  const socket = createConnection(place);
  let text = "";
  socket.on("data", (data) => (text += data)).on("end", () => done(text)).on("error", fail);
});
mkdirSync(${JSON.stringify(marks)}, { recursive: true });
writeFileSync(join(${JSON.stringify(marks)}, "place"), JSON.stringify({ place, said, cache: process.env.XDG_CACHE_HOME ?? "" }));
server.close();
process.exit(said === "hello" ? 0 : 3);
`,
      );
      const pad = 90 - directory.length - 1;
      expect(pad).toBeGreaterThan(0);
      const root = join(directory, "x".repeat(pad));
      expect(root.length).toBe(90);
      const env = { ...process.env };
      delete env.FURB_GALLERY_HOME;
      delete env.FURB_GALLERY_PARENT;
      delete env.XDG_CACHE_HOME;
      const run = Bun.spawn([process.execPath, script], {
        env: { ...env, FURB_GALLERY_ROOT: root },
        stderr: "inherit",
      });
      expect(await run.exited).toBe(0);
      const got = JSON.parse(await readFile(join(marks, "place"), "utf8")) as {
        place: string;
        said: string;
        cache: string;
      };
      expect(got.said).toBe("hello");
      expect(Buffer.byteLength(got.place)).toBeLessThan(104);
      // The private cache of the child ends with the run.
      expect(got.cache.startsWith("/tmp/furb-sock-")).toBe(true);
      expect(
        await stat(got.cache).then(
          () => true,
          () => false,
        ),
      ).toBe(false);
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  },
);
