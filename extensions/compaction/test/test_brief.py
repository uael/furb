"""BRIEF, what the prompt of choice says first."""

from pathlib import Path

from extensions.conftest import compacting
from furb import engine


async def test_brief_is_what_the_prompt_of_choice_says_first(tmp_path: Path) -> None:
  """BRIEF is what the prompt of choice says first: which acts to keep, and how to close."""
  sand, root, _ = compacting(tmp_path)
  sand.script[root] = ["kept = 'keep' in BRIEF and 'tuple' in BRIEF\nBRIEF = 'pick'\nc = choice()\nclose(kept)"]
  assert await engine.prompt(bool, "brief", on=root) is True
  assert [str(a[5]).split("\n")[:3] for a in engine.transcript("chain2") if a[0] == "prompt"] == [
    ["pick", "", "Open prompts:"]
  ]
