"""Out, the streams of a command as they come."""

from conftest import STANDS, Sand, life, relived, settle, world_says
from furb import engine
from furb.engine import TAIL


async def test_the_streams_of_a_command_come_as_out_facts_while_the_command_runs() -> None:
  """The streams of a command come as out facts while the command runs, which the record keeps."""
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  act = engine.bash("run", show_err=TAIL, on=root)
  world_says("out", act, "one\n", "stdout")
  world_says("out", act, "bad\n", "stderr")
  await settle()
  assert engine.read(f"{act}/stdout", on=root).content == "one\n"
  assert engine.read(f"{act}/stderr", on=root).content == "bad\n"
  kept = [fact for fact, *_ in sand.record if fact[0] == "out"]
  assert [(one[4], one[3]) for one in kept] == [("stdout", "one\n"), ("stderr", "bad\n")]


async def test_a_later_life_reads_the_parts_that_a_command_told_before_the_death_of_the_process() -> None:
  """A later life reads the parts that a command told before the death of the process."""
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  act = engine.bash("run", on=root)
  world_says("out", act, "half\n", "stdout")
  await settle()
  later = Sand(stands=STANDS, auto=False)
  _, over = await relived(later, list(sand.record))
  assert engine.read(f"{act}/stdout", on=over).content == "half\n"
  assert [one for one in later.calls if one[0] == "bash"] == []
  assert over == root
