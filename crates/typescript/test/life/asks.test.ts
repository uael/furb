import { expect, test } from "bun:test";
import { type Ask, aimed, beside, gist, wants } from "../../src/life/asks.ts";

/** An ask as the crate folds it, with the label of its chain: a question of a text, or a notice. */
const ask = (id: string, chain: string, began: number, notice = false): Ask => ({
  id,
  chain,
  label: chain === "chain1" ? "Main" : `life3/${chain}`,
  by: "rung1",
  shape: notice ? "None" : "str",
  message: id,
  notice,
  choices: [],
  ticks: [],
  hint: "",
  began,
});

test("what an ask wants reads as its choices, its values, its shape, or nothing for a notice", () => {
  const given = { notice: false, shape: "bool", choices: ["yes", "no"], ticks: [] };
  expect(wants(given)).toBe("yes or no");
  expect(wants({ ...given, shape: "Literal", choices: ["a", "b", "c"] })).toBe("a, b or c");
  expect(wants({ ...given, shape: "int", choices: [] })).toBe("int");
  expect(wants({ ...given, shape: "None", choices: [], notice: true })).toBe("nothing");
  expect(
    wants({
      ...given,
      shape: "list[Literal['a', 'b']]",
      choices: [],
      ticks: [
        { shown: "a", json: '"a"' },
        { shown: "b", json: '"b"' },
      ],
    }),
  ).toBe("any of a or b");
});

test("the ask aimed on a chain is the one pointed at, else its oldest question, else its oldest notice", () => {
  const asks = [
    ask("prompt1", "chain1", 3, true),
    ask("prompt2", "chain1", 4),
    ask("prompt3", "chain1", 5),
    ask("prompt4", "chain2", 6, true),
  ];
  expect(aimed(asks, "chain1")?.id).toBe("prompt2");
  expect(aimed(asks, "chain1", "prompt3")?.id).toBe("prompt3");
  expect(aimed(asks, "chain1", "prompt4")?.id).toBe("prompt2");
  expect(aimed(asks, "chain2")?.id).toBe("prompt4");
  expect(aimed(asks, "chain3")).toBeUndefined();
  expect(aimed(asks, "chain2")?.label).toBe("life3/chain2");
  expect(beside(asks, "prompt4", 1)?.id).toBe("prompt1");
  expect(beside(asks, "prompt1", -1)?.id).toBe("prompt4");
  expect(beside(asks, "gone", 1)?.id).toBe("prompt1");
  expect(beside(asks, "gone", -1)?.id).toBe("prompt4");
  expect(beside([], "prompt1", 1)).toBeUndefined();
});

test("the gist of a message is its first line that holds words, with the next one when the first leads in with a colon", () => {
  expect(gist("\n## Merge it?\n\nThe checks are green.")).toBe("Merge it?");
  expect(gist("chain30 asks the operator:\n\nA question on the **item** answers.\n\nMore.")).toBe(
    "chain30 asks the operator: A question on the item answers.",
  );
  expect(gist("Choose one:")).toBe("Choose one:");
});
