import { constants } from "node:fs";
import { copyFile, rename, rm } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { program } from "./program.ts";

const root = fileURLToPath(new URL("../../", import.meta.url));
const release = process.argv.includes("--release");
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
  // The engine and the program furb come from one invocation of cargo over the workspace, which the build of the
  // gates and the tests of the crate select too: a second invocation for the program alone resolved other features of
  // the dependencies and built most crates again, 11 minutes on Windows.
  "--",
  "--workspace",
];
const child = Bun.spawn(command, { cwd: root, stdout: "inherit", stderr: "inherit" });
if (await child.exited) process.exit(1);
// The program furb serves the daemon of the machine, furb host, in which each window of the TUI holds its lives, of
// the profile of the engine. It stands where cargo puts each program of the target, and goes to the place of the
// program of this tree.
const metadata = Bun.spawnSync(["cargo", "metadata", "--format-version", "1", "--no-deps"], { cwd: root });
if (metadata.exitCode) process.exit(1);
const built = join(
  JSON.parse(metadata.stdout.toString()).target_directory,
  process.env.CARGO_BUILD_TARGET ?? "",
  release ? "release" : "debug",
  process.platform === "win32" ? "furb.exe" : "furb",
);
// A copy on write where the file system has it, so the copy of a program of 100 MB takes no time and no space. It goes
// beside the program and then takes its name, so a daemon that runs the old program keeps it, as cargo keeps it.
await copyFile(built, `${program}.new`, constants.COPYFILE_FICLONE);
await rename(`${program}.new`, program);

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
