import { type Act, isDone } from "./acts.js";

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

/** The messages of a life at a moment: each open prompt from one chain to another or to the operator, and each
 * answer that closed in the last moments, to the chain of the rung that made its prompt. */
export function messages(acts: readonly Act[], recent: (act: Act) => boolean): Message[] {
  const rows = new Map(acts.map((act) => [act.id, act]));
  const out: Message[] = [];
  for (const act of acts) {
    if (act.kind !== "prompt" || act.words[0] === "None") continue;
    const maker = rows.get(act.by);
    const from = maker?.chain;
    if (act.words[2] === "operator" && !isDone(act))
      out.push({ act: act.id, from: act.chain, to: you, kind: "asks you" });
    if (!from || from === act.chain) continue;
    if (!isDone(act)) out.push({ act: act.id, from, to: act.chain, kind: "ask" });
    else if (recent(act)) out.push({ act: act.id, from: act.chain, to: from, kind: "answer" });
  }
  return out;
}
