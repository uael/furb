# furb for python

One life of the engine, in the sandbox, with a World you write in python.

[furb](https://github.com/uael/furb) runs the engine in monty and gives its surface to a host. This is that
surface for a host of python. The word of a model runs where the engine runs and reaches nothing of yours: the
one way out is the World you hand over, and every value that crosses is plain data.

```python
from furb_sand import Fact, Life, Voice


class Yard:
  """A World: it hears every fact, and says what it knows."""

  def hears(self, fact):
    if fact.kind == "stand":
      return Fact("done", fact.about, "", ((("operator", (), 200_000),), "/yard", "m/low"))
    return None

  def answered(self, got):
    return None


voice = Voice()
life = Life(Yard(), voice=voice)
print(life.root)  # chain://operator.1
print(life.word(f"cwd(on={life.root!r})"))  # /yard
```

`furb_sand.pyi` is the whole surface, and every name in it says what it is for.

## What a host writes

A World hears every fact and answers it: nothing at all, one fact, the facts to say in order, or an `Ask`, which
is a question of the engine that comes back through `answered`. What the World starts and does not finish there,
it says later into the `Voice` it holds, and the life hears all of it, in the order it was said, at every
`Life.heard`.

A gate is what reads the word of a rung before it runs. It is yours to write too, and a host that reads no python
leaves it out: the word then runs, raises where it stands, and the engine asks the model again.

## Build

```sh
maturin build -m bind/python/Cargo.toml
```

The module is compiled against one interpreter and imported by that one alone, so the wheel is for one
interpreter and one platform.

In a checkout, `uv run python script/bound.py` builds it for the interpreter that runs the rig, puts it where that
interpreter reads it, and runs the tests of the binding on it.
