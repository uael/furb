"""A life as a program holds it, in this process, on a claude command line that answers from a script."""

import json
import shutil
from pathlib import Path

import furb

FAKE = Path(__file__).parents[2] / "src" / "world" / "provider" / "fake-claude.sh"


async def test_the_acts_of_a_chain_have_named_fields_and_a_chain_names_its_actor(tmp_path: Path) -> None:
  """The acts of a chain have named fields, and a chain names the actor of its prompts."""
  fake = tmp_path / "fake"
  fake.mkdir()
  shutil.copy(FAKE, fake / "claude")
  (fake / "claude").chmod(0o755)
  (fake / "word.1").write_text(json.dumps('close("done")'))
  life = await furb.open(tmp_path / "life.jsonl", directory=tmp_path, claude=str(fake / "claude"), extensions=False)
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
