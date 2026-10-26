import type { LiveAct } from "./acts.js";

/** One logical chain, from its first chain through each recognized continuation to its current head. */
export interface ChainRun {
  head: string;
  members: readonly string[];
}

/** Logical chains from the cut metadata of Activity. A source alone is a branch, not a continuation.
 * Ambiguous continuations stay separate, so no arbitrary branch becomes a destination for work. */
export function chainRuns(chains: readonly LiveAct[]): Map<string, ChainRun> {
  const ids = new Set(chains.map((chain) => chain.id));
  const next = new Map<string, string[]>();
  for (const chain of chains) {
    const source = chain.compactions?.find((cut) => cut.source)?.source;
    if (!source || !ids.has(source) || source === chain.id) continue;
    next.set(source, [...(next.get(source) ?? []), chain.id]);
  }
  const parent = new Map<string, string>();
  for (const [source, children] of next)
    if (children.length === 1 && children[0]) parent.set(children[0], source);
  const runs = new Map<string, ChainRun>();
  for (const chain of chains) {
    let first = chain.id;
    const seen = new Set<string>();
    while (parent.has(first) && !seen.has(first)) {
      seen.add(first);
      first = parent.get(first) ?? first;
    }
    if (seen.has(first)) {
      runs.set(chain.id, { head: chain.id, members: [chain.id] });
      continue;
    }
    const members = [first];
    let head = first;
    while (next.get(head)?.length === 1) {
      const child = next.get(head)?.[0];
      if (!child || parent.get(child) !== head || members.includes(child)) break;
      members.push(child);
      head = child;
    }
    const run = { head, members };
    for (const id of members) runs.set(id, run);
  }
  return runs;
}
