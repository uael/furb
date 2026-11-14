// A soak: a life of thousands of facts, whose workers run commands while its host is killed at random points, opens
// again after each kill, ends each job with its right value, and replays at the end to the same turns and values.
import { expect } from "bun:test";
import { copyFileSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { alive, scenario, test, until, Window } from "./harness.ts";

/** The temporary folder of the machine, which outlives the yard of the run, where a red run keeps its record. */
const KEPT = process.env.FURB_MACHINE_TMPDIR ?? tmpdir();

/** How many commands each worker runs, and how many workers the root hires. */
const ROUNDS = 120;
const WORKERS = 3;

/** The word of a worker: it sums the numbers that its commands echo, and starts again from zero when a kill cut it. */
const JOB = `total = 0\nfor i in range(${ROUNDS}):\n  total += int((await bash(f"echo {i}")).stdout.content)\nclose(total)`;

/** A random number from a seed, so a run that fails says the seed that plays it again. */
function random(seed: number): () => number {
  let state = seed >>> 0 || 1;
  return () => {
    state ^= state << 13;
    state ^= state >>> 17;
    state ^= state << 5;
    return (state >>> 0) / 2 ** 32;
  };
}

/** The facts of each chain of a life, in all. */
async function size(window: Window): Promise<number> {
  let all = 0;
  for (let n = 1; await window.engine.get(`chain${n}`); n++)
    all += (await window.engine.transcript({ on: `chain${n}` })).length;
  return all;
}

/** What each job came to, or null while it runs. */
async function jobs(window: Window): Promise<unknown[]> {
  const found: unknown[] = [];
  for (let n = 2; await window.engine.get(`chain${n}`); n++) {
    const asked = (await window.engine.transcript({ on: `chain${n}`, of: "prompt" })).filter(
      (fact) => fact[5] === "Scenario: job." && fact[3] === `chain${n}`,
    );
    for (const one of asked) found.push(await window.engine.peek(one[1]));
  }
  return found;
}

test(
  "a life of thousands of facts with workers, stopped at random points, goes on to the right values and replays",
  () =>
    scenario(
      [
        // The root tells the operator that it hired its workers, and rests on their jobs: a close of its prompt waits
        // for each job, so it closes once the last job ends.
        {
          cue: "Scenario: soak.",
          word: `workers = [chain(f"w{i}") for i in range(${WORKERS})]\njobs = [prompt(int, "Scenario: job.", on=w) for w in workers]\nprompt(None, "Hired.", to=OPERATOR, on="prompt1")`,
        },
        { cue: " done\nprompt", word: 'if all(over(j) for j in jobs):\n  close("All done.")' },
        { cue: "Scenario: job.", word: JOB },
        // A word that a kill cut is refused as interrupted, and the worker runs its job again.
        { cue: "raised", word: JOB },
        { cue: "interrupted", word: JOB },
      ],
      async (first, yard) => {
        const seed = Number(process.env.FURB_SOAK_SEED ?? Date.now() % 100000);
        const next = random(seed);
        console.log(`The soak plays the seed ${seed}: FURB_SOAK_SEED=${seed} plays it again.`);
        // A red run keeps its record beside its seed, so it plays again exactly.
        try {
          let window = first;
          await window.type("Scenario: soak.");
          await window.shows((frame) => frame.includes("Hired."), "the root to hire its workers");
          for (let kill = 0; kill < 3; kill++) {
            await Bun.sleep(500 + next() * 3000);
            const pid = yard.host() ?? 0;
            expect(pid).toBeGreaterThan(0);
            await window.close(false);
            process.kill(pid, "SIGKILL");
            await until(() => !alive(pid), "the host to die");
            window = await Window.open(yard);
          }
          const sum = (ROUNDS * (ROUNDS - 1)) / 2;
          await until(
            async () => {
              const got = await jobs(window);
              return got.length === WORKERS && got.every((one) => one !== null);
            },
            "each job to end",
            180,
          );
          expect(await jobs(window)).toEqual(Array(WORKERS).fill(sum));
          // The kills come at random, so a run seldom ends a life at rest; this end writes the image that the next
          // life opens from, which each run then proves.
          await window.close(true);
          await yard.ended();
          window = await Window.open(yard);
          await until(
            async () => (await jobs(window)).length >= WORKERS,
            "the born life to hold each job",
            60,
          );
          expect(await jobs(window)).toEqual(Array(WORKERS).fill(sum));
          const facts = await size(window);
          expect(facts).toBeGreaterThan(2000);
          const cues = yard.turns().map((one) => one.cue);
          const born = readFileSync(yard.record, "utf8")
            .split("\n")
            .filter((line) => line.startsWith('[["checkpoint"')).length;
          console.log(
            `The soak said ${facts} facts, its model took ${cues.length} turns: ${JSON.stringify(cues)}, and ${born} of its lives were born on a checkpoint.`,
          );
          const turns = async (one: Window) => {
            const all: unknown[] = [];
            for (let n = 1; await one.engine.get(`chain${n}`); n++)
              all.push(await one.engine.turns({ on: `chain${n}` }));
            return JSON.stringify(all);
          };
          const before = await turns(window);
          await window.close(true);
          await yard.ended();
          // The end wrote an image, which a next life opens from with no replay: the replay reads the record with no
          // image and no checkpoint.
          await yard.alone();
          const replayed = await Window.open(yard);
          try {
            expect(await turns(replayed)).toBe(before);
            expect(await jobs(replayed)).toEqual(Array(WORKERS).fill(sum));
          } finally {
            await replayed.close(true);
          }
        } catch (error) {
          const kept = join(KEPT, `furb-soak-${seed}.jsonl`);
          copyFileSync(yard.record, kept);
          throw new Error(`The soak of the seed ${seed} failed, and its record is ${kept}: ${error}`, {
            cause: error,
          });
        }
      },
    ),
  300000,
);
