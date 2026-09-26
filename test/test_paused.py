"""paused, whether an act is paused."""

from conftest import Sand, life, said, settle
from furb import engine


async def test_paused_reads_whether_an_act_is_paused_off_the_transcript_of_its_chain() -> None:
  """paused reads whether an act is paused off the transcript of its chain, so an ear born while a pause over its act stands is born paused, and takes its act all the same."""
  sand = Sand()
  log, root = life(sand)
  engine.pause(root)
  step = engine.rung("k = 1", on=root)
  await settle()
  assert engine.paused(root) and engine.paused(step)
  assert [a[2] for a in said(log, "started") if a[1] == step] == [step] and engine.peek(step, ...) is ...
  engine.wake(root)
  await settle()
  assert not engine.paused(step) and await step is None and engine.module(root)["k"] == 1
