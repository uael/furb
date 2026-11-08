// The place of the daemon and the dial of its socket have one home, in furb/skill: a window dials the place that a
// skill dials for one folder, and the one dial gives nothing where no daemon listens and a socket where one does.
import { expect, test } from "bun:test";
import { mkdtemp, rm } from "node:fs/promises";
import { createServer } from "node:net";
import { join } from "node:path";
import { daemonPlace, dialed } from "furb/skill";
import { Daemon } from "../src/client.ts";

test("a window and a skill dial one place for one folder, from one function", () => {
  const folder = join("/tmp", "furb-place");
  expect(new Daemon(folder).place).toBe(daemonPlace(folder));
});

test("the one dial gives nothing where no daemon listens, and a socket where one does", async () => {
  const folder = await mkdtemp(join("/tmp", "furb-dial-"));
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
