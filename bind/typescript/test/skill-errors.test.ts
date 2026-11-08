import { expect, test } from "bun:test";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Closed, Connection, Refused } from "../src/skill.ts";

test("a refusal of the daemon is named by its kind and a request that its socket left is Closed, the two that the window and a skill share", () => {
  expect(new Refused("role", "No role.").name).toBe("role");
  expect(new Refused("refused", "No.").name).toBe("Refused");
  expect(new Closed("gone")).toMatchObject({ name: "Closed", message: "gone" });
  expect(new Closed("gone")).toBeInstanceOf(Error);
});

test("an error that comes on the socket takes the stack of the verb that asked it, which names its caller", async () => {
  const place = join(tmpdir(), `furb-stack-${process.pid}.sock`);
  const server = createServer((socket) => {
    socket.on("data", (chunk) => {
      for (const line of String(chunk).split("\n").filter(Boolean)) {
        const { id, verb } = JSON.parse(line) as { id: number; verb: string };
        const frame =
          verb === "hello"
            ? { frame: "reply", id, answer: { answer: "welcome", data: { peer: {} } } }
            : verb === "list"
              ? { frame: "refused", id, error: { error: "role" }, why: "No role." }
              : undefined;
        if (frame) socket.write(`${JSON.stringify(frame)}\n`);
        else socket.end();
      }
    });
  });
  await new Promise<void>((resolve) => server.listen(place, resolve));
  try {
    const connection = await Connection.dial(place, "window");
    const fromHere = async (verb: "list" | "get") =>
      connection
        .ask(verb === "list" ? { verb: "list" } : { verb: "get", life: "l", act: "a" })
        .catch((e: Error) => e) as Promise<Error>;
    const refused = await fromHere("list");
    expect(refused).toBeInstanceOf(Refused);
    expect(refused.stack).toContain("fromHere");
    const closed = await fromHere("get");
    expect(closed).toBeInstanceOf(Closed);
    expect(closed.stack).toContain("fromHere");
  } finally {
    server.close();
  }
});
