"""A life as a program holds it, in this process, on a claude command line that answers from a script."""

import asyncio
import json
import shutil
import sys
from pathlib import Path

import pytest

import furb

FAKE = Path(__file__).parents[2] / "src" / "world" / "provider" / "fake-claude.sh"

pytestmark = pytest.mark.skipif(
  sys.platform == "win32", reason="the claude of the tests is a script of sh, which Windows does not start"
)


async def scripted(tmp_path: Path, *words: str) -> furb.Life:
  """A life in a folder of the test, on a claude command line that answers each prompt with the next word."""
  fake = tmp_path / "fake"
  fake.mkdir()
  shutil.copy(FAKE, fake / "claude")
  (fake / "claude").chmod(0o755)
  for n, word in enumerate(words, 1):
    (fake / f"word.{n}").write_text(json.dumps(word))
  return await furb.open(tmp_path / "life.jsonl", directory=tmp_path, claude=str(fake / "claude"), extensions=False)


async def test_the_acts_of_a_chain_have_named_fields_and_a_chain_names_its_actor(tmp_path: Path) -> None:
  """The acts of a chain have named fields, and a chain names the actor of its prompts."""
  life = await scripted(tmp_path, 'close("done")')
  try:
    other = life.chain("other", model="claude-cli:sonnet")
    assert (life.actor, other.actor, other.parent and other.parent.label) == (
      "claude-cli:opus/high",
      "claude-cli:sonnet",
      "root",
    )
    assert await life.prompt(str, "Say done.") == "done"
    acts = [(one.kind, one.id, one.by, one.done) for one in life.acts() if one.kind in ("chain", "prompt")]
    assert acts == [("chain", other.id, "operator", False), ("prompt", "prompt1", "operator", True)]
    assert [one.value for one in life.acts() if one.kind == "prompt"] == ["done"]
  finally:
    life.end()


async def test_a_wait_of_a_life_that_its_end_cuts_raises_that_the_life_ended(tmp_path: Path) -> None:
  """A wait of a program on a life raises once the life ended, since nothing of it moves again, and so does a wait
  that starts after the end."""
  life = await scripted(tmp_path)
  waits = asyncio.ensure_future(life.until(lambda: False))
  await asyncio.sleep(0)
  life.end()
  with pytest.raises(RuntimeError, match=r"The life ended\."):
    await asyncio.wait_for(waits, 5)
  with pytest.raises(RuntimeError, match=r"The life ended\."):
    await life.until(lambda: False)
