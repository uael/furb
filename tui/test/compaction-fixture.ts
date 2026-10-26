import type { Fact } from "furb";

/** An inert recording of a legacy continuation and an in-place cut. No word of this recording runs. */
export function compactionFacts(directory: string): Fact[] {
  const facts: Fact[] = [
    ["chain", "chain1", "operator", "", "Main", ""],
    ["enable", "chain1", "extensions", "compaction", "def compact(): pass", ""],
    ["stand", "stand1", "operator", "chain1"],
    ["done", "stand1", "", [[["operator", [], 200000]], directory, "operator"]],
    [
      "prompt",
      "prompt1",
      "operator",
      "chain1",
      "str",
      "Keep the work on one program.",
      "codex:gpt-6.1-sol/high",
    ],
    ["rung", "rung1", "prompt1", "chain1", "answer = 41", "", ""],
    ["done", "rung1", "", null],
    ["done", "prompt1", "", null],
    ["chain", "chain2", "rung1", "chain1", "Middle", "chain1"],
    [
      "rung",
      "rung2",
      "operator",
      "chain2",
      'restore("chain1")\n<s:summary>\nKeep answer = 41.\n</s:summary>\nreground()',
      "",
      "",
    ],
    ["done", "rung2", "", null],
    ["chain", "chain3", "rung2", "chain2", "Current", "chain2"],
    [
      "rung",
      "rung3",
      "operator",
      "chain3",
      'restore("chain2")\n<s:summary>\nKeep the answer and check it.\n</s:summary>\nreground()',
      "",
      "",
    ],
    ["done", "rung3", "", null],
    [
      "rung",
      "rung4",
      "operator",
      "chain3",
      'compact(summary="Keep the exact value and the pending question.")',
      "",
      "",
    ],
    ["prefix", "chain3", "rung4", []],
    [
      "rung",
      "rung5",
      "rung4",
      "chain3",
      "<s:summary>\nKeep the exact value and the pending question.\nThe next step verifies the answer. Keep the source of each decision, the exact value that the operator approved, and the pending question together. Check the original files before the next edit. Do not drop this final sentence.\n</s:summary>\nreground()",
      "",
      "",
    ],
    ["done", "rung5", "", null],
    ["done", "rung4", "", null],
    ["chain", "chain4", "operator", "", "Separate branch", "chain1"],
    ["prefix", "chain4", "chain4", []],
    ["rung", "rung6", "operator", "chain4", "other = 9", "", ""],
    ["done", "rung6", "", null],
    ["chain", "chain5", "rung5", "chain3", "Nested work", ""],
    ["rung", "rung7", "operator", "chain5", "nested = 1", "", ""],
    ["done", "rung7", "", null],
    ["chain", "chain6", "rung7", "chain5", "Nested middle", "chain5"],
    [
      "rung",
      "rung8",
      "operator",
      "chain6",
      'restore("chain5")\n<s:summary>\nKeep nested = 1.\n</s:summary>\nreground()',
      "",
      "",
    ],
    ["done", "rung8", "", null],
    ["chain", "chain7", "rung8", "chain6", "Nested current", "chain6"],
    [
      "rung",
      "rung9",
      "operator",
      "chain7",
      'restore("chain6")\n<s:summary>\nCheck the nested work.\n</s:summary>\nreground()',
      "",
      "",
    ],
    ["done", "rung9", "", null],
  ];
  return facts.flatMap((fact): Fact[] => {
    if (fact[0] !== "rung") return [fact];
    const id = `run${fact[1].replace("rung", "")}`;
    return [
      fact,
      ["run", id, String(fact[3]), String(fact[3]), fact[1], fact[4], ""],
      ["done", id, "", null],
    ];
  });
}
