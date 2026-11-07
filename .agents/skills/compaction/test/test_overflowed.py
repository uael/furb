"""OVERFLOWED, the summary of a cut for a prompt past the window."""

from pathlib import Path

from furb import engine
from skills.conftest import compacting


async def test_overflowed_is_the_summary_that_fitted_gives_compact(tmp_path: Path) -> None:
  """OVERFLOWED is the summary that fitted gives compact, which says why the chain lost its acts."""
  sand, root, _ = compacting(tmp_path)
  sand.script[root] = ["close(OVERFLOWED)"]
  said = await engine.prompt(str, "summary", on=root)
  assert said.startswith("The model refused the last reply of this chain, since its prompt passed the window")
