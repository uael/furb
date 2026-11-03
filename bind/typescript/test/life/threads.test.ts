import { expect, test } from "bun:test";
import type { LiveAct } from "../../src/life/acts.ts";
import {
  attentionOf,
  type ChainThread,
  chainThreads,
  concerns,
  nestThreads,
} from "../../src/life/threads.ts";
import { act } from "./acts.ts";

const prompt = (id: string, began: number, more: Partial<LiveAct> = {}) =>
  act(id, "prompt", "chain1", "operator", began, undefined, { words: ["str", `${id} says`, ""], ...more });

test("what a host keeps of a prompt belongs to that act, so a prompt of the same id in another session or another branch is read again and joins its thread", () => {
  const known = new WeakMap<LiveAct, { message: string | undefined; title: string }>();
  const list = (acts: LiveAct[]) =>
    chainThreads(acts, new Map(acts.map((one) => [one.id, one])), new Set(), known).map((thread) => [
      thread.act.id,
      thread.messages.map((one) => one.id),
    ]);
  // In one session, prompt2 starts a thread of its own.
  expect(list([prompt("prompt1", 1), prompt("prompt2", 2)])).toEqual([
    ["prompt1", []],
    ["prompt2", []],
  ]);
  // In another session, or on another branch, prompt2 is a message to prompt1, and it stands in that thread at once.
  expect(list([prompt("prompt1", 1), prompt("prompt2", 2, { thread: "prompt1" })])).toEqual([
    ["prompt1", ["prompt2"]],
  ]);
});

test("a question and a notice that a thread made for the operator stand in that thread, with the questions that wait, and one that no thread made stands in none", () => {
  const acts = [
    prompt("prompt1", 1),
    act("rung1", "rung", "chain1", "prompt1", 2),
    act("prompt2", "prompt", "chain1", "rung1", 3, undefined, { words: ["bool", "Keep it?", "operator"] }),
    act("prompt3", "prompt", "chain1", "rung1", 4, undefined, {
      words: ["None", "It is 14 lines.", "operator"],
    }),
    act("prompt4", "prompt", "chain1", "chain1", 5, undefined, {
      words: ["str", "A question of no thread.", "operator"],
    }),
  ];
  const [thread, ...others] = chainThreads(
    acts,
    new Map(acts.map((one) => [one.id, one])),
    new Set(["prompt2", "prompt4"]),
  );
  expect(others).toEqual([]);
  expect(thread?.asked.map((one) => one.id)).toEqual(["prompt2", "prompt3"]);
  expect(thread?.asks).toEqual(["prompt2"]);
});

test("the order in which the acts land changes no verdict: each read with what a host kept gives what a fresh read gives", () => {
  const all = [
    act("chain1", "chain", "", "", 0),
    prompt("prompt1", 1, { words: ["str", "Hire two workers.", "claude"] }),
    prompt("prompt2", 2, { thread: "prompt1" }),
    prompt("prompt3", 3, { thread: "prompt2" }),
    act("rung1", "rung", "chain1", "prompt1", 4),
    act("bash5", "bash", "chain1", "rung1", 5),
    prompt("prompt9", 6, { by: "chain1", words: ["None", "bash5 done", ""] }),
  ];
  const read = (
    landed: LiveAct[],
    known?: WeakMap<LiveAct, { message: string | undefined; title: string }>,
  ) => {
    const on = all.filter((one) => landed.includes(one));
    return chainThreads(on, new Map(landed.map((one) => [one.id, one])), new Set(), known).map((thread) => [
      thread.act.id,
      thread.messages.map((one) => one.id),
    ]);
  };
  const orders = (rest: LiveAct[]): LiveAct[][] =>
    rest.length
      ? rest.flatMap((one) => orders(rest.filter((x) => x !== one)).map((tail) => [one, ...tail]))
      : [[]];
  let checked = 0;
  for (const order of orders(all)) {
    const known = new WeakMap<LiveAct, { message: string | undefined; title: string }>();
    for (let n = 1; n <= order.length; n++) {
      const landed = order.slice(0, n);
      expect(read(landed, known)).toEqual(read(landed));
      checked++;
    }
  }
  expect(checked).toBe(5040 * 7);
  expect(read(all)).toEqual([["prompt1", ["prompt2", "prompt3", "prompt9"]]]);
});

test("the threads that the ladder of a thread made nest under it, and a question in an agent sets the state of each thread over it to asks", () => {
  const thread = (one: LiveAct, asks: string[] = []): ChainThread => ({
    act: one,
    title: one.id,
    messages: [],
    asked: [],
    asks,
    state: asks.length ? "asks" : "running",
    resolved: false,
    origin: "rung",
    yours: false,
  });
  const lead = prompt("prompt1", 1);
  const agents = Array.from({ length: 8 }, (_, n) =>
    act(`prompt${n + 2}`, "prompt", "chain1", "rung1", n + 3),
  );
  const deep = act("prompt20", "prompt", "chain1", "rung2", 20);
  const other = prompt("prompt30", 30);
  const steps = [act("rung1", "rung", "chain1", "prompt1", 2), act("rung2", "rung", "chain1", "prompt9", 19)];
  const rows = new Map([lead, ...steps, ...agents, deep, other].map((one) => [one.id, one]));
  const tree = nestThreads(
    [thread(lead), ...agents.map((one) => thread(one)), thread(deep, ["prompt21"]), thread(other)],
    rows,
  );
  expect(tree.map((one) => one.thread.act.id)).toEqual(["prompt1", "prompt30"]);
  expect(tree[0]?.agents.map((one) => one.thread.act.id)).toEqual(agents.map((one) => one.id));
  expect(tree[0]?.agents.at(-1)?.agents.map((one) => one.thread.act.id)).toEqual(["prompt20"]);
  expect([tree[0]?.state, tree[0]?.agents.at(-1)?.state, tree[1]?.state]).toEqual([
    "asks",
    "asks",
    "running",
  ]);
  expect(tree[0]?.thread.asks).toEqual([]);
  expect(tree[0]?.thread.state).toBe("running");
});

test("nesting changes no thread of its list, so a question that is answered leaves the parent in its own state on a cached list, and a failed thread of the operator whose agent asks keeps failed", () => {
  const thread = (one: LiveAct, asks: string[] = [], more: Partial<ChainThread> = {}): ChainThread => ({
    act: one,
    title: one.id,
    messages: [],
    asked: [],
    asks,
    state: asks.length ? "asks" : "running",
    resolved: false,
    origin: "rung",
    yours: false,
    ...more,
  });
  const lead = prompt("prompt1", 1);
  const agent = act("prompt2", "prompt", "chain1", "rung1", 3);
  const rows = new Map(
    [lead, act("rung1", "rung", "chain1", "prompt1", 2), agent].map((one) => [one.id, one]),
  );
  // The thread of the lead is one object that a cache keeps from list to list.
  const parent = thread(lead);
  const asking = nestThreads([parent, thread(agent, ["prompt9"])], rows);
  expect([asking[0]?.state, asking[0]?.agents[0]?.state, parent.state]).toEqual(["asks", "asks", "running"]);
  const answered = nestThreads([parent, thread(agent)], rows);
  expect([answered[0]?.state, answered[0]?.agents[0]?.state, parent.state]).toEqual([
    "running",
    "running",
    "running",
  ]);
  // A failed thread of the operator whose agent asks counts as asks in its node and stays failed in its own record.
  const failed = thread(lead, [], { state: "failed", yours: true });
  const node = nestThreads([failed, thread(agent, ["prompt9"])], rows);
  expect([node[0]?.state, failed.state, node[0] && concerns(node[0])]).toEqual(["asks", "failed", "asks"]);
  expect(attentionOf(node)).toEqual({ asks: 1, failed: 0, held: 0 });
  expect(attentionOf(nestThreads([failed, thread(agent)], rows))).toEqual({ asks: 0, failed: 1, held: 0 });
});

test("a question waits for the operator in any thread, and a failure or a pause concerns them only in a thread that they started", () => {
  const error = { is: "Refused", args: ["no"] };
  const threads = (acts: LiveAct[], owed: string[] = []) =>
    chainThreads(acts, new Map(acts.map((one) => [one.id, one])), new Set(owed));
  const word = act("rung1", "rung", "chain1", "operator", 1);
  const agent = act("rung2", "rung", "chain1", "prompt1", 3);
  const made = (id: string, by: string, began: number, more: Partial<LiveAct> = {}) =>
    act(id, "prompt", "chain1", by, began, undefined, { words: ["str", `${id} says`, ""], ...more });
  const base = [prompt("prompt1", 1), word, agent];
  const read = (acts: LiveAct[], owed: string[] = []) =>
    threads([...base, ...acts], owed).map((one) => [
      one.act.id,
      one.origin,
      one.yours,
      concerns({ thread: one, state: one.state }) !== undefined,
    ]);
  // A question that waits in a thread of an agent concerns the operator.
  const step = act("rung3", "rung", "chain1", "prompt2", 5);
  const ask = act("prompt9", "prompt", "chain1", "rung3", 9, undefined, {
    words: ["bool", "Keep it?", "operator"],
  });
  expect(read([made("prompt2", "rung2", 4), step, ask], ["prompt9"])).toContainEqual([
    "prompt2",
    "rung",
    false,
    true,
  ]);
  // A failed agent thread does not concern them, and a thread of their word does when it fails or is held.
  expect(read([made("prompt2", "rung2", 4, { done: true, ended: 5, value: error })])).toContainEqual([
    "prompt2",
    "rung",
    false,
    false,
  ]);
  expect(read([made("prompt2", "rung1", 4, { done: true, ended: 5, value: error })])).toContainEqual([
    "prompt2",
    "rung",
    true,
    true,
  ]);
  expect(read([made("prompt2", "rung1", 4, { paused: true })])).toContainEqual([
    "prompt2",
    "rung",
    true,
    true,
  ]);
  expect(read([made("prompt2", "rung2", 4, { paused: true })])).toContainEqual([
    "prompt2",
    "rung",
    false,
    false,
  ]);
  // The operator's own message is theirs, and a thread that finished is no concern.
  expect(read([made("prompt2", "operator", 4)])).toContainEqual(["prompt2", "operator", true, false]);
  expect(read([made("prompt2", "ear7", 4, { done: true, ended: 5, value: error })])).toContainEqual([
    "prompt2",
    "ear",
    false,
    false,
  ]);
});
