// A life born on a checkpoint goes on whole. The root of a first life binds a value, a definition that reads the
// value, a definition that reads a value that no checkpoint carries, and a chain, and the window shows its thread. The
// host ends the life with a checkpoint, a second life opens on the record and ends, and a third opens on it. Each born
// life holds each value and each life word of the root, a chain that it makes binds the setup of the standing, and a
// reply on the thread of the first life goes on with that thread.
import { expect } from "bun:test";
import { scenario, test, until, Window } from "./harness.ts";

/** The kinds of the acts that the life words of the extensions of every chain make. */
const LIFE = ["catchup", "compacted", "fitted", "remember"];

/** The word of the first message. desk reads a value that is no constant, so a birth binds it only once the values
 * stand, and held reads a value that no checkpoint carries, so a birth can never bind it. */
const WORK = [
  "EVERY = 300.0",
  "limit = 7",
  "handle = range(3)",
  "def desk(every: float = limit):",
  "  return every + EVERY",
  "def held(one=handle):",
  "  return one",
  'early = chain("early")',
  'close(f"desk={desk()}")',
].join("\n");

/** The word of the reply on the old thread, in the third life. */
const AGAIN = ['later = chain("later")', "seen = [limit, desk(), desk(1.0), EVERY]", 'close("again")'].join(
  "\n",
);

/** The kinds of the life words that a chain holds. */
async function kinds(window: Window, on: string): Promise<string[]> {
  const facts = (await window.engine.transcript({ on })) as unknown[][];
  return [...new Set(facts.map((fact) => String(fact[0])).filter((kind) => LIFE.includes(kind)))].sort();
}

/** The value of a name of the module of a chain, or nothing when the module holds no such name. */
async function value(window: Window, name: string, on: string): Promise<unknown> {
  const names = (await window.engine.names(on)) as string[];
  return names.includes(name) ? (await window.engine.inspect(name, on)).value : undefined;
}

/** Each life word of a chain stands, or a failure that names the life words that it holds. */
async function lives(window: Window, on: string, what: string): Promise<void> {
  await until(async () => (await kinds(window, on)).length === LIFE.length, what, 30).catch(
    async (error: Error) => {
      throw new Error(`${error.message} ${on} holds ${JSON.stringify(await kinds(window, on))}.`);
    },
  );
}

/** Each value and each life word of the root of a born life stands. */
async function whole(window: Window, root: string, life: string): Promise<void> {
  await lives(window, root, `the life words of the root of the ${life} life`);
  expect(await value(window, "limit", root)).toEqual(7);
  expect(await value(window, "EVERY", root)).toEqual(300);
}

test(
  "a life born on a checkpoint keeps each value and life word, sets up a new chain, and goes on with its thread",
  () =>
    scenario(
      [
        { cue: "Scenario: work.", word: WORK },
        { cue: "Scenario: again.", word: AGAIN },
      ],
      async (first, yard) => {
        await first.type("Scenario: work.");
        await first.shows((frame) => frame.includes("desk=307.0"), "the first answer");
        const root = first.root;
        await lives(
          first,
          String(await value(first, "early", root)),
          "the life words of a chain of the first life",
        );
        expect(await kinds(first, root)).toEqual(LIFE);
        const facts = (await first.engine.transcript({ on: root })) as unknown[][];
        const thread = String(facts.filter((fact) => fact[0] === "prompt").at(-1)?.[1]);
        expect(thread).toMatch(/^prompt\d+$/);
        await first.close(true);
        const second = await Window.open(yard);
        try {
          await whole(second, root, "second");
        } finally {
          await second.close(true);
        }
        const third = await Window.open(yard);
        try {
          await whole(third, root, "third");
          expect(await third.engine.get(thread)).toBeTruthy();
          await third.type("Scenario: again.");
          await until(
            async () => (await value(third, "seen", root)) !== undefined,
            "the word on the old thread",
          ).catch(async (error: Error) => {
            throw new Error(`${error.message} The last frame:\n${await third.frame()}`);
          });
          expect(await value(third, "seen", root)).toEqual([7, 307, 301, 300]);
          const again = (await third.engine.transcript({ on: root })) as unknown[][];
          expect(
            again.some(
              (fact) => fact[0] === "prompt" && fact[3] === thread && fact[5] === "Scenario: again.",
            ),
          ).toBe(true);
          await lives(
            third,
            String(await value(third, "later", root)),
            "the life words of a chain made after a birth",
          );
        } finally {
          await third.close(true);
        }
      },
    ),
  300000,
);
