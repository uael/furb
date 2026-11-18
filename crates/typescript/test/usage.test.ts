import { expect, test } from "bun:test";
import { unmarked, usageOf } from "../src/index.ts";

test("the usage of an answer reads a float that the life marked as a number and counts a part that is missing as nothing", () => {
  const marked = { is: "float", args: ["0"] };
  expect(usageOf([10, 2, 0, 0, marked])).toEqual([10, 2, 0, 0, 0]);
  expect(usageOf([10, 2, 0, 0, { is: "float", args: ["1.5"] }])).toEqual([10, 2, 0, 0, 1.5]);
  expect(usageOf([10, 2])).toEqual([10, 2, 0, 0, 0]);
  expect(usageOf([10, 2, 0, 0, { is: "float", args: ["NaN"] }])[4]).toBe(0);
});

test("a float that the life marked is the number it stands for, whatever holds it", () => {
  expect(
    unmarked({
      words: [
        { is: "float", args: ["3"] },
        { is: "float", args: ["-inf"] },
      ],
    }),
  ).toEqual({
    words: [3, -Infinity],
  });
});
