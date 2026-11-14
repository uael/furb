---
name: ouroboros
description: Teaches a model how to extend furb, and gives extend(), which writes a skill of the project.
metadata:
  furb-word: ouroboros.py
  furb-interface: ouroboros.pyi
---

# Extending furb

furb has no plugin surface. What extends it is python, as a word is, and the record shows who made each act. Extend it in the smallest of three ways that serves.

- A def in a word lives with its chain: a chain with a source and a later life run it again. Take it for the work of this chain.
- `extend(name, *rungs)` lives with the project: it writes the words of those rungs as the word of the skill `name` in `.furb/skills/name`, and turns it on in `.furb/config.json`, so it plays on every chain, now and in every later life. Take it for a helper that later work needs too, and give it only rungs that define.
- An official skill lives in furb: a word, a contract, a suite and an ear of the World in the crate. Take it for what every project needs, and ask the operator first.

`enabled()` gives the skills that the life runs. `bound_by()` gives each name that a word of the chain bound, with the rung of that word. `dump(*names)` gives the words that define the functions and the classes that it names, which a word of another life runs to define them again. Their contract is `ouroboros.pyi`.

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

## A skill with code

A skill with code is a folder that a host enables on each chain of a life, and on each chain born later, or on the root
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

The life word starts the work of the skill on each chain. The file `.furb/config.json` of a project turns it on, or
`.furb/config.local.json` beside it, which git ignores, for a skill of this machine alone:

```json
{ "skills": { "notes": "path/to/notes" } }
```

An official skill of furb also has a contract, `notes.pyi`, a suite, `test/`, and, when it reads the disk, an ear
of the World in the crate, `src/world/notes.rs`.
[The guide of the skills](https://github.com/uael/furb/blob/main/docs/skills.md) says the rest.
