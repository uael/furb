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
 * compactions into new chains shows as one feed, and its compactions count as one series. */
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
}
