import { rm } from "node:fs/promises";
import { fileURLToPath } from "node:url";

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
  "--dts",
  "index.d.cts",
  // The engine runs in the interpreter, which a debug build runs many times slower: release unless asked.
  ...(process.argv.includes("--debug") ? [] : ["--release"]),
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
