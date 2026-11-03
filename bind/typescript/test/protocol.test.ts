import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { generated, typesPath } from "../script/protocol.ts";

test("the types of the protocol are the ones that its schema makes", async () => {
  expect(readFileSync(typesPath, "utf8")).toBe(await generated());
});
