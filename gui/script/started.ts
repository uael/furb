// A host of a life started as the core of the app starts it, for a script that shows the UI in a browser: the
// program of gui/host on this bun, with its words, and the address of its socket from its first line.

import { join } from "node:path";

/** A host that runs, the address of its socket, and its end, which closes its stdin as the app does. */
export async function startHost(
  words: string[],
  env: Record<string, string | undefined> = process.env,
): Promise<{ url: string; stop(): void }> {
  const child = Bun.spawn(["bun", join(import.meta.dir, "..", "host", "main.ts"), ...words], {
    stdin: "pipe",
    stdout: "pipe",
    stderr: "inherit",
    env,
  });
  const reader = child.stdout.getReader();
  let first = "";
  while (!first.includes("\n")) {
    const { value, done } = await reader.read();
    if (done) throw new Error("The host ended before it said its address.");
    first += new TextDecoder().decode(value);
  }
  const { url } = JSON.parse(first.slice(0, first.indexOf("\n"))) as { url: string };
  return {
    url,
    stop: () => {
      child.stdin.end();
    },
  };
}
