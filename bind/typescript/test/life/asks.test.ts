import { expect, test } from "bun:test";
import { aimed, beside, choicesOf, desk, gist, wants } from "../../src/life/asks.ts";
import { act } from "./acts.ts";

const label = (id: string) => (id === "chain1" ? "Main" : `life3/${id}`);

test("a bool gives yes and no, a Literal gives each of its values as the operator writes it, and any other shape none", () => {
  expect(choicesOf("bool")).toEqual(["yes", "no"]);
  expect(choicesOf("Literal['fast', 'safe']")).toEqual(["fast", "safe"]);
  expect(choicesOf(`Literal["a, b", 'it\\'s', 3, True, None]`)).toEqual([
    "a, b",
    "it's",
    "3",
    "True",
    "None",
  ]);
  expect(choicesOf("typing.Literal['one']")).toEqual(["one"]);
  for (const shape of ["str", "int", "float", "list", "dict", "None", "list[str]"])
    expect(choicesOf(shape)).toEqual([]);
});

test("what an ask wants reads as its choices, its shape, or nothing for a notice", () => {
  expect(wants({ notice: false, shape: "bool", choices: ["yes", "no"] })).toBe("yes or no");
  expect(wants({ notice: false, shape: "Literal", choices: ["a", "b", "c"] })).toBe("a, b or c");
  expect(wants({ notice: false, shape: "int", choices: [] })).toBe("int");
  expect(wants({ notice: true, shape: "None", choices: [] })).toBe("nothing");
});

test("the desk holds the questions and the notices of every chain, the oldest first, and drops a question whose act is done", () => {
  const acts = [
    act("chain1", "chain", "", "operator", 1),
    act("chain2", "chain", "", "rung1", 2),
    act("prompt1", "prompt", "chain2", "rung2", 3, undefined, { words: ["str", "Which base?", "operator"] }),
    act("prompt2", "prompt", "chain1", "rung1", 4, 4, { words: ["None", "The build is green.", "operator"] }),
    act("prompt3", "prompt", "chain1", "rung1", 5, undefined, { words: ["bool", "Merge it?", "operator"] }),
    act("prompt4", "prompt", "chain2", "rung2", 6, 7, { words: ["int", "How many?", "operator"] }),
  ];
  const { asks, questions, notices } = desk({
    acts,
    prompts: [
      { id: "prompt3", shape: "bool", message: "Merge it?" },
      { id: "prompt1", shape: "str", message: "Which base?" },
      { id: "prompt4", shape: "int", message: "How many?" },
      { id: "prompt9", shape: "str", message: "Not in the acts yet." },
    ],
    notices: [{ id: "prompt2", message: "The build is green." }],
    label,
  });
  expect(asks.map((ask) => [ask.id, ask.label, ask.notice, ask.choices])).toEqual([
    ["prompt1", "life3/chain2", false, []],
    ["prompt2", "Main", true, []],
    ["prompt3", "Main", false, ["yes", "no"]],
  ]);
  expect([questions, notices]).toEqual([2, 1]);
});

test("a prompt of a Literal in a model of a life asks for one of its values", () => {
  const acts = [
    act("chain1", "chain", "", "operator", 1),
    act("prompt1", "prompt", "chain1", "rung1", 2, undefined, {
      words: ["Literal['squash', 'rebase']", "How should #305 merge?", "operator"],
    }),
  ];
  const [ask] = desk({
    acts,
    prompts: [{ id: "prompt1", message: "How should #305 merge?" }],
    notices: [],
    label,
  }).asks;
  expect(ask?.shape).toBe("Literal['squash', 'rebase']");
  expect(ask?.choices).toEqual(["squash", "rebase"]);
  expect(ask && wants(ask)).toBe("squash or rebase");
});

test("the ask aimed on a chain is the one pointed at, else its oldest question, else its oldest notice", () => {
  const acts = [
    act("chain1", "chain", "", "operator", 1),
    act("chain2", "chain", "", "rung1", 2),
    act("prompt1", "prompt", "chain1", "rung1", 3, 3, { words: ["None", "Note", "operator"] }),
    act("prompt2", "prompt", "chain1", "rung1", 4, undefined, { words: ["str", "One", "operator"] }),
    act("prompt3", "prompt", "chain1", "rung1", 5, undefined, { words: ["str", "Two", "operator"] }),
    act("prompt4", "prompt", "chain2", "rung2", 6, 6, { words: ["None", "Other", "operator"] }),
  ];
  const { asks } = desk({
    acts,
    prompts: [
      { id: "prompt2", message: "One" },
      { id: "prompt3", message: "Two" },
    ],
    notices: [
      { id: "prompt1", message: "Note" },
      { id: "prompt4", message: "Other" },
    ],
    label,
  });
  expect(aimed(asks, "chain1")?.id).toBe("prompt2");
  expect(aimed(asks, "chain1", "prompt3")?.id).toBe("prompt3");
  expect(aimed(asks, "chain1", "prompt4")?.id).toBe("prompt2");
  expect(aimed(asks, "chain2")?.id).toBe("prompt4");
  expect(aimed(asks, "chain3")).toBeUndefined();
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
