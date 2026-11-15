import { expect, test } from "bun:test";
import { runners } from "../../script/run.ts";

test("the runs of the tests take a third of the cores, and each core on CI", () => {
  expect(runners({}, 14)).toBe(4);
  expect(runners({}, 2)).toBe(1);
  expect(runners({ CI: "true" }, 8)).toBe(8);
});

test("FURB_TEST_JOBS caps the runs that go at once, and never raises them past the cores", () => {
  expect(runners({ FURB_TEST_JOBS: "2" }, 8)).toBe(2);
  expect(runners({ FURB_TEST_JOBS: "16" }, 8)).toBe(8);
});

test("a FURB_TEST_JOBS that names no count of runs leaves a third of the cores", () => {
  expect(runners({ FURB_TEST_JOBS: "" }, 9)).toBe(3);
  expect(runners({ FURB_TEST_JOBS: "0" }, 9)).toBe(3);
  expect(runners({ FURB_TEST_JOBS: "many" }, 9)).toBe(3);
});
