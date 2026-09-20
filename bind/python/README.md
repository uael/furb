# furb for python, with the engine in monty

`furb_monty.engine` holds every name of `engine.pyi`, one to one, over one life of the engine running in the
sandbox of monty with the Kernel of the crate. The word of a model runs where the engine runs and reaches nothing
of yours: the one way out is the World you hand `boot`, which is a generator as it is for the engine of this
interpreter, and every value that crosses is plain data.

```python
import asyncio

from furb_monty import engine


def world():
  """A World: it hears every fact, and says what it knows."""
  while True:
    match (yield):
      case ("stand", qid, *_):
        yield "done", qid, ((("operator", (), 200_000),), "/yard", "m/low")


async def main():
  root = engine.boot(world=world())
  print(root)  # chain://operator.1
  print(engine.cwd(on=root))  # /yard


asyncio.run(main())
```

`FURB_ENGINE=monty` makes `from furb import engine` give this module, so a program of furb runs on monty with
no line of it changed. A life of monty takes no Kernel: the crate holds one, and `boot` refuses a generator under
that name.

## Build

The distribution is `furb-monty`, a member of the uv workspace at the root of the repository, which `uv sync`
builds with maturin for the interpreter of the environment. `uv run pytest -q` then runs the suite of the engine
on both engines.
