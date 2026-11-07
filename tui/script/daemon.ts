import { mkdirSync } from "node:fs";
import { PROTOCOL } from "furb/protocol";
import { daemonFolder, daemonPlace, dialed, furbProgram } from "../src/client.ts";

/** How long the daemon of a run has to listen, or a daemon of an earlier build to give its place, in milliseconds. */
const PATIENCE = 60000;

/** The daemon that a run of the tests owns in the cache of the tests, of the build that the run tests: a daemon of an
 * earlier build there gives its place first, so that no test reaches it. A life of a test ends as soon as its last
 * window lets it go, so that a test reads its record as that end left it. It gives the end of that daemon, which the
 * run says once its tests ran. A run that reopens a record soon after it let it go asks for a grace of some seconds. */
export async function ownDaemon(
  env: Record<string, string | undefined>,
  grace = 0,
): Promise<() => Promise<void>> {
  const folder = daemonFolder(env);
  const place = daemonPlace(folder);
  await handedOver(place);
  mkdirSync(folder, { recursive: true, mode: 0o700 });
  const served = Bun.spawn([furbProgram(env), "host", "--grace", String(grace)], {
    env,
    stdout: "ignore",
    stderr: "pipe",
  });
  // A run that a signal ends ends its daemon first, which would else outlive it with no parent.
  for (const signal of ["SIGINT", "SIGTERM", "SIGHUP"] as const)
    process.once(signal, () => {
      served.kill();
      process.exit(128 + (signal === "SIGHUP" ? 1 : signal === "SIGINT" ? 2 : 15));
    });
  // The run holds one peer of its daemon for as long as it lives, so that the daemon ends once its grace passed after
  // the run is gone, even by a kill that no handler hears.
  let held: Awaited<ReturnType<typeof dialed>>;
  for (const until = Date.now() + PATIENCE; ; ) {
    held = await dialed(place);
    if (held) {
      held.on("error", () => {});
      held.unref();
      break;
    }
    if (served.exitCode !== null || Date.now() > until) {
      served.kill();
      throw new Error(`The daemon of the tests did not listen: ${await new Response(served.stderr).text()}`);
    }
    await Bun.sleep(25);
  }
  return async () => {
    held?.destroy();
    served.kill();
    await served.exited;
  };
}

/** The daemon that listens at a place, if one does, asked to give its place, once it closed. */
export async function handedOver(place: string): Promise<void> {
  const socket = await dialed(place);
  if (!socket) return;
  const closed = new Promise<void>((resolve) => socket.once("close", () => resolve()));
  socket.on("error", () => {});
  socket.on("data", () => {});
  socket.write(`${JSON.stringify({ id: 0, verb: "hello", protocol: PROTOCOL, kind: "daemon" })}\n`);
  socket.write(`${JSON.stringify({ id: 1, verb: "handover" })}\n`);
  const late = setTimeout(() => socket.destroy(), PATIENCE);
  await closed;
  clearTimeout(late);
  // The daemon that gave its place stops to listen before it ends, and a daemon that starts while it listens refuses.
  for (const until = Date.now() + PATIENCE; ; await Bun.sleep(25)) {
    const still = await dialed(place);
    if (!still) return;
    still.destroy();
    if (Date.now() > until) throw new Error(`The daemon at ${place} gave its place, and still listens.`);
  }
}
