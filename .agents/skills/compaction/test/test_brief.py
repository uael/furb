"""BRIEF, the message of the prompt of choice."""

from pathlib import Path

from furb import engine
from skills.conftest import compacting


async def test_brief_is_the_message_of_the_prompt_of_choice(tmp_path: Path) -> None:
  """BRIEF is the message of the prompt of choice: which acts to keep, and how to close."""
  sand, root, _ = compacting(tmp_path)
  word = (
    "kept = 'keep' in BRIEF and 'close((ids, summary))' in BRIEF\nBRIEF = 'pick'\nc = choice()\nawait c\nclose(kept)"
  )
  sand.script[root] = [word, "close(([], ''))"]
  assert await engine.prompt(bool, "brief", on=root) is True
  assert [a[5] for a in engine.transcript(root) if a[0] == "prompt" and a[4] == "tuple"] == ["pick"]
