import { expect, test } from "bun:test";
import { guarded } from "../src/faults.ts";

test("a step that raises gives its message as a fault and raises nothing", () => {
  const faults: string[] = [];
  const run = guarded(
    () => {
      throw new Error("memory limit exceeded");
    },
    (fault) => faults.push(fault),
  );
  expect(run()).toBeUndefined();
  expect(faults).toEqual(["memory limit exceeded"]);
});

test("a step that raises a value that is no error gives it as text", () => {
  const faults: string[] = [];
  guarded(
    () => {
      throw "cut";
    },
    (fault) => faults.push(fault),
  )();
  expect(faults).toEqual(["cut"]);
});

test("a step that ends gives no fault", () => {
  const faults: string[] = [];
  let ran = 0;
  guarded(
    () => ran++,
    (fault) => faults.push(fault),
  )();
  expect([ran, faults]).toEqual([1, []]);
});
