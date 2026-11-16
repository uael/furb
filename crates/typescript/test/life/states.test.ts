import { expect, test } from "bun:test";
import { busy } from "../../src/life/states.ts";

test("a state that thinks, runs or delegates is busy, and a done one is not", () => {
  expect(["thinks", "runs", "delegates", "done"].map(busy)).toEqual([true, true, true, false]);
});
