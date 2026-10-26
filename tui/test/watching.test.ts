import { afterAll, expect, test } from "bun:test";
import { demoSession, removeDemoDirectories, tour } from "../src/demo.ts";
import {
  asker,
  branchMessages,
  branchStart,
  branchTakes,
  earlierSteps,
  plainActs,
  stepName,
  watchBar,
} from "../src/watching.ts";
import { composing } from "./composing.ts";

afterAll(removeDemoDirectories);

/** Each line of a frame that shows a word of furb that means nothing at the first minute. */
const jargonIn = (frame: string) =>
  frame
    .split("\n")
    .filter((line) => jargon.test(line))
    .map((line) => line.trim());

/** The words of furb that mean nothing at the first minute, which the watch never shows. */
const jargon = /\b(rung\d*|rungs|word|words|transcript|module|(read|write|bash|prompt|chain)\d+)\b/i;

test("the bar of the watch says what plays, what it costs, and how to move in time, in common words", () => {
  expect(watchBar("$0.00", false)).toEqual([
    "Watching a life",
    "0 model calls, $0.00",
    "A short Python program asks three models at once.",
    "Press [ to go back in time, then b to start a branch from there.",
  ]);
  expect(watchBar("$0.00", true)[3]).toBe("An earlier moment. b branches here, ] steps on, Esc comes back.");
  for (const line of [...watchBar("$0.00", false), ...watchBar("$0.00", true)])
    expect(line).not.toMatch(jargon);
});

test("the acts of a step, its name, and who asks a model read in common words", () => {
  expect([
    plainActs("read1", 1),
    plainActs("write1", 1),
    plainActs("bash2", 3),
    plainActs("prompt4", 2),
  ]).toEqual(["read a file", "changed a file", "ran 3 commands", "asked 2 models"]);
  expect([stepName("you"), stepName("haiku")]).toEqual(["Your program", "haiku writes Python"]);
  expect([asker("haiku", false), asker("sonnet", true)]).toEqual([
    "the program asks haiku",
    "furb tells sonnet",
  ]);
  expect([earlierSteps(1), earlierSteps(6)]).toEqual(["1 earlier step", "6 earlier steps"]);
});

test("a branch says where it starts, with the files as they were, and the messages that it takes", () => {
  expect(branchStart("Main")).toBe("This branch starts from a moment of Main, with the files as they were.");
  expect([branchTakes(0), branchTakes(1), branchTakes(2)]).toEqual([
    "It takes no message from before.",
    "It takes the message before it, with its answer.",
    "It takes the 2 messages before it, with their answers.",
  ]);
  const acts = [
    { id: "prompt1", on: "chain1", began: 1 },
    { id: "prompt2", on: "chain1", began: 2 },
    { id: "prompt3", on: "chain1", began: 5 },
    { id: "prompt4", on: "chain2", began: 3 },
  ];
  // The word that made the branch names prompt2, which the branch leaves out, and prompt3 came after it.
  const word = 'chain("Another way", "chain1", take("prompt2", inside=False))';
  expect(branchMessages(acts, "chain1", { began: 4 }, word, () => true)).toBe(1);
});

test("the watch keeps its bar over the feed at 80x24, and no act runs at an earlier moment", async () => {
  const session = await demoSession({ watch: true });
  await composing(
    async ({ frame }) => {
      await tour(session, 0);
      await session.select(session.engine.root);
      const asks = () => session.at(session.activity).some((act) => act.kind === "prompt" && act.words[2]);
      while (asks()) session.travel(-1);
      const shown = await frame();
      const rows = shown.split("\n");
      // The bar stands at the top, under the header, whatever the feed shows.
      expect(rows.slice(0, 6).join("\n")).toContain("Watching a life   0 model calls, $0.00");
      expect(shown).toContain("An earlier moment. b branches here, ] steps on, Esc comes back.");
      expect(shown).not.toContain("<$0.01");
      expect(shown).not.toContain("running");
      expect(jargonIn(shown)).toEqual([]);
    },
    { width: 80, height: 24 },
    session,
  );
});

test("the branch of the watch says what it takes, and the views are named in common words", async () => {
  const session = await demoSession({ watch: true });
  await composing(
    async ({ frame }) => {
      await tour(session, 0);
      const branch = await frame();
      expect(branch).toContain("This branch starts from a moment of Main, with the files as they were.");
      expect(branch).toContain("It takes the message before it, with its answer.");
      expect(branch).toContain("What models read");
      expect(jargonIn(branch)).toEqual([]);
      await session.select(session.engine.root);
      const main = await frame();
      expect(main).toContain("You ran");
      expect(jargonIn(main)).toEqual([]);
    },
    { width: 152, height: 60 },
    session,
  );
});
