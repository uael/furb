import type { Furb, Line, Mark, Seen } from "../../tui/src/parts.ts";

/** The summary that a compaction gives the chain: the quote summary of the word that compact() runs on the chain right
 * after the prefix, which reads the files again with reground(). */
function summary(seen: Seen, chain: string, at: number): string {
  const word = seen.acts
    .filter((act) => act.kind === "rung" && act.on === chain && act.began > at)
    .map((act) => String(act.words[0] ?? ""))
    .find((one) => one.trimEnd().endsWith("reground()"));
  return /^<s:(summary_*)>\n([\s\S]*?)<\/s:\1>$/m.exec(word ?? "")?.[2]?.trim() ?? "";
}

/** The part of the extension compaction: in the feed of a chain, a rule at the place of each compaction, with its
 * summary under it, which parts what the model of the chain no longer reads from what it reads now. */
export default function compaction(furb: Furb): void {
  furb.mark((seen) => {
    const cuts = seen.compactions.filter((one) => one.chain === seen.chain);
    return cuts.map(({ chain, at }, index): Mark => {
      const count = cuts.length > 1 ? ` ${index + 1} of ${cuts.length}` : "";
      const lines = summary(seen, chain, at)
        .split("\n")
        .filter((line) => line.trim())
        .map((line): Line => [[line, "muted"]]);
      return {
        chain,
        at,
        title: `Compaction${count}: the model reads on from here`,
        lines,
      };
    });
  });
}
