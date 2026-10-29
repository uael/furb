import { expect, test } from "bun:test";
import { recordLabel } from "../src/format.ts";

test("a record reads as the time that it was made, in the time of the machine, and a name of another form stays as it is", () => {
  const made = new Date(Date.UTC(2026, 9, 28, 7, 45, 43));
  const clock = `${String(made.getHours()).padStart(2, "0")}:${String(made.getMinutes()).padStart(2, "0")}`;
  const label = recordLabel("2026-10-28T07-45-43.846Z-6213ee10");
  expect(label).toMatch(/^Oct (27|28|29), \d\d:\d\d$/);
  expect(label.endsWith(clock)).toBe(true);
  expect(recordLabel("notes")).toBe("notes");
  expect(recordLabel("2026-10-28T07-45-43.846Z-6213ee10.jsonl")).toBe(
    "2026-10-28T07-45-43.846Z-6213ee10.jsonl",
  );
});
