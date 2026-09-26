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

// Build the same system prompt as the Python World. No Python process is needed at runtime. The prompt goes out as
// UTF-8 bytes, since a text stream on Windows writes a line end as CRLF and a character in the code page of the
// system.
const source = Bun.spawnSync(
  [
    "uv",
    "run",
    "--no-sync",
    "python",
    "-c",
    "import sys; from furb.world import SYSTEM; sys.stdout.buffer.write(SYSTEM.encode())",
  ],
  { cwd: root },
);
if (source.exitCode) throw new Error(source.stderr.toString());
await Bun.write(new URL("system.json", import.meta.url), `${JSON.stringify(source.stdout.toString())}\n`);
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
