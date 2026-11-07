"""OVERFLOW, what a refusal says of a prompt past the window of its model."""

from pathlib import Path

from furb import engine
from skills.conftest import compacting


async def test_overflow_is_what_the_refusal_of_a_reply_says_first(tmp_path: Path) -> None:
  """OVERFLOW is what the refusal of a reply says first when the prompt of the reply passes the window of its model."""
  sand, root, _ = compacting(tmp_path)
  sand.script[root] = ["close(OVERFLOW)"]
  assert await engine.prompt(str, "overflow", on=root) == "the prompt passes the window of the model"
