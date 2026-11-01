import { expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, utimesSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { stale, warning } from "../script/stale.ts";

const built = ["bind/typescript/index.cjs", "bind/typescript/dist/index.js"];

/** A checkout with its sources and its build, the build made after the sources. */
function checkout(): string {
  const root = mkdtempSync(join(tmpdir(), "furb-stale-"));
  for (const file of ["Cargo.toml", "src/lib.rs", "src/furb/engine.py", "bind/typescript/src/index.ts"])
    touch(root, file, 1000);
  for (const file of built) touch(root, file, 2000);
  return root;
}

/** Make a file with the time of its last change, in seconds. */
function touch(root: string, file: string, seconds: number) {
  const path = join(root, file);
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, "");
  utimesSync(path, seconds, seconds);
}

test("a build made after its sources is current", () => {
  expect(stale(checkout())).toEqual([]);
});

test("a source of the crate that changed after the build stales the native module", () => {
  const root = checkout();
  touch(root, "src/furb/engine.py", 3000);
  expect(stale(root)).toEqual(["src/furb/engine.py changed after bind/typescript/index.cjs"]);
});

test("a source of the TypeScript side that changed after the build stales the package", () => {
  const root = checkout();
  touch(root, "bind/typescript/src/index.ts", 3000);
  expect(stale(root)).toEqual(["bind/typescript/src/index.ts changed after bind/typescript/dist/index.js"]);
});

test("a test, a note and a cache that the build does not read stale nothing", () => {
  const root = checkout();
  touch(root, "src/engine.test.rs", 3000);
  touch(root, "src/furb/AGENTS.md", 3000);
  touch(root, "src/furb/__pycache__/engine.cpython-314.pyc", 3000);
  expect(stale(root)).toEqual([]);
});

test("a part of the build that is gone is missing", () => {
  const root = mkdtempSync(join(tmpdir(), "furb-stale-"));
  touch(root, "bind/typescript/index.cjs", 2000);
  expect(stale(root)).toEqual(["bind/typescript/dist/index.js is missing"]);
});

test("the warning names each stale part and the command that builds again", () => {
  expect(warning(["a changed after b"])).toBe(
    "The build is stale: a changed after b. Run bun run build, and trust no failure that reads the build before it.",
  );
});
