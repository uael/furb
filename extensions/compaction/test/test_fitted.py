"""fitted, a chain that its model refuses as too long goes on with no pause."""

from dataclasses import dataclass
from pathlib import Path

from conftest import Sand, World, settle
from extensions.conftest import extended
from furb import engine
from furb.engine import Refused

BIG = "\n".join(f"line {n} " + "x" * 50 for n in range(200)) + "\n"
"""BIG is a file whose tell passes the window of the double."""


@dataclass
class Narrow(Sand):
  """A World whose model refuses each reply whose turns pass most characters, as a provider refuses a prompt past the
  window of its model, and answers the others with its script."""

  most: int = 0

  def hear(self, a: tuple) -> World:
    """A reply whose turns pass most characters is refused with OVERFLOW, and every other fact goes to the Sand."""
    match a:
      case ("reply", about, _, on, _) if len(repr(engine.turns(on=on))) > self.most:
        yield "started", about
        why = Refused(f"m answered nothing: {engine.module(on)['OVERFLOW']}: Prompt is too long")
        self.loop.call_soon(engine.say, "done", about, why)
      case _:
        yield from super().hear(a)


async def narrowed(tmp_path: Path, *script: str) -> tuple[Narrow, str]:
  """A life that runs the extension compaction on the Narrow World, whose folder work holds big.txt, and whose root
  fitted watches and whose model answers the script: the World and the root."""
  sand, root = extended("compaction", tmp_path, sand=Narrow())
  assert isinstance(sand, Narrow)
  sand.files[f"{tmp_path / 'work'}/big.txt"] = BIG
  await engine.rung("fitted()", on=root)
  sand.most = len(repr(engine.turns(on=root))) + 4000
  sand.script[root] = [*script]
  return sand, root


def cuts(root: str) -> list[str]:
  """The words of the rungs that fitted made on a chain."""
  return [str(a[4]) for a in engine.transcript(root) if a[0] == "rung" and str(a[2]).startswith("fitted")]


async def test_fitted_watches_a_chain(tmp_path: Path) -> None:
  """fitted() watches a chain: when its model refuses a reply of the chain with OVERFLOW, it compacts the chain at once to its open prompts with the summary OVERFLOWED, so the next reply reads less and the chain goes on with no pause."""
  _, root = await narrowed(tmp_path, "read('big.txt')", "close(7)")
  act = engine.prompt(int, "go", on=root)
  assert await act == 7
  summary = engine.module(root)["OVERFLOWED"]
  assert cuts(root) == [f"<s:summary>\n{summary}</s:summary>\nreground()"]
  assert [a for a in engine.transcript(root) if a[0] == "pause"] == []
  assert "go" in repr(engine.turns(on=root)) and "line 199" not in repr(engine.turns(on=root)[-2:])


async def test_it_compacts_once_in_a_row_of_refusals(tmp_path: Path) -> None:
  """It compacts once in a row of refusals with OVERFLOW, and again only after a reply of the chain that the model answered."""
  sand, root = await narrowed(tmp_path, "read('big.txt')")
  wide, sand.most = sand.most, 10
  engine.prompt(int, "go", on=root)
  await settle()
  assert len(cuts(root)) == 1
  sand.most = wide
  sand.script[root] = ["read('big.txt')", "close(1)"]
  engine.prompt(None, "poke", on=root)
  await settle()
  assert len(cuts(root)) == 2


async def test_it_never_completes(tmp_path: Path) -> None:
  """It never completes."""
  _, root = await narrowed(tmp_path, "w = fitted()\nclose(1)")
  assert await engine.prompt(int, "go", on=root) == 1
  await settle()
  assert engine.peek(engine.module(root)["w"], "open") == "open"
