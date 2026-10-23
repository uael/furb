// The UI of the app in a browser, on a host of its own: `bun run web [words of the host]` starts the host, on the
// demo of the TUI when it is given no word, and the dev server of the UI, then prints the address of the page, which
// names the socket of the host. Ctrl+C ends both.

import { spawn } from "node:child_process";
import { join } from "node:path";
import { startHost } from "./started.ts";

const words = process.argv.slice(2);
const host = await startHost(words.length ? words : ["--demo"]);
const server = spawn(join(import.meta.dir, "..", "node_modules", ".bin", "vite"), [], {
  cwd: join(import.meta.dir, ".."),
  stdio: ["ignore", "ignore", "inherit"],
});
console.log(`Open http://localhost:5174/?host=${encodeURIComponent(host.url)}`);
const end = () => {
  server.kill();
  host.stop();
  process.exit();
};
for (const signal of ["SIGINT", "SIGTERM"] as const) process.once(signal, end);
