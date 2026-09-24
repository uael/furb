"""Merged, the question of whether the stderr of a command flows into its stdout."""

from conftest import BASH, STANDS, Sand, life, made, said, settle, verb
from furb import engine
from furb.engine import WORLD


async def test_a_merged_is_the_question_of_whether_the_stderr_of_a_command_flows_into_its_stdout() -> None:
  """A merged is the question of whether the stderr of a command flows into its stdout, which the command answers from what its verb was given, and which the World asks before it starts the command."""
  sand = Sand(stands=STANDS, words=BASH)
  log, root = life(sand)
  one = verb("bash", root)("plain")
  two = verb("bash", root)("split", show_err=made(root, "span", -9, -1))
  await settle()
  asked = [word for word in engine.asked.values() if word[0] == "merged"]
  assert [word[2:] for word in asked] == [(WORLD, root, one), (WORLD, root, two)]
  answered = [word for word in said(log, "done") if word[1] in [a[1] for a in asked]]
  assert [(word[2], word[3]) for word in answered] == [(one, True), (two, False)]
  assert log.index(answered[0]) < min(i for i, word in enumerate(log) if word[0] == "out")
