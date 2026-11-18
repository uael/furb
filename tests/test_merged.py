"""Merged, the question of whether the stderr of a command flows into its stdout."""

from conftest import WORLD, acts, born, said, settle
from furb import engine
from furb.engine import TAIL


async def test_a_merged_is_the_question_of_whether_the_stderr_of_a_command_flows_into_its_stdout() -> None:
  """A merged is the question of whether the stderr of a command flows into its stdout, which the command answers from what its verb was given, and which the World asks when it takes the command."""
  _, log, root = born()
  one = engine.bash("plain", on=root)
  two = engine.bash("split", show_err=TAIL, on=root)
  await settle()
  asked = [word for word in acts(log).values() if word[0] == "merged"]
  assert asked == [("merged", "merged1", WORLD, root, one), ("merged", "merged2", WORLD, root, two)]
  answered = [word for word in said(log, "done") if word[1].startswith("merged")]
  assert answered == [("done", "merged1", one, True), ("done", "merged2", two, False)]
  assert log.index(answered[0]) < min(i for i, word in enumerate(log) if word[0] == "out")


async def test_the_world_runs_a_merged_command_with_its_stderr_in_its_stdout() -> None:
  """The World runs a merged command with its stderr in its stdout, so no out fact of it is of its stderr, and the door of its stderr stays empty."""
  # The World of the suite writes as the World of the crate does, which the test of the crate
  # the_world_writes_the_stderr_of_a_merged_command_into_its_stdout proves; the engine reads each out fact by the
  # stream that it names, so the door of the stderr of a merged command stays empty and that of another does not.
  sand, log, root = born(auto=False)
  merged, apart = engine.bash("one", on=root), engine.bash("two", show_err=TAIL, on=root)
  await settle()
  for one in (merged, apart):
    sand.writes(one, "a\n", "stdout")
    sand.writes(one, "b\n", "stderr")
  await settle()
  assert [(a[1], a[4]) for a in said(log, "out")] == [
    (merged, "stdout"),
    (merged, "stdout"),
    (apart, "stdout"),
    (apart, "stderr"),
  ]
  doors = [
    engine.read(f"{one}/{stream}", on=root).content for one in (merged, apart) for stream in ("stdout", "stderr")
  ]
  assert doors == ["a\nb\n", "", "a\n", "b\n"]
