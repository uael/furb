import type { Script } from "./script.ts";

/** The part for the TUI that the model writes, as TypeScript. It imports no value, as the skill extending says. */
const part = String.raw`<s:part>
export default function (furb) {
  const items = (seen) =>
    seen.of("todo").map((act) => ({ item: String(act.words[0]), done: Boolean(act.words[1]) }));
  furb.panel("Todo", (seen) => {
    const all = items(seen);
    const open = all.filter((one) => !one.done).length;
    return [
      [[open + " open of " + all.length, "faint"]],
      ...all.map(({ item, done }) => [
        [done ? "✓ " : "○ ", done ? "success" : "accent"],
        [item, done ? "faint" : "text"],
      ]),
    ];
  });
  furb.row("todo", (act) => ({ label: (act.words[1] ? "done: " : "to do: ") + act.words[0] }));
}
</s:part>`;

const define = String.raw`def todo(item, *, done=False, on=""):
  def ear(name):
    yield "done", name, None
  return act("todo", on, ear, item, done)`;

const extend = `${part}
extend("todo", next(k for k, w in program().items() if w.startswith("def todo")), tui=part)`;

const items = String.raw`todo("Read the capture", done=True)
todo("Draw the todo list in a panel")
todo("Load the part again when its file changes")`;

const read = String.raw`part = read(".furb/extensions/todo/tui.ts", grep("items|panel"))`;

const sorted = String.raw`part = read(".furb/extensions/todo/tui.ts")
write(part.replace("const all = items(seen);", "const all = seen.todos.sort((one, other) => Number(one.done) - Number(other.done));"))`;

const mended = String.raw`part = read(".furb/extensions/todo/tui.ts")
write(part.replace("seen.todos.sort(", "items(seen).sort("))`;

export const extending: Script[] = [
  {
    cue: "Add a component for this todo list",
    weight: { input: 3200, output: 420, cost: 0.017 },
    steps: [
      {
        thinking:
          "The skill extending shows how a part for the TUI is made, so I read the lines that name its functions first.",
        word: 'guide = skill("extending", grep("furb[.]"))',
      },
      {
        thinking:
          "A todo is an act of a kind, so a def makes it. The panel reads the acts of that kind, and the def goes in the extension.",
        word: define,
      },
      {
        thinking:
          "The panel and the rows belong to this project from now on, so extend writes the def and the part as an extension that every chain runs.",
        word: extend,
      },
      { thinking: "The part loads at once. Three items show that it draws them.", word: items },
      {
        thinking: "The panel stands over the input, so I report.",
        word: 'close("The Todo panel stands over the input, and the feed shows a line for each todo. It lives in .furb/extensions/todo: todo.py makes the acts and tui.ts draws them. Make one with todo(item).")',
      },
    ],
  },
  {
    cue: "Sort the Todo panel",
    weight: { input: 3900, output: 300, cost: 0.014 },
    steps: [
      { thinking: "I read the lines of the part that build the panel.", word: read },
      {
        thinking: "The open items come first, so the panel sorts the items on their done flag.",
        word: sorted,
      },
      {
        thinking: "The part loads again by itself, so I report.",
        word: 'close("The Todo panel lists the open items first.")',
      },
    ],
  },
  {
    cue: "The Todo panel shows an error",
    weight: { input: 4300, output: 260, cost: 0.012 },
    steps: [
      { thinking: "The panel failed, so I read the part again to find what the sort reads.", word: read },
      {
        thinking:
          "The sort reads seen.todos, which a panel does not have. The acts of the list come from items(seen).",
        word: mended,
      },
      {
        thinking: "The part loads again, so I report.",
        word: 'close("The sort read seen.todos, which does not exist. It reads items(seen) now, and the open items come first.")',
      },
    ],
  },
];
