---
name: ouroboros
description: Teaches a model how to extend furb, and gives extend(), which writes an extension of the project.
metadata:
  furb-word: ouroboros.py
  furb-interface: ouroboros.pyi
---

# Extending furb

furb has no plugin surface. What extends it is python, as a word is, and the record shows who made each act. Extend it in the smallest of three ways that serves.

- A def in a word lives with its chain: a chain with a source and a later life run it again. Take it for the work of this chain.
- `extend(name, *rungs)` lives with the project: it writes the words of those rungs as the word of the extension `name` in `.furb/skills/name`, and turns it on in `.furb/config.json`, so it plays on every chain, now and in every later life. Take it for a helper that later work needs too, and give it only rungs that define.
- `extend(name, *rungs, tui=module)` also writes `module`, TypeScript, as the part of the extension that the TUI loads live (see below).
- An official extension lives in furb: a word, a contract, a suite and an ear of the World in the crate. Take it for what every project needs, and ask the operator first.

`enabled()` gives the extensions that the life runs. `bound_by()` gives each name that a word of the chain bound, with the rung of that word. Their contract is `ouroboros.pyi`.

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

An extension is a folder that a host enables on each chain of a life, and on each chain born later, or on the root
alone when its manifest says `furb-plays: root`. The record keeps that it is on, so a later life runs it again.

```text
notes/
  SKILL.md    the manifest, a skill whose metadata holds the fields of furb
  notes.py    the word: a python module that imports from furb.engine what it uses, and only defines
```

```markdown
---
name: notes
description: Keeps the notes of a chain.
metadata:
  furb-word: notes.py
  furb-life: act('notes', '', notes)
---
What the notes are for.
```

The life word starts the work of the extension on each chain. The file `.furb/config.json` of a project turns it on, or
`.furb/config.local.json` beside it, which git ignores, for an extension of this machine alone:

```json
{ "extensions": { "notes": "path/to/notes" } }
```

## A part for the TUI

An extension can also give the TUI a part: a module of TypeScript beside its word, which its manifest names under
`furb-view`. The TUI loads it at its start, and again when its file changes or when a write of the extension lands, with
no restart. The module imports no value: it exports a function, which the TUI calls with `furb`.

```text
todo/
  SKILL.md    its metadata gives furb-word: todo.py and furb-view: tui.ts
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
a rule with the title and the lines under it, after the last act whose `began` is before `at`. A key acts while no
dialog is open, and a command runs from the palette or as `/name`. A key or a command of the TUI wins.
`extend(name, *rungs, tui=module)` writes the part with the word.

A panel reads the acts of a kind, so make them with a def in the word of the extension, as `todo` above does,
and `seen.of("todo")` gives them in the order of the record.

Check a part before you say that it works. The TUI shows a part that fails, in plain words, in a panel under the name of
its extension, and only the operator sees it. A module that has a syntax error, one that imports a value, and a
function that throws when it runs all show there. So run `bun build --no-bundle .furb/skills/todo/tui.ts`, which
finds a syntax error, and read your code again for each name that `seen` and `furb` do not have. When the operator
tells you an error, read the file again, change it with `replace`, and write it: the panel draws again at once.
`/extensions` in the TUI lists what each part gives. To take an extension away, set it to `false` in
`.furb/config.json`, and its part goes with it.

An official extension of furb also has a contract, `notes.pyi`, a suite, `test/`, and, when it reads the disk, an ear
of the World in the crate, `src/world/notes.rs`. The crate carries its part, so the TUI loads it in an installed
package too. [The guide of the extensions](https://github.com/uael/furb/blob/main/docs/skills.md) says the rest.
