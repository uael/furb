import type { Furb } from "../../tui/src/parts.ts";

export default function compaction(furb: Furb): void {
  furb.command("compact", "Keep the needed context and continue this chain", (summary, seen) =>
    seen.run(`compact(summary=${JSON.stringify(summary)})`),
  );
}
