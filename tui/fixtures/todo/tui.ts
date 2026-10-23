import type { Furb, Line, Seen, Tone } from "../../src/parts.ts";

/** A run of text in a tone. */
const run = (text: string, tone: Tone) => [text, tone] as const;

/** The items of the list, one for each act todo, as its words say it: the item, and whether it is done. */
const items = (seen: Seen) =>
  seen.of("todo").map((act) => ({ item: String(act.words[0]), done: Boolean(act.words[1]) }));

/** The part of the extension todo: a panel that lists the items, the row of each act todo in the feed, a command that
 * counts what is open, and a key that adds an item. It imports types alone, so it loads in any project. */
export default function todo(furb: Furb): void {
  furb.panel("Todo", (seen) => {
    const all = items(seen);
    const head: Line = [run(`${all.filter((one) => !one.done).length} open of ${all.length}`, "faint")];
    const rows = all.map(
      ({ item, done }): Line => [
        run(done ? "✓ " : "○ ", done ? "success" : "accent"),
        run(item, done ? "faint" : "text"),
      ],
    );
    return [head, ...rows];
  });
  furb.row("todo", (act) => ({
    label: `${act.words[1] ? "done" : "to do"}: ${String(act.words[0])}`,
    details: [[run("A todo of the list, which the panel Todo shows.", "muted")]],
  }));
  furb.command("todos", "Say how many items are open", (_, seen) => {
    const all = items(seen);
    seen.say(`${all.filter((one) => !one.done).length} of ${all.length} items are open`);
  });
  furb.key("+", (seen) => seen.run('todo("one more")'));
}
