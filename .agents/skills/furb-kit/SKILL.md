---
name: furb-kit
description: How to make and use a kit of furb, a skill with code, and extend(), which writes a kit of the project or of the session. Use it when a word should outlive its chain, as a helper, a list with a view, or a door to a tool.
metadata:
  furb-word: furb-kit.py
  furb-interface: furb-kit.pyi
---

# Make a kit

A kit is a skill with code: a folder with a `SKILL.md` whose metadata names a word of python, which each chain where it
plays runs before any other word, so the functions that it defines are there for the model. The record keeps that it
plays, so a later life runs it again. Extend furb in the smallest of three ways that serves.

- A def in a word lives with its chain: a chain with a source and a later life run it again. Take it for the work of
  this chain.
- `extend(name, *rungs)` makes a kit: it writes the words of those rungs as the word of the kit `name`. With
  `session=True` the kit lives in the record alone, and plays on each chain of this session and of each later life on
  its record. Without it, the kit lives in `.furb/skills/name` of the project and `.furb/config.json` turns it on, so
  every later session of the project plays it. Give it only rungs that define.
- An official kit lives in furb: a word, a contract, a suite and a process in `.agents/skills`. Take it for what every
  project needs, and ask the operator first.

`enabled()` gives the kits that the life runs. `bound_by()` gives each name that a word of the chain bound, with the
rung of that word. `dump(*names)` gives the words that define the functions and the classes that it names, which a word
of another life runs to define them again. Their contract is `furb-kit.pyi`.

## A kit of the session with a view

A request such as a list of tasks with a view is a kit of the session. Define its state and its functions in one word,
then give the rung that bound them, which `bound_by()` names, to `extend`, with `view` naming the value of the root that holds the tree of the view:

```python
todo_view = {"id": "todo", "node": "list", "items": []}


def todo(*tasks: str) -> dict:
  view = module(ROOT)["todo_view"]
  view["items"] += [{"id": f"task{len(view['items'])}", "node": "text", "runs": [{"text": one}]} for one in tasks]
  return view
```

```python
kit = extend("todo", bound_by()["todo"], session=True, view="todo_view")
todo("write the tests")
```

Each window that mounts the view of the kit draws the tree that the value holds, and draws it again each time that the
value changes. A node is one of `box`, `text`, `list`, `table`, `input`, `button`, `progress`, `code`, `diff` and
`markdown`, each with an `id`, as `docs/protocol.md` says under Views. A call of `extend` with the same name adds the
words of its rungs to the word of the kit, and each chain plays the new definitions.

## A def in a word

A name that a word binds stays bound in the module of its chain. A word can also rebind a verb of the engine, and it can
make an act whose ear answers the reads of a scheme of its own, which is a door:

```python
def notes(id: str):
  yield "started", id
  while True:
    match (yield):
      case ("read", qid, _, _, path) if path.startswith("notes://"):
        yield "done", qid, Text(path, "kept")


act("notes", "", notes)
```

## The parts of a kit

```text
notes/
  SKILL.md    the manifest, a skill whose metadata holds the fields of furb
  notes.py    the word: a python module that imports from furb.engine what it uses, and only defines
  notes.pyi   the contract, which it can have: the laws of the kit, one sentence on each line
  test/       the suite, which it can have: one test for each sentence of the contract
  process.py  the World part, which it can have: a process that answers the acts of the kinds that it names
```

```markdown
---
name: notes
description: Keeps the notes of a chain.
metadata:
  furb-word: notes.py
  furb-interface: notes.pyi
  furb-life: act('notes', '', notes)
  furb-process: process.py
  furb-acts: notes_add
---
What the notes are for.
```

The life word starts the work of the kit on each chain. The word runs in the sandbox of the engine, which reaches the
disk, the network and each tool only through acts. What it cannot do there, as a call of a server or of a library, is
the World part: a process in any language that the life starts at the first act of a kind that `furb-acts` names, and
speaks to with one frame of JSON on each line of its stdin and its stdout. The file `.furb/config.json` of a project
turns a kit on, or `.furb/config.local.json` beside it, which git ignores, for a kit of this machine alone:

```json
{ "skills": { "notes": "path/to/notes" } }
```

[The guide of the skills](https://github.com/uael/furb/blob/main/docs/skills.md) says each field and the rest.
