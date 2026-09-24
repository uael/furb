import { expect, test } from "bun:test";
import type { TuiContext } from "@furb/engine";
import skills from "../tui.ts";

/** A context that keeps what the part asks of it, and answers a skills question with one skill. */
function context(): { context: TuiContext; said: unknown[][] } {
  const said: unknown[][] = [];
  const held = {
    chain: "chain2",
    call: async (verb: string, args: unknown[] = []) => {
      said.push(["call", verb, ...args]);
      return "rung9";
    },
    life: {
      call: async (verb: string, args: unknown[]) => {
        said.push(["life", verb, ...args]);
        return [null, [{ name: "brew", description: "Make tea.", path: "/p/SKILL.md" }]];
      },
    },
    track: (id: string) => said.push(["track", id]),
    show: (view: string) => said.push(["show", view]),
    notify: (message: string) => said.push(["notify", message]),
  };
  return { context: held as unknown as TuiContext, said };
}

test("/skill reads a skill into the chain on screen by a rung of the operator", async () => {
  const { context: given, said } = context();
  await skills().commands?.skill?.run("brew", given);
  expect(said).toEqual([
    ["call", "rung", 'skill("brew")'],
    ["track", "rung9"],
    ["show", "feed"],
  ]);
});

test("/skills asks the skills of the chain on screen and says them", async () => {
  const { context: given, said } = context();
  await skills().commands?.skills?.run("", given);
  expect(said).toEqual([
    ["life", "ask", "skills", "chain2"],
    ["notify", "Skills: brew. Use /skill with a name."],
  ]);
});

test("/reload-skills tells the chain on screen what changed of its skills", async () => {
  const { context: given, said } = context();
  await skills().commands?.["reload-skills"]?.run("", given);
  expect(said[0]).toEqual(["call", "rung", "skills()"]);
});
