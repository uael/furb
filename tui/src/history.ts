/** A bounded, recent-first view of history that matches the text before the cursor. */
export function matchingHistory(history: readonly string[], prefix: string, limit = 200): string[] {
  if (limit <= 0) return [];
  const wanted = prefix.toLowerCase();
  const seen = new Set<string>();
  const found: string[] = [];
  for (const text of [...history].reverse()) {
    if (!text.trim() || seen.has(text) || !text.toLocaleLowerCase().startsWith(wanted)) continue;
    seen.add(text);
    found.push(text);
    if (found.length >= limit) break;
  }
  return found;
}
