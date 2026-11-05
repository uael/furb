import { expect, test } from "bun:test";
import type { Fact } from "furb";
import { dispatchesFrom, queueDispatches, queueHash } from "../src/queue.ts";

/** A follow-up of the operator: its begin, the prompt that it makes, and the sent that confirms it. */
function followUp(key: string, prompt: string): Fact[] {
  const hash = queueHash("chain1", "str", `message ${key}`, "");
  return [
    ["queue", "chain1", "operator", "begin", key, hash],
    ["prompt", prompt, "operator", "chain1", "str", `message ${key}`, ""],
    ["queue", "chain1", "operator", "sent", key, prompt],
  ] as unknown as Fact[];
}

test("a read of the facts in parts finds the dispatches that a read of them all finds", () => {
  const facts = [
    ...followUp("a", "prompt1"),
    ["tell", "chain1", "chain1", ["x"]] as unknown as Fact,
    ...followUp("b", "prompt2"),
  ];
  const whole = queueDispatches(facts.map((fact) => [fact] as never));
  for (let cut = 0; cut <= facts.length; cut++) {
    const sent = new Map<string, string>();
    const at = dispatchesFrom(facts.slice(0, cut), sent);
    dispatchesFrom(facts, sent, at);
    expect([...sent]).toEqual([...whole]);
  }
  expect([...whole]).toEqual([
    ["a", "prompt1"],
    ["b", "prompt2"],
  ]);
});

test("a begin that is the last fact waits for the prompt that follows it", () => {
  const [begin, prompt] = followUp("a", "prompt1") as [Fact, Fact];
  const sent = new Map<string, string>();
  const at = dispatchesFrom([begin], sent);
  expect(at).toBe(0);
  expect(sent.size).toBe(0);
  expect(dispatchesFrom([begin, prompt], sent, at)).toBe(2);
  expect(sent.get("a")).toBe("prompt1");
});
