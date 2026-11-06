import { expect, test } from "bun:test";
import { plain, reason } from "../../src/life/plain.ts";
import { act } from "./acts.ts";

test("a message reads with no name of its exception, no directory of the life, and no period at its end", () => {
  const raw = "Refused('There is no file at /work/notes/test/shortcut.test.ts.')";
  expect(plain(raw, "/work/notes")).toBe("There is no file at test/shortcut.test.ts");
  expect(plain("Refused: There is no file at /work/notes/a.ts.", "/work/notes/")).toBe(
    "There is no file at a.ts",
  );
});

test("a message that names no exception keeps its first word", () => {
  expect(plain("Note: the build is slow")).toBe("Note: the build is slow");
});

test("a failed rung says its reason, and an act that did not fail says nothing", () => {
  const rung = act("rung1", "rung", "chain1", "prompt1", 3, 5, {
    run: { status: "failed", reason: "Refused: There is no file at /p/a.ts." },
  });
  expect(reason(rung, "/p")).toBe("There is no file at a.ts");
  expect(reason(act("read1", "read", "chain1", "rung1", 4, 5))).toBeUndefined();
});

test("a failed act that is no rung says the first argument of its exception", () => {
  const read = act("read1", "read", "chain1", "rung1", 4, 5, {
    value: { is: "Refused", args: ["No file."] },
  });
  expect(reason(read)).toBe("No file");
});
