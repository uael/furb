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

An official extension of furb also has a contract, `notes.pyi`, a suite, `test/`, and, when it reads the disk, an ear
of the World in the crate, `src/extension/notes.rs`. The guide of the extensions of furb says the rest:
https://github.com/uael/furb/blob/main/docs/extensions.md.
