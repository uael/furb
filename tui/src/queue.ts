import { createHash } from "node:crypto";
import type { Entry } from "@furb/engine";

export function queueHash(chain: string, shape: string, message: string, actor: string): string {
  return createHash("sha256")
    .update(JSON.stringify([chain, shape, message, actor]))
    .digest("hex");
}

/** Begin and prompt are synchronous calls in one worker message. A following sent fact confirms the ID. */
export function queueDispatches(entries: readonly Entry[]): Map<string, string> {
  const sent = new Map<string, string>();
  for (const [index, entry] of entries.entries()) {
    const fact = entry[1];
    if (fact[2] !== "operator") continue;
    if (fact[0] === "queue_sent" && typeof fact[3] === "string" && typeof fact[4] === "string")
      sent.set(fact[3], fact[4]);
    if (fact[0] === "queue_begin") {
      const next = entries[index + 1]?.[1];
      if (
        next?.[0] === "prompt" &&
        next[2] === "operator" &&
        fact[4] === queueHash(String(next[3]), String(next[4]), String(next[5]), String(next[6]))
      )
        sent.set(String(fact[3]), next[1]);
    }
  }
  return sent;
}
