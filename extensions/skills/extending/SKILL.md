---
name: extending
description: "How to extend furb: a def in a word, which lives as long as its chain, and an extension, which a host enables on each chain of a life."
---

# Extending furb

furb has no plugin surface. What extends it is python, as a word is, and the record shows who made each act.

## A def in a word

A name that a word binds stays bound in the module of its chain. Each later rung of the chain sees it, a chain with
that chain as its source has it too, and a later life on the record binds it again, since it runs the word again. A
chain with no source does not have it.

```python
def tests(path: str = "") -> Act[Exit]:
  return bash(f"uv run pytest -q {path}", show=span(-30, -1))
```

A word can also rebind a verb of the engine, and it can make an act whose ear answers the reads of a scheme of its
own, which is a door:

```python
def notes(id: str):
  yield "started", id
  while True:
    match (yield):
      case ("read", qid, _, _, path) if path.startswith("notes://"):
        yield "done", qid, Text(path, "kept")


act("notes", "", notes)
```

## An extension

An extension is a folder that a host enables on each chain of a life, and on each chain born later. The record keeps
that it is on, so a later life runs it again.

```
notes/
  furb.json   { "name": "notes", "word": "notes.py", "life": "act('notes', '', notes)" }
  notes.py    the word: a python module that imports from furb.engine what it uses, and only defines
```

The life word starts the work of the extension on each chain. The file `.furb/config.json` of a project turns it on:

```json
{ "extensions": { "notes": "path/to/notes" } }
```

## A part for the TUI

An extension can also give the TUI a part: a module of TypeScript beside its word, which `furb.json` names under
`tui`. The TUI loads it at its start, and again when its file changes or when a write of the extension lands, with
no restart. The module imports no value: it exports a function, which the TUI calls with `furb`.

```
todo/
  furb.json   { "name": "todo", "word": "todo.py", "tui": "tui.ts" }
  todo.py     def todo(item: str, *, done: bool = False, on: str = "") -> Act:
                def ear(name: str) -> Iterator[tuple]:
                  yield "done", name, None
                return act("todo", on, ear, item, done)
  tui.ts      the part, below
```

```ts
export default function (furb) {
  const items = (seen) => seen.of("todo").map((act) => ({ item: String(act.words[0]), done: Boolean(act.words[1]) }));
  furb.panel("Todo", (seen) => items(seen).map(({ item, done }) => [[done ? "✓ " : "○ ", "accent"], [item]]));
  furb.row("todo", (act) => ({ label: String(act.words[0]) }));
  furb.command("todos", "Say how many items are open", (_, seen) => seen.say(`${items(seen).length} items`));
  furb.key("ctrl+o", (seen) => seen.run('todo("one more")'));
}
```

A panel stands over the input and gives its lines, each a text or a list of `[text, tone]`, where a tone is `text`,
`muted`, `faint`, `accent`, `success`, `warning` or `danger`. A row gives the label and the details of each act of
its kind in the feed. `furb.mark(draw)` gives marks, each `{ chain, at, title, lines }`: the feed of that chain shows
a rule with the title and the lines under it, after the last act whose `began` is before `at`. A key acts while no dialog is open, and a command runs from the palette or as `/name`. A key or
a command of the TUI wins. A module that fails shows its error in its panel. `extend(name, *rungs, tui=module)`
writes the part with the word.

An official extension of furb also has a contract, `notes.pyi`, a suite, `test/`, and, when it reads the disk, an ear
of the World in the crate, `src/extension/notes.rs`. The crate carries its part, so the TUI loads it in an installed
package too. The guide of the extensions of furb says the rest:
https://github.com/uael/furb/blob/main/docs/extensions.md.
