import { createHash } from "node:crypto";
import type { Entry, Fact } from "furb";
import { operated } from "furb/life";

/** The hash of the words of a prompt, which the begin fact of a follow-up holds. */
export function queueHash(chain: string, shape: string, message: string, to: string): string {
  return createHash("sha256")
    .update(JSON.stringify([chain, shape, message, to]))
    .digest("hex");
}
function queueEvent(fact: Fact): { step: string; key: string; value: unknown } | undefined {
  if (!operated(fact[2])) return undefined;
  if (fact[0] === "queue") return { step: String(fact[3]), key: String(fact[4]), value: fact[5] };
  return undefined;
}

/** Begin and prompt are synchronous calls in one worker message. A following sent fact confirms the ID. */
export function queueDispatches(entries: readonly Entry[]): Map<string, string> {
  const sent = new Map<string, string>();
  dispatchesFrom(
    entries.map(([fact]) => fact),
    sent,
  );
  return sent;
}

/** The dispatches of the facts from an index on, added to those that sent holds, and the index at which the next
 * read starts. A view of a long life reads each fact once in this way, since a read of each fact at each change costs
 * more as the life grows. A begin that is the last fact waits for the fact that follows it, so the next read starts
 * at it again. */
export function dispatchesFrom(facts: readonly Fact[], sent: Map<string, string>, from = 0): number {
  for (let index = from; index < facts.length; index++) {
    const event = queueEvent(facts[index] as Fact);
    if (event?.step === "sent" && typeof event.value === "string") sent.set(event.key, event.value);
    if (event?.step === "begin") {
      if (index + 1 === facts.length) return index;
      const next = facts[index + 1];
      if (
        next?.[0] === "prompt" &&
        operated(next[2]) &&
        event.value === queueHash(String(next[3]), String(next[4]), String(next[5]), String(next[6]))
      )
        sent.set(event.key, next[1]);
    }
  }
  return facts.length;
}
