"""Merged, the question of whether the stderr of a command flows into its stdout."""

from conftest import STANDS, WORLD, Sand, acts, life, said, settle
from furb import engine
from furb.engine import TAIL


async def test_a_merged_is_the_question_of_whether_the_stderr_of_a_command_flows_into_its_stdout() -> None:
  """A merged is the question of whether the stderr of a command flows into its stdout, which the command answers from what its verb was given, and which the World asks when it takes the command."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  one = engine.bash("plain", on=root)
  two = engine.bash("split", show_err=TAIL, on=root)
  await settle()
  asked = [word for word in acts(log).values() if word[0] == "merged"]
  assert asked == [("merged", "merged1", WORLD, root, one), ("merged", "merged2", WORLD, root, two)]
  answered = [word for word in said(log, "done") if word[1].startswith("merged")]
  assert answered == [("done", "merged1", one, True), ("done", "merged2", two, False)]
  assert log.index(answered[0]) < min(i for i, word in enumerate(log) if word[0] == "out")
