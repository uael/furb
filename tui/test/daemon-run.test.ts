import { expect, test } from "bun:test";
import { mkdtempSync, rmSync } from "node:fs";
import { join } from "node:path";
import { daemonFolder, daemonPlace, dialed } from "../src/client.ts";
import { handedOver } from "../script/daemon.ts";

test("a run of the tests that a kill ends leaves no daemon of it", async () => {
  // A short folder, since the path of a socket has a bound.
  const cache = mkdtempSync("/tmp/fr");
  const env = { ...process.env, XDG_CACHE_HOME: cache };
  const place = daemonPlace(daemonFolder(env));
  const script = `import { ownDaemon } from ${JSON.stringify(join(import.meta.dir, "../script/daemon.ts"))};
await ownDaemon(process.env);
console.log("ready");
await Bun.sleep(600000);`;
  const run = Bun.spawn([process.execPath, "-e", script], { env, stdout: "pipe", stderr: "ignore" });
  const reader = run.stdout.getReader();
  for (let read = ""; !read.includes("ready"); ) {
    const { value, done } = await reader.read();
    if (done) throw new Error("the run ended before its daemon listened");
    read += new TextDecoder().decode(value);
  }
  run.kill("SIGKILL");
  await run.exited;
  let left = true;
  for (const until = Date.now() + 15000; Date.now() < until; await Bun.sleep(100)) {
    const socket = await dialed(place);
    if (!socket) {
      left = false;
      break;
    }
    socket.destroy();
  }
  if (left) await handedOver(place);
  rmSync(cache, { recursive: true, force: true });
  expect(left).toBe(false);
}, 30000);
