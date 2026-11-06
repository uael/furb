/** A map as JavaScript holds it again: a map the life marked as its pairs, since it holds the key `is`, whose keys
 * are all strings becomes that map, and its entries are read the same way. */
export function unmarked(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(unmarked);
  if (!value || typeof value !== "object") return value;
  const held = value as Record<string, unknown>;
  if (held.is === "float" && Array.isArray(held.args))
    return Number(String(held.args[0]).replace(/^(-?)inf$/, "$1Infinity"));
  const pairs = held.is === "dict" && Array.isArray(held.args) ? held.args[0] : undefined;
  if (Array.isArray(pairs) && pairs.every((pair) => Array.isArray(pair) && typeof pair[0] === "string"))
    return Object.fromEntries(pairs.map(([key, one]) => [key, unmarked(one)]));
  return Object.fromEntries(Object.entries(held).map(([key, one]) => [key, unmarked(one)]));
}
