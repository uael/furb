import { expect, test } from "bun:test";
import { PassThrough } from "node:stream";
import { LIFE, linked, SKILL, Skill } from "../src/skill.ts";

/** A double of the life of a kit on two streams: it keeps each verb that the kit writes, answers it with `answer`, and
 * pushes frames to the kit. */
function life(answer: (verb: Record<string, unknown> & { id: number }) => object | undefined) {
  const said: (Record<string, unknown> & { id: number })[] = [];
  const toKit = new PassThrough();
  const fromKit = new PassThrough();
  let rest = "";
  fromKit.on("data", (chunk) => {
    rest += String(chunk);
    const lines = rest.split("\n");
    rest = lines.pop() ?? "";
    for (const line of lines.filter(Boolean)) {
      const verb = JSON.parse(line) as Record<string, unknown> & { id: number };
      said.push(verb);
      const frame = answer(verb);
      if (frame) toKit.write(`${JSON.stringify({ id: verb.id, ...frame })}\n`);
    }
  });
  const link = linked(toKit, fromKit);
  const push = (frame: object) => toKit.write(`${JSON.stringify(frame)}\n`);
  const fact = (seq: number, data: unknown[]) =>
    push({ frame: "event", life: "a.jsonl", seq, event: { event: "fact", data } });
  return { said, link, fact };
}

const welcome = { frame: "reply", answer: { answer: "welcome", data: { peer: {} } } };
const done = { frame: "reply", answer: { answer: "done" } };
const env = { [LIFE]: "a.jsonl", [SKILL]: "k" };

test("a skill answers each open act of its kind with a close, and skips one that is done", async () => {
  const double = life((verb) =>
    verb.verb === "hello"
      ? welcome
      : verb.verb === "peek" && verb.act !== "k.add1"
        ? { frame: "refused", error: { error: "refused" }, why: `No act named ${verb.act} is done.` }
        : done,
  );
  const skill = await Skill.attach({ kinds: ["prompt"], env, link: double.link });
  skill.answers("k.add", async (fact) => `added ${String(fact[4])}`);
  skill.answers("k.drop", () => {
    throw new Error("no room");
  });
  skill.answers("k.gone", () => 1)();
  const acts = [
    ["k.add", "k.add1"],
    ["k.add", "k.add2"],
    ["k.drop", "k.drop1"],
    ["k.gone", "x"],
  ];
  acts.forEach(([kind, act], seq) => {
    double.fact(seq, [kind, act, "rung4", "chain1", "milk"]);
  });
  const closes = () => double.said.filter((verb) => verb.verb === "close");
  for (let tries = 0; closes().length < 2 && tries < 500; tries++) await Bun.sleep(10);
  const got = closes()
    .map((verb) => [verb.act, verb.value])
    .sort();
  expect(got).toEqual([
    ["k.add2", JSON.stringify("added milk")],
    ["k.drop1", JSON.stringify({ is: "Refused", args: ["no room"] })],
  ]);
  const peeked = double.said.filter((verb) => verb.verb === "peek").map((verb) => verb.act);
  expect(peeked.sort()).toEqual(["k.add1", "k.add2", "k.drop1"]);
  skill.close();
});

test("the done of an act that a kit answers aborts its answer, which closes nothing", async () => {
  const double = life((verb) =>
    verb.verb === "hello"
      ? welcome
      : verb.verb === "peek"
        ? { frame: "refused", error: { error: "refused" }, why: "not done" }
        : done,
  );
  const skill = await Skill.attach({ env, link: double.link });
  let aborted = false;
  skill.answers(
    "k.slow",
    (_fact, signal) =>
      new Promise((resolve) => {
        signal.addEventListener("abort", () => {
          aborted = true;
          resolve("late");
        });
      }),
  );
  double.fact(0, ["k.slow", "k.slow1"]);
  for (let tries = 0; !double.said.some((verb) => verb.verb === "peek") && tries < 500; tries++)
    await Bun.sleep(10);
  double.fact(1, ["done", "k.slow1", "operator", "chain1", null]);
  for (let tries = 0; !aborted && tries < 500; tries++) await Bun.sleep(10);
  expect(aborted).toBe(true);
  await Bun.sleep(20);
  expect(double.said.filter((verb) => verb.verb === "close")).toEqual([]);
  skill.close();
});

test("a kit reads its own keys from a life that gives config, and an older life refuses", async () => {
  const settings = [
    { key: ["k", "folder"], value: "journal", skill: "k" },
    { key: ["other", "folder"], value: "x", skill: "other" },
    { key: ["automatic"], value: [] },
  ];
  const configured = {
    frame: "reply",
    answer: { answer: "welcome", data: { peer: {}, capabilities: ["config"] } },
  };
  const double = life((verb) =>
    verb.verb === "hello"
      ? configured
      : verb.verb === "config"
        ? { frame: "reply", answer: { answer: "values", data: settings } }
        : done,
  );
  const skill = await Skill.attach({ env, link: double.link });
  expect(await skill.settings()).toEqual({ folder: "journal" });
  expect(double.said.at(-1)).toEqual({ id: 2, verb: "config", config: { action: "get" }, life: "a.jsonl" });
  const older = await Skill.attach({
    env,
    link: life((verb) => (verb.verb === "hello" ? welcome : done)).link,
  });
  await expect(older.settings()).rejects.toThrow("older than the capability config");
});

test("a kit answers an act that waits for it as it attaches, with the answers that its welcome made", async () => {
  let fact = (_seq: number, _data: unknown[]) => {};
  const double = life((verb) => {
    if (verb.verb === "hello")
      return {
        frame: "reply",
        answer: { answer: "welcome", data: { peer: {}, folders: { project: "/p" } } },
      };
    if (verb.verb === "peek") return { frame: "refused", error: { error: "refused" }, why: "not done" };
    if (verb.verb === "attach") queueMicrotask(() => fact(0, ["k.add", "k.add1", "rung4", "chain1", "milk"]));
    return done;
  });
  fact = double.fact;
  const skill = await Skill.attach({
    env,
    link: double.link,
    answering: (welcomed) => ({
      "k.add": (one) => `added ${String(one[4])} in ${String(welcomed.folders?.project)}`,
    }),
  });
  const closes = () => double.said.filter((verb) => verb.verb === "close");
  for (let tries = 0; closes().length < 1 && tries < 500; tries++) await Bun.sleep(10);
  expect(closes().map((verb) => [verb.act, verb.value])).toEqual([
    ["k.add1", JSON.stringify("added milk in /p")],
  ]);
  skill.close();
});
