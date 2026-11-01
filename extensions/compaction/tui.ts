import type { Furb, Line, Mark, Seen } from "../../tui/src/parts.ts";

/** The summary that a compaction gives a chain: the quote summary of the first word on the chain after the place of
 * the compaction that reads the files again with reground(), as compact() gives it, in place or in a new chain. */
function summary(seen: Seen, chain: string, at: number): string {
  const word = seen.acts
    .filter((act) => act.kind === "rung" && act.on === chain && act.began > at)
    .map((act) => String(act.words[0] ?? ""))
    .find((one) => one.trimEnd().endsWith("reground()"));
  return /^<s:(summary_*)>\n([\s\S]*?)<\/s:\1>$/m.exec(word ?? "")?.[2]?.trim() ?? "";
}

/** The part of the extension compaction: in the feed of a chain, a rule at the place of each compaction, with its
 * summary under it, which parts what the model of the chain no longer reads from what it reads now. A run of
 * compactions into new chains shows as one feed, and its compactions count as one series. The command /compact
 * compacts the chain on screen at once, to what a model chooses, with its summary. */
export default function compaction(furb: Furb): void {
  furb.mark((seen) => {
    const run = furb.life.compactedRun(seen.chain, seen.compactions);
    const cuts = seen.compactions
      .filter((one) => run.includes(one.chain))
      .sort((one, other) => one.at - other.at);
    return cuts.map(({ chain, at, from }, index): Mark => {
      const count = cuts.length > 1 ? ` ${index + 1} of ${cuts.length}` : "";
      const lines = summary(seen, chain, at)
        .split("\n")
        .filter((line) => line.trim())
        .map((line): Line => [[line, "muted"]]);
      const where = from ? "goes on in a new chain" : "reads on";
      return { chain, at, title: `Compaction${count}: the model ${where} from here`, lines };
    });
  });
  furb.command(
    "compact",
    "Compact this chain now: a model keeps what the work still needs and sums up the rest",
    async (argument, seen) => {
      if (argument.trim()) return seen.say("/compact takes no argument");
      // A life that runs no word of the extension on this chain has no condense(), and the gate would refuse the word.
      const runs = seen.acts.some(
        (act) =>
          act.kind === "rung" &&
          act.on === seen.chain &&
          String(act.words[0] ?? "").includes("def condense("),
      );
      if (!runs) return seen.say("This chain has no compaction");
      // The footer shows a short notice whole; the rule of the compaction says the rest once the model chose. The run
      // ends only once the model chose, so the notice comes first.
      seen.say("A model compacts this chain");
      await seen.run("await condense()");
    },
  );
}
