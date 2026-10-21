import { expect, test } from "bun:test";
import { join } from "node:path";

const example = join(import.meta.dir, "..", "..", "..", "docs", "examples", "typescript.ts");

test("the example of TypeScript runs as it stands, and every claim it asserts holds", async () => {
  const run = Bun.spawn([process.execPath, example], { stdout: "pipe", stderr: "pipe" });
  const [code, out, err] = await Promise.all([
    run.exited,
    new Response(run.stdout).text(),
    new Response(run.stderr).text(),
  ]);
  expect(err).toBe("");
  expect(code).toBe(0);
  expect(out.trim().split("\n").at(-1)).toBe(
    "the second process resumed it from the record, and every claim holds",
  );
});
