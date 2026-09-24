"""Out, the streams of a command as they come."""

from conftest import BASH, STANDS, Sand, life, made, relived, settle, texted, verb
from furb import engine
from furb.engine import WORLD


async def test_the_streams_of_a_command_come_as_out_facts_while_the_command_runs() -> None:
  """The streams of a command come as out facts while the command runs, which the record keeps."""
  sand = Sand(stands=STANDS, auto=False, words=BASH)
  _, root = life(sand)
  act = verb("bash", root)("run", show_err=made(root, "span", -9, -1))
  engine.send("out", str(act), "one\n", "stdout", by=WORLD)
  engine.send("out", str(act), "bad\n", "stderr", by=WORLD)
  await settle()
  assert texted(verb("read", root)(f"{act}/stdout"))[1] == "one\n"
  assert texted(verb("read", root)(f"{act}/stderr"))[1] == "bad\n"
  kept = [fact for fact, *_ in sand.record if fact[0] == "out"]
  assert [(one[4], one[3]) for one in kept] == [("stdout", "one\n"), ("stderr", "bad\n")]


async def test_a_later_life_reads_the_parts_that_a_command_told_before_the_death_of_the_process() -> None:
  """A later life reads the parts that a command told before the death of the process."""
  sand = Sand(stands=STANDS, auto=False, words=BASH)
  _, root = life(sand)
  act = verb("bash", root)("run")
  engine.send("out", str(act), "half\n", "stdout", by=WORLD)
  await settle()
  later = Sand(stands=STANDS, auto=False, words=BASH)
  _, over = await relived(later, list(sand.record))
  assert texted(verb("read", over)(f"{act}/stdout"))[1] == "half\n"
  assert [one for one in later.calls if one[0] == "start"] == []
  assert over == root
