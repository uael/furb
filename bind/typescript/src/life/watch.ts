/** One binding that a chain told: a name, its value as python shows it, the moment it was told, and the act whose name
 * the name starts with, as bash2 for bash2_stdout. */
export interface Binding {
  name: string;
  value: string;
  at: number;
  act?: string | null;
}
/** The names of one act in the watch, or the names of no act. */
export interface Watched {
  act?: string;
  items: { name: string; value: string; fresh: boolean }[];
}

/** The watch of a chain at a moment: the last value of each name that it told by then, the newest first, each under the
 * act whose name it starts with, and with no binding of the act itself. A name told in the last moments is fresh. */
export function watch(bindings: readonly Binding[], at: number, recent = 12): Watched[] {
  const last = new Map<string, Binding>();
  for (const binding of bindings) {
    if (binding.at > at) continue;
    last.delete(binding.name);
    last.set(binding.name, binding);
  }
  const groups: Watched[] = [];
  for (const binding of [...last.values()].reverse()) {
    const act = binding.act ?? undefined;
    if (act === binding.name) continue;
    let group = groups.find((one) => one.act === act);
    if (!group) {
      group = { ...(act ? { act } : {}), items: [] };
      groups.push(group);
    }
    group.items.push({ name: binding.name, value: binding.value, fresh: at - binding.at < recent });
  }
  return groups;
}
