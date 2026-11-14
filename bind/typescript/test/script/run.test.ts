import { expect, test } from "bun:test";
import { runners } from "../../script/run.ts";

test("the runs of the tests go one for each core", () => {
  expect(runners({}, 8)).toBe(8);
});

test("FURB_TEST_JOBS caps the runs that go at once, and never raises them past the cores", () => {
  expect(runners({ FURB_TEST_JOBS: "2" }, 8)).toBe(2);
  expect(runners({ FURB_TEST_JOBS: "16" }, 8)).toBe(8);
});

test("a FURB_TEST_JOBS that names no count of runs leaves one for each core", () => {
  expect(runners({ FURB_TEST_JOBS: "" }, 8)).toBe(8);
  expect(runners({ FURB_TEST_JOBS: "0" }, 8)).toBe(8);
  expect(runners({ FURB_TEST_JOBS: "many" }, 8)).toBe(8);
});
