import type { Furb, Line, Mark, Seen } from "../../tui/src/parts.ts";

/** The first word on the chain after the place of a compaction that reads the files again with reground(), as
 * compact() gives it, in place or in a new chain. A clear gives none. */
function regrounding(seen: Seen, chain: string, at: number): string | undefined {
  return seen.acts
    .filter((act) => act.kind === "rung" && act.on === chain && act.began > at)
    .map((act) => String(act.words[0] ?? ""))
    .find((one) => one.trimEnd().endsWith("reground()"));
}

/** Whether the command may go on: it takes no argument, and the life runs the extension, since a life that does not
 * has no condense() or clear(), and the gate would refuse the word. */
async function ready(name: string, argument: string, seen: Seen): Promise<boolean> {
  if (argument.trim()) seen.say(`/${name} takes no argument`);
  else if (!(await seen.extensions()).includes("compaction")) seen.say("This chain has no compaction");
  else return true;
  return false;
}

/** The part of the extension compaction: in the feed of a chain, a rule at the place of each compaction, with its
 * summary under it, which parts what the model of the chain no longer reads from what it reads now. A cut that
 * regrounds nothing is a clear, and its rule says Cleared. The command /compact compacts the chain on screen at once,
 * to what a model chooses, with its summary, and the command /clear, or /reset, cuts it to nothing. */
export default function compaction(furb: Furb): void {
  furb.mark((seen) => {
    const cuts = seen.compactions
      .filter((one) => one.chain === seen.chain)
      .sort((one, other) => one.at - other.at);
    return cuts.map(({ chain, at }, index): Mark => {
      const count = cuts.length > 1 ? ` ${index + 1} of ${cuts.length}` : "";
      const word = regrounding(seen, chain, at);
      const lines = (/^<s:(summary_*)>\n([\s\S]*?)<\/s:\1>$/m.exec(word ?? "")?.[2]?.trim() ?? "")
        .split("\n")
        .filter((line) => line.trim())
        .map((line): Line => [[line, "muted"]]);
      const what = word === undefined ? "Cleared" : "Compaction";
      return { chain, at, title: `${what}${count}: the model reads on from here`, lines };
    });
  });
  furb.command(
    "compact",
    "Compact this chain now: a model keeps what the work still needs and sums up the rest",
    async (argument, seen) => {
      if (!(await ready("compact", argument, seen))) return;
      // The footer shows a short notice whole; the rule of the compaction says the rest once the model chose. The run
      // ends only once the model chose, so the notice comes first.
      seen.say("A model compacts this chain");
      await seen.run("await condense()");
    },
  );
  const clear =
    "Clear this chain: its model reads from zero, and the chain keeps its name, its prompts and its module";
  furb.command("clear", clear, async (argument, seen) => {
    if (await ready("clear", argument, seen)) await seen.run("clear()");
  });
  furb.command("reset", `Same as /clear: ${clear}`, async (argument, seen) => {
    if (await ready("reset", argument, seen)) await seen.run("clear()");
  });
}
