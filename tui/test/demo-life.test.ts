import { expect, test } from "bun:test";
import { scripts } from "../src/demo/index.ts";
import { seedLife } from "../src/demo/life.ts";
import { scripted } from "../src/demo/script.ts";
import { demoSession, removeDemoDirectories } from "../src/demo.ts";

const turn = (prompt: string, message: string, advances: number) => [
  {
    role: "user",
    content: [
      {
        type: "text",
        text:
          `#${prompt}\n${prompt}_message = '${message}'\n` + `#rung1 advance on ${prompt}\n`.repeat(advances),
      },
    ],
  },
];

test("a step of a script is found by the message of its prompt, and counts each advance on that prompt", () => {
  const message = "Time the search on 2000 notes, and say where the time goes.";
  expect(scripted(scripts, turn("prompt4", message, 1))?.step).toBe(0);
  expect(scripted(scripts, turn("prompt4", message, 3))?.step).toBe(2);
  const text = `prompt4_message = "List the keys that src/keys.ts answers, as 'key: what it does'."\n#rung1 advance on prompt4\n`;
  const quoted = [{ role: "user", content: [{ type: "text", text }] }];
  expect(scripted(scripts, quoted)?.script.cue).toBe("List the keys that src/keys.ts answers");
  expect(scripted(scripts, turn("prompt4", "Say hello.", 1))).toBeUndefined();
  expect(scripted(scripts, turn("prompt4", "Say hello.", 0))).toBeUndefined();
});

test("each script of the demo has a cue that no other script shares, and its last step closes", () => {
  const cues = scripts.map((script) => script.cue);
  expect(new Set(cues).size).toBe(cues.length);
  for (const script of scripts)
    expect(script.steps.at(-1)?.word).toMatch(/close\(|await wait\(|await bash\(/);
});

test("the life of the demo is as dense as a real one: chains in a tree, threads, control flow, and every state", async () => {
  const session = await demoSession();
  try {
    const standing = await seedLife(session);
    const labels = session.chains.map((chain) => String(chain.words[0]));
    expect(labels.length).toBeGreaterThanOrEqual(17);
    expect(labels).toEqual(
      expect.arrayContaining(["w/storage", "rivals/logseq", "logseq/blocks", "legacy/verify"]),
    );
    const prompts = session.acts.filter((act) => act.kind === "prompt");
    expect(prompts.length).toBeGreaterThanOrEqual(30);
    const words = session.acts.filter((act) => act.kind === "rung").map((act) => act.word ?? "");
    const kinds = ["while ", "def ", "for ", "if ", "try:", "lambda", "await "];
    const has = (word: string, kind: string) =>
      kind === "lambda" || kind === "await "
        ? word.includes(kind)
        : word.split("\n").some((line) => line.trimStart().startsWith(kind));
    const missing = kinds.filter((kind) => !words.some((word) => has(word, kind)));
    expect(missing).toEqual([]);
    expect(Math.max(...words.map((word) => word.split("\n").length))).toBeGreaterThanOrEqual(30);
    const asks = [...session.host.prompts.values()].map((ask) => ask.shape);
    expect(asks).toEqual(expect.arrayContaining(["bool", "int", "str"]));
    expect(asks.some((shape) => shape.startsWith("Literal"))).toBe(true);
    expect(session.acts.some((act) => act.run?.status === "failed")).toBe(true);
    expect(session.acts.some((act) => act.kind === "grant")).toBe(true);
    expect(session.acts.some((act) => act.kind === "bash" && !act.done)).toBe(true);
    expect(standing.size).toBeGreaterThan(0);
    for (const id of standing) expect(session.acts.find((act) => act.id === id)?.done).toBe(false);
  } finally {
    await session.dispose();
    await removeDemoDirectories();
  }
}, 30_000);
