import type { ActRow } from "./session.ts";

/** The place of a chain on the map: its column, which is its depth in the tree of hires, and its row. */
export interface Place {
  column: number;
  row: number;
}
/** A message between two programs: a prompt from the chain of the rung that made it to the chain that it stands on,
 * which flows while it is open, or the answer that flows back once it closed. A question to the operator flows to
 * the operator. */
export interface Message {
  act: string;
  from: string;
  to: string;
  kind: "ask" | "answer" | "asks you";
}

/** The operator, which stands on the map as a program of its own, at the top of the first column. */
export const you = "operator";

/** The places of the chains of a life, which no later chain moves: the operator at the top of the first column, and
 * each chain, in the order it was born, in the column of its depth, at the first free row at or under the row of the
 * chain that hired it. */
export function places(
  chains: readonly ActRow[],
  parent: (chain: ActRow) => string | undefined,
): Map<string, Place> {
  const out = new Map<string, Place>([[you, { column: 0, row: 0 }]]);
  const taken = new Map<number, Set<number>>([[0, new Set([0])]]);
  for (const chain of [...chains].sort((one, other) => one.began - other.began)) {
    const above = out.get(parent(chain) ?? "");
    const column = above ? above.column + 1 : 0;
    const used = taken.get(column) ?? new Set<number>();
    taken.set(column, used);
    let row = above ? above.row : 0;
    while (used.has(row)) row++;
    used.add(row);
    out.set(chain.id, { column, row });
  }
  return out;
}

/** The messages of a life at a moment: each open prompt from one chain to another or to the operator, and each
 * answer that closed in the last moments, to the chain of the rung that made its prompt. */
export function messages(acts: readonly ActRow[], recent: (act: ActRow) => boolean): Message[] {
  const rows = new Map(acts.map((act) => [act.id, act]));
  const out: Message[] = [];
  for (const act of acts) {
    if (act.kind !== "prompt" || act.words[0] === "None") continue;
    const maker = rows.get(act.by);
    const from = maker?.on;
    if (act.words[2] === "operator" && !act.done)
      out.push({ act: act.id, from: act.on, to: you, kind: "asks you" });
    if (!from || from === act.on) continue;
    if (!act.done) out.push({ act: act.id, from, to: act.on, kind: "ask" });
    else if (recent(act)) out.push({ act: act.id, from: act.on, to: from, kind: "answer" });
  }
  return out;
}

/** The chains on the way from one chain to another along the tree of hires: up to the chain that both stand under,
 * then down. The operator stands over every chain at the top of the tree. */
export function route(from: string, to: string, parent: (id: string) => string | undefined): string[] {
  const up = (id: string): string[] => {
    const out = [id];
    for (let at = parent(id); at && out.length < 256; at = parent(at)) out.push(at);
    if (out.at(-1) !== you) out.push(you);
    return out;
  };
  const first = up(from),
    second = up(to);
  const meet = first.find((id) => second.includes(id)) ?? you;
  return [...first.slice(0, first.indexOf(meet) + 1), ...second.slice(0, second.indexOf(meet)).reverse()];
}
