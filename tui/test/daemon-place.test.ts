// The dial of the daemon has one home, in furb/skill: it gives nothing where no daemon listens, and a socket where one
// does.
import { expect, test } from "bun:test";
import { mkdtemp, rm } from "node:fs/promises";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { daemonPlace, dialed } from "furb/skill";

/** A folder of a short path, since a socket has a bound on its path. */
const short = () => mkdtemp(join(process.platform === "win32" ? tmpdir() : "/tmp", "furb-dial-"));

test("the one dial gives nothing where no daemon listens, and a socket where one does", async () => {
  const folder = await short();
  const place = daemonPlace(folder);
  try {
    expect(await dialed(place)).toBeUndefined();
    const server = createServer((socket) => socket.end());
    await new Promise<void>((done) => server.listen(place, done));
    try {
      const socket = await dialed(place);
      expect(socket).toBeDefined();
      socket?.destroy();
    } finally {
      await new Promise((done) => server.close(done));
    }
  } finally {
    await rm(folder, { recursive: true, force: true });
  }
});
