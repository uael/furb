import { expect, test } from "bun:test";
import { modelLabel } from "../src/format.ts";

test("a model has one label, its id and its effort after a slash, wherever it is named", () => {
  expect(modelLabel("opus", "high")).toBe("opus/high");
  expect(modelLabel("opus", "off")).toBe("opus");
  expect(modelLabel("opus")).toBe("opus");
});
