import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { isBuiltin } from "node:module";
import { tmpdir } from "node:os";
import { dirname, extname, join } from "node:path";
import type { Fact } from "@furb/engine";
import { HostView } from "../src/bridge.ts";

test("the view of the host takes more facts than one call takes as arguments", () => {
  const view = new HostView(async () => null);
  const count = 700_000;
  view.update({
    completed: 0,
    cost: 0,
    directory: "/tmp",
    imageDirectory: "/tmp/images",
    actor: "operator",
    effort: "low",
    roster: [],
    facts: new Array<Fact>(count).fill(["done", "x", "world", null]),
    prompts: [],
    streams: [],
    pending: [],
    changes: 0,
  });
  expect(view.facts).toHaveLength(count);
});

test("the view of the host counts the file changes of the session", () => {
  const view = new HostView(async () => null);
  view.update({
    completed: 0,
    cost: 0,
    directory: "/tmp",
    imageDirectory: "/tmp/images",
    actor: "operator",
    effort: "low",
    roster: [],
    facts: [],
    prompts: [],
    streams: [],
    pending: [],
    changes: 3,
  });
  expect(view.changes).toBe(3);
});

test("a session whose life does not open ends its worker, so the process that asked it can exit", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-open-fail-"));
  const record = join(directory, "damaged.jsonl");
  const script = join(directory, "open.ts");
  try {
    await writeFile(record, "not json\n");
    await writeFile(
      script,
      `import { openEngine } from ${JSON.stringify(join(import.meta.dir, "../src/bridge.ts"))};
await openEngine({ cwd: ${JSON.stringify(directory)}, record: ${JSON.stringify(record)}, demo: true }).catch((error) =>
  console.log(error.message),
);
`,
    );
    const child = Bun.spawn([process.execPath, script], { stdout: "pipe", stderr: "pipe" });
    const ended = await Promise.race([child.exited, Bun.sleep(15000).then(() => "still running")]);
    child.kill();
    expect(ended).toBe(0);
    expect(await new Response(child.stdout).text()).toContain("Invalid record entry");
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}, 30000);

/** Every module that an entry loads, by its own imports and those of its packages, and the file that names each one. */
function importsOf(entry: string): Map<string, string> {
  const named = new Map<string, string>();
  const files = new Set([entry]);
  // The transpiler of bun drops an import of types alone, as the runtime does.
  const scanners = { ts: new Bun.Transpiler({ loader: "ts" }), js: new Bun.Transpiler({ loader: "js" }) };
  for (const file of files)
    for (const { path } of (extname(file) === ".ts" ? scanners.ts : scanners.js).scanImports(
      readFileSync(file, "utf8"),
    )) {
      if (isBuiltin(path) || path.startsWith("bun")) continue;
      named.set(path, file);
      // A package of another system does not resolve here, and it keeps its name above.
      const resolved = (() => {
        try {
          return Bun.resolveSync(path, dirname(file));
        } catch {
          return undefined;
        }
      })();
      if (resolved && [".ts", ".js", ".mjs", ".cjs"].includes(extname(resolved))) files.add(resolved);
    }
  return named;
}

test("no worker of the TUI loads OpenTUI, so its native library stays with the thread that draws", () => {
  for (const worker of ["worker.ts", "record-worker.ts"]) {
    const named = importsOf(join(import.meta.dir, "../src", worker));
    expect(named.size).toBeGreaterThan(1);
    expect([...named].filter(([path]) => path.startsWith("@opentui/"))).toEqual([]);
  }
}, 30000);
