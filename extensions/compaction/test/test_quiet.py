"""quiet, the filter that drops the commands."""

from pathlib import Path

from conftest import of, settle
from extensions.conftest import compacting
from furb import engine


async def test_quiet_drops_each_command_with_its_output(tmp_path: Path) -> None:
  """quiet() drops each command with its output, and keeps the rung whose word ran it, so the word stays in the turns."""
  sand, root, _ = compacting(tmp_path)
  word = "x = bash('echo hi')\nawait x"
  sand.script[root] = [word, "close(chain('two', 'chain1', quiet()))"]
  two = await engine.prompt(str, "run and compact", on=root)
  await settle()
  assert of(engine.turns(on=root), "bash") != [] and of(engine.turns(on=two), "bash") == []
  assert word in [turn[1] for turn in engine.turns(on=two) if turn[0] == "assistant"]
