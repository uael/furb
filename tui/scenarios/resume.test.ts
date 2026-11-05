// A life goes on after its window quits and after its host dies: a window that opens it again reads the same
// transcript, the same values and the same module, from its checkpoint and from its first entry with no checkpoint.
import { expect } from "bun:test";
import { readFileSync } from "node:fs";
import { alive, scenario, test, until, Window } from "./harness.ts";

/** What a life holds that a resume must keep: the turns of each chain, which its model reads, what each act of its
 * first life came to, and each value of the module of each chain that a word bound. */
async function held(window: Window, acts?: string[]) {
  const { engine } = window;
  const chains: string[] = [];
  for (let n = 1; await engine.get(`chain${n}`); n++) chains.push(`chain${n}`);
  const named = acts ?? [
    ...new Set(
      (await Promise.all(chains.map((on) => engine.transcript({ on }))))
        .flat()
        .map((fact) => fact[1])
        .filter((id) => /^(prompt|bash|read|write|wait|rung)\d+$/.test(id)),
    ),
  ];
  const life = {
    turns: Object.fromEntries(await Promise.all(chains.map(async (on) => [on, await engine.turns({ on })]))),
    peeks: Object.fromEntries(await Promise.all(named.map(async (id) => [id, await engine.peek(id)]))),
    modules: Object.fromEntries(
      await Promise.all(
        chains.map(async (on) => {
          const module = ((await engine.module({ on })) ?? {}) as Record<string, unknown>;
          const words = Object.keys((await engine.program({ on })) ?? {});
          return [
            on,
            Object.fromEntries(Object.entries(module).filter(([key]) => WORDS.includes(key))),
            words.length,
          ];
        }),
      ),
    ),
  };
  return { life: JSON.parse(JSON.stringify(life)), acts: named };
}

/** The names that the words of the script bind. */
const WORDS = ["n", "said", "notes"];

/** The first place where two lives differ, short enough to read, or nothing when they are the same. */
function differ(one: unknown, other: unknown, at = ""): string | undefined {
  if (JSON.stringify(one) === JSON.stringify(other)) return undefined;
  if (one && other && typeof one === "object" && typeof other === "object") {
    const keys = [...new Set([...Object.keys(one), ...Object.keys(other)])];
    for (const key of keys) {
      const found = differ(
        (one as Record<string, unknown>)[key],
        (other as Record<string, unknown>)[key],
        `${at}.${key}`,
      );
      if (found) return found;
    }
  }
  return `${at}: ${JSON.stringify(one)?.slice(0, 600)}\n  became ${JSON.stringify(other)?.slice(0, 600)}`;
}

/** The life of a window holds what another held. */
async function same(window: Window, before: Awaited<ReturnType<typeof held>>): Promise<void> {
  const { life } = await held(window, before.acts);
  expect(differ(before.life, life)).toBeUndefined();
}

const SCRIPT = [
  {
    cue: "Scenario: work.",
    word: 'n = 41\nsaid = await bash("echo hi")\nwrite(Text("notes.md", "one\\ntwo\\n"))\nclose(f"n={n}")',
  },
  { cue: "Scenario: more.", word: 'n += 1\nnotes = read("notes.md").content\nclose(f"n={n}")' },
];

/** A life that worked and settled: two messages answered, a command, a write and a read. */
async function work(window: Window): Promise<void> {
  await window.type("Scenario: work.");
  await window.shows((frame) => frame.includes("n=41"), "the first answer");
  await window.type("Scenario: more.");
  await window.shows((frame) => frame.includes("n=42"), "the second answer");
}

test("a life that quits opens again, and after its end replays from its first entry to the same turns, values and module", () =>
  scenario(SCRIPT, async (first, yard) => {
    await work(first);
    const before = await held(first);
    // A quit detaches the window, and the life goes on in its host.
    const pid = yard.host() ?? 0;
    await first.close(false);
    expect(alive(pid)).toBe(true);
    const attached = await Window.open(yard);
    try {
      await same(attached, before);
    } finally {
      await attached.close(true);
    }
    await until(() => !yard.host(), "the host to end");
    // The record holds no checkpoint, so the next life replays each word from the first entry.
    const kinds = readFileSync(yard.record, "utf8")
      .split("\n")
      .filter(Boolean)
      .map((line) => (JSON.parse(line) as unknown[][])[0]?.[0]);
    expect(kinds).not.toContain("checkpoint");
    const replayed = await Window.open(yard);
    try {
      await same(replayed, before);
      await replayed.type("Scenario: more.");
      await replayed.shows((frame) => frame.includes("n=43"), "an answer after the resume");
    } finally {
      await replayed.close(true);
    }
    expect(yard.turns().length).toBe(3);
  }));

test("a life whose host is killed opens again to the same transcript, values and module, and goes on", () =>
  scenario(SCRIPT, async (first, yard) => {
    await work(first);
    const before = await held(first);
    const pid = yard.host() ?? 0;
    expect(pid).toBeGreaterThan(0);
    await first.close(false);
    process.kill(pid, "SIGKILL");
    await until(() => !alive(pid), "the host to die");
    const again = await Window.open(yard);
    try {
      await same(again, before);
      await again.type("Scenario: more.");
      await again.shows((frame) => frame.includes("n=43"), "an answer after the kill");
    } finally {
      await again.close(true);
    }
    expect(yard.turns().length).toBe(3);
  }));
