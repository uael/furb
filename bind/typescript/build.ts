import { rm } from "node:fs/promises";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../../", import.meta.url));
const command = [
  "bunx",
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
  "--platform",
  "--js",
  "index.cjs",
  "--dts",
  "index.d.cts",
  ...(process.argv.includes("--release") ? ["--release"] : []),
];
const child = Bun.spawn(command, { cwd: root, stdout: "inherit", stderr: "inherit" });
if (await child.exited) process.exit(1);

// Build the same system prompt as the Python World. No Python process is needed at runtime.
const source = Bun.spawnSync(
  ["uv", "run", "--no-sync", "python", "-c", "from furb.world import SYSTEM; print(SYSTEM, end='')"],
  { cwd: root },
);
if (source.exitCode) throw new Error(source.stderr.toString());
await Bun.write(new URL("system.json", import.meta.url), `${JSON.stringify(source.stdout.toString())}\n`);
await rm(new URL("dist", import.meta.url), { recursive: true, force: true });
const javascript = Bun.spawn(["bunx", "--no-install", "tsc", "-p", "bind/typescript/tsconfig.json"], {
  cwd: root,
  stdout: "inherit",
  stderr: "inherit",
});
if (await javascript.exited) process.exit(1);
