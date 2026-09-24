"""Merged, the question of whether the stderr of a command flows into its stdout."""

from conftest import STANDS, Sand, life, said, settle
from furb import engine
from furb.engine import TAIL, WORLD


async def test_a_merged_is_the_question_of_whether_the_stderr_of_a_command_flows_into_its_stdout() -> None:
  """A merged is the question of whether the stderr of a command flows into its stdout, which the command answers from what its verb was given, and which the World asks before it starts the command."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  one = engine.bash("plain", on=root)
  two = engine.bash("split", show_err=TAIL, on=root)
  await settle()
  asked = [word for word in engine.asked.values() if word[0] == "merged"]
  assert asked == [("merged", "merged@world.1", WORLD, root, one), ("merged", "merged@world.2", WORLD, root, two)]
  answered = [word for word in said(log, "done") if word[1] in engine.asked and word[1].startswith("merged@")]
  assert answered == [("done", "merged@world.1", one, True), ("done", "merged@world.2", two, False)]
  assert log.index(answered[0]) < min(i for i, word in enumerate(log) if word[0] == "out")
