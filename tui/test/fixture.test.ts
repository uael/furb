import { expect, test } from "bun:test";
import { fixture, momentOf, scrubbed } from "../script/fixture.ts";

/** The log of a life of three commands, each a rung, a bash and its done, as the lines of the file hold them. */
const log = [1, 2, 3].flatMap((n) => [
  JSON.stringify(["rung", `rung${n}`, "operator", "chain1", "x = 1", "", ""]),
  JSON.stringify(["bash", `bash${n}`, `rung${n}`, "chain1", `echo ${n}`]),
  JSON.stringify([
    "done",
    `bash${n}`,
    "World",
    { is: "Exit", code: 0, stdout: { content: "" }, stderr: { content: "" } },
  ]),
]);

async function* lines(all: string[]): AsyncGenerator<string> {
  yield* all;
}

test("a fixture reads the log of a life one line at a time and shows its end by default", async () => {
  const { facts, header } = await fixture(lines(log));
  expect(facts.length).toBe(9);
  expect(header).toEqual({ recorded: true, cut: 9, of: 9 });
});

test("a fixture cut at a count of facts stops there, and a line that does not parse, as a last line half written, is skipped", async () => {
  const { facts, header } = await fixture(
    lines([...log.slice(0, 4), '["done", "bash2"', ...log.slice(4)]),
    4,
  );
  expect(facts.map((fact) => fact[1])).toEqual(["rung1", "bash1", "bash1", "rung2"]);
  expect(header.cut).toBe(4);
  expect(momentOf()).toBe("end");
  expect(momentOf("busiest")).toBe("busiest");
  expect(momentOf("12")).toBe(12);
  expect(() => momentOf("0")).toThrow("no moment");
});

test("each fact of a fixture stays JSON after the scrub, though a secret or a home stands beside a quote or a backslash", () => {
  const fact = [
    "bash",
    "bash1",
    "rung1",
    "chain1",
    'API_TOKEN=abc" cd /Users/alice\\ && echo "a@b.co"',
    { path: "/home/bob" },
  ];
  const line = JSON.stringify(scrubbed(fact));
  expect(JSON.parse(line)).toEqual([
    "bash",
    "bash1",
    "rung1",
    "chain1",
    'API_TOKEN=[key] cd ~ && echo "someone@example.com"',
    { path: "~" },
  ]);
});
