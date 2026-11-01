import type { Furb, Item, Line, Seen, Tone } from "../../src/parts.ts";

/** A run of text in a tone. */
const run = (text: string, tone: Tone) => [text, tone] as const;

/** The items of the list, as their last act todo says each: the item, and whether it is done. */
const items = (seen: Seen) => [
  ...new Map(
    seen
      .of("todo")
      .map((act) => [String(act.words[0]), { item: String(act.words[0]), done: Boolean(act.words[1]) }]),
  ).values(),
];

/** The word that marks an item done. */
const finish = (item: string) => `todo(${JSON.stringify(item)}, done=True)`;

/** The part of the extension todo, which uses each component that a part has: a panel that lists the items, the row
 * of each act todo in the feed, a status in the footer, commands, one for each open item that the life gives, the
 * open items after `#` in the input, dialogs of a list, a form and a confirm, and a key that adds an item. It imports
 * types alone, so it loads in any project. */
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
  furb.status((seen) => {
    const open = items(seen).filter((one) => !one.done).length;
    return open ? [run(`Todo: ${open} open`, "accent")] : "";
  });
  furb.command("todos", "Say how many items are open", (_, seen) => {
    const all = items(seen);
    seen.say(`${all.filter((one) => !one.done).length} of ${all.length} items are open`);
  });
  // A command for each open item, which goes when the item is done.
  furb.commands((seen) =>
    items(seen)
      .filter((one) => !one.done)
      .map((one, index) => ({
        name: `finish-${index + 1}`,
        detail: `Mark "${one.item}" done`,
        run: (_: string, later: Seen) => later.run(finish(one.item)),
      })),
  );
  furb.complete("#", (text, seen) =>
    items(seen)
      .filter((one) => !one.done && one.item.toLowerCase().includes(text.toLowerCase()))
      .map((one) => ({ label: `#${one.item}`, detail: "an open item", text: one.item })),
  );
  furb.command("pick", "Choose an item: Enter puts it in the input, x marks it done", async (_, seen) => {
    const chosen = await seen.choose(
      "Todo items",
      items(seen).map(
        ({ item, done }): Item<string> => ({
          label: item,
          detail: done ? "done" : "open",
          tone: done ? "faint" : "text",
          lines: [[run(`The item ${item} is ${done ? "done" : "open"}.`, "muted")]],
          value: item,
        }),
      ),
      { actions: [{ key: "x", label: "marks it done" }] },
    );
    if (!chosen) seen.say("No item was chosen.");
    else if (chosen.action === "x") await seen.run(finish(chosen.value));
    else seen.draft(`Do this next: ${chosen.value}`);
  });
  furb.command("add", "Add an item with a form, and say what the word gave", async (_, seen) => {
    const got = await seen.form("New item", [
      { name: "item", label: "Item", detail: "What to do", required: true },
      { name: "token", label: "Token", detail: "A secret that the dialog hides", secret: true },
    ]);
    if (!got) return seen.say("No item was added.");
    const added = await seen.run(`todo(${JSON.stringify(got.item)})`);
    seen.say(`Added ${String(added)}${got.token ? `, with a secret of ${got.token.length} signs` : ""}.`);
  });
  furb.command("clear", "Mark every item done, once you confirm it", async (_, seen) => {
    const open = items(seen).filter((one) => !one.done);
    if (
      !(await seen.confirm("Clear the list", `Mark ${open.length} items done?`, { yes: "Clear", no: "Keep" }))
    )
      return seen.say("The list stays as it is.");
    await seen.run(open.map((one) => finish(one.item)).join("\n"));
  });
  furb.command("ask", "Ask the model which item comes first", (_, seen) =>
    seen.send(
      `Which of these comes first? ${items(seen)
        .map((one) => one.item)
        .join(", ")}`,
    ),
  );
  furb.command("docs", "Open the guide of the parts in the browser", (_, seen) =>
    seen.open("https://github.com/uael/furb/blob/main/docs/extensions.md#a-part-for-the-tui"),
  );
  furb.key("+", (seen) => seen.run('todo("one more")'));
}
