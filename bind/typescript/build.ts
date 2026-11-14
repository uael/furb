import { rm } from "node:fs/promises";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../../", import.meta.url));
const release = process.argv.includes("--release");
// napi names a target to cargo for each build, the one of the host when none is asked, and cargo puts what it builds
// for a named target in a folder of that name. So the build names its target once, to napi and to the path below.
const target =
  process.env.CARGO_BUILD_TARGET ??
  Bun.spawnSync(["rustc", "--print", "host-tuple"], { cwd: root }).stdout.toString().trim();
const command = [
  process.execPath,
  "x",
  "--no-install",
  "napi",
  "build",
  "--manifest-path",
  "Cargo.toml",
  "--features",
  "typescript",
  "--package-json-path",
  "bind/typescript/package.json",
  "--output-dir",
  "bind/typescript",
  "--target",
  target,
  // The engine is furb.<system>.node, as the package of the engine of each system names it, and index.cjs, which napi
  // writes, loads it from beside itself or from that package.
  "--platform",
  "--js",
  "index.cjs",
  "--dts",
  "index.d.cts",
  // The dev profile optimizes every dependency, the interpreter among them, so a debug build runs a word as fast as a
  // release and builds many times faster: debug unless asked.
  ...(release ? ["--release"] : []),
  // The engine comes from one invocation of cargo over the workspace, which the build of the gates and the tests of
  // the crate select too: a selection of the crate alone resolves other features of the dependencies, and the tests
  // then build most crates again, 11 minutes on Windows. The window stays out: no part of this package reads it, and
  // its build needs Zig or the native library of OpenTUI, which only a build of the window has.
  "--",
  "--workspace",
  "--exclude",
  "furb-window",
];
const child = Bun.spawn(command, { cwd: root, stdout: "inherit", stderr: "inherit" });
if (await child.exited) process.exit(1);

await rm(new URL("dist", import.meta.url), { recursive: true, force: true });
const javascript = Bun.spawn(
  [process.execPath, "x", "--no-install", "tsc", "-p", "bind/typescript/tsconfig.json"],
  {
    cwd: root,
    stdout: "inherit",
    stderr: "inherit",
  },
);
if (await javascript.exited) process.exit(1);
