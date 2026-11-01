import { expect, test } from "bun:test";
import { covering } from "../src/app/blocks.ts";

const NAME = /[\p{L}_][\p{L}\p{N}_]*/gu;

test("the name under the pointer is the one that covers its column, as the terminal draws the line", () => {
  expect(covering("x = [n0, n1]", 10, NAME)?.[0]).toBe("n1");
  expect(covering("x = [n0, n1]", 2, NAME)).toBeUndefined();
  expect(covering("日本 = name", 7, NAME)?.[0]).toBe("name");
  expect(covering("#read3 done", 2, /^#([\w@.]+)/g)?.[1]).toBe("read3");
});

test("the pointer over a line of half a million characters finds its name at once", () => {
  const line = `x = [${Array.from({ length: 60000 }, (_, i) => `n${i}`).join(", ")}]`;
  const start = performance.now();
  expect(covering(line, line.length - 4, NAME)?.[0]).toBe("n59999");
  expect(performance.now() - start).toBeLessThan(1000);
});
