"""Ear, any generator that hears every fact and speaks by yielding one."""

from collections.abc import Generator

from conftest import Py, sown
from furb import engine
from furb.engine import Text


def note(kept: list[tuple]) -> Generator[tuple | None, tuple]:
  """An ear of the outside that answers a read of a door of its own and keeps every fact it hears."""
  while True:
    match a := (yield):
      case ("read", qid, _, _, path) if path.startswith("note://"):
        kept.append(a)
        yield "done", qid, Text(path, "kept")
      case (_, _, _, *_):
        kept.append(a)


async def test_an_ear_is_any_generator_of_that_shape() -> None:
  """An ear is any generator of that shape, so the World and the Kernel are ears, and boot takes an ear of the outside under any name it is to hear by."""
  sand, py = sown(), Py()
  kept: list[tuple] = []
  root = engine.boot((), world=sand.hears(), kernel=py.kernel(), note=note(kept))
  sand.script[root] = ["close(1)"]
  assert await engine.prompt(int, "work", on=root) == 1
  assert engine.read("note://one", on=root) == Text("note://one", "kept")
  assert engine.read("a.txt", on=root) == Text("/w/a.txt", "one\ntwo\n")
  assert [a[0] for a in sand.calls] == ["stand", "ask", "read", "read"]
  assert py.gated and py.ran == ["close(1)"]
  assert [a[4] for a in kept if a[0] == "read"] == ["note://one"]
  assert {"chain", "prompt", "rung", "ask", "answer", "run", "ran"} <= {a[0] for a in kept}
