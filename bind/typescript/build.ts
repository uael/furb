import { constants } from "node:fs";
import { copyFile, rename, rm } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { program } from "./program.ts";

const root = fileURLToPath(new URL("../../", import.meta.url));
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
  ...(process.argv.includes("--release") ? ["--release"] : []),
];
const child = Bun.spawn(command, { cwd: root, stdout: "inherit", stderr: "inherit" });
if (await child.exited) process.exit(1);
// The program furb serves the daemon of the machine, furb host, in which each window of the TUI holds its lives, of
// the profile of the engine. Cargo names the program that it built, which goes to the place of the program of this tree.
const cargo = Bun.spawn(
  [
    "cargo",
    "build",
    "--package",
    "furb-cli",
    "--message-format",
    "json-render-diagnostics",
    ...(process.argv.includes("--release") ? ["--release"] : []),
  ],
  { cwd: root, stdout: "pipe", stderr: "inherit" },
);
const built = (await new Response(cargo.stdout).text())
  .split("\n")
  .filter((line) => line.startsWith("{"))
  .map((line) => JSON.parse(line))
  .findLast(
    (one) => one.reason === "compiler-artifact" && one.target?.name === "furb" && one.executable,
  )?.executable;
if ((await cargo.exited) || !built) process.exit(1);
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
