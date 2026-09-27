import { createHash } from "node:crypto";
import type { Entry, Fact } from "@furb/engine";

export function queueHash(chain: string, shape: string, message: string, actor: string): string {
  return createHash("sha256")
    .update(JSON.stringify([chain, shape, message, actor]))
    .digest("hex");
}
function queueEvent(fact: Fact): { step: string; key: string; value: unknown } | undefined {
  if (fact[2] !== "operator") return undefined;
  if (fact[0] === "queue") return { step: String(fact[3]), key: String(fact[4]), value: fact[5] };
  return undefined;
}

/** Begin and prompt are synchronous calls in one worker message. A following sent fact confirms the ID. */
export function queueDispatches(entries: readonly Entry[]): Map<string, string> {
  const sent = new Map<string, string>();
  for (const [index, entry] of entries.entries()) {
    const [fact] = entry;
    const event = queueEvent(fact);
    if (event?.step === "sent" && typeof event.value === "string") sent.set(event.key, event.value);
    if (event?.step === "begin") {
      const next = entries[index + 1]?.[0];
      if (
        next?.[0] === "prompt" &&
        next[2] === "operator" &&
        event.value === queueHash(String(next[3]), String(next[4]), String(next[5]), String(next[6]))
      )
        sent.set(event.key, next[1]);
    }
  }
  return sent;
}
