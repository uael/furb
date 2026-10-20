"""Tell, the fact that carries the tags of an act."""

from conftest import Sand, born, paragraphs, plain, relived, said
from furb import engine
from furb.engine import OPERATOR


async def test_a_tell_carries_notes_about_the_act_it_is_about() -> None:
  """A tell carries notes about the act it is about, and the turns are folded from them."""
  _, log, root = born()
  act = engine.rung("k = 1", on=root)
  assert await act is None
  told = [a for a in said(log, "tell") if a[1] == act]
  assert [a[3] for a in told] == [[f"#{act}\n{act}_word = 'k = 1'"]]
  assert paragraphs(engine.turns(on=root))[2:] == ["\n".join(a[3]) for a in told]


async def test_what_an_act_tells_is_the_telling_of_its_chain() -> None:
  """What an act tells is the telling of its chain, so the journal keeps none of it, and a later life tells it again."""
  sand, log, root = born()
  assert await engine.rung("k = 1", on=root) is None
  assert [engine.scope(a[1]) for a in said(log, "tell")] == [root] * len(said(log, "tell"))
  assert [entry for entry in sand.record if entry[0][0] == "tell"] == []
  was = engine.turns(on=root)
  again, over = await relived(Sand(), plain(sand.record))
  assert over == root and engine.turns(on=over) == was
  assert said(again, "tell") != []


async def test_a_fact_that_carries_notes_is_what_the_turns_are_folded_from() -> None:
  """A tell is the one fact that carries notes, which the turns are folded from: a control tells its header by a tell."""
  _, log, root = born()
  act = engine.rung("k = 1", on=root)
  assert await act is None
  held = engine.wait(9, on=root)
  engine.pause(root)
  engine.wake(root)
  engine.cancel(held)
  engine.close(None, root)
  controls = [a for a in log if a[0] in ("pause", "wake", "cancel", "close")]
  assert controls == [
    ("pause", root, OPERATOR),
    ("wake", root, OPERATOR),
    ("cancel", held, OPERATOR),
    ("close", root, OPERATOR, None),
  ]
  carrying = [a for a in log if a[0] == "tell"]
  assert [a[3] for a in carrying][-4:] == [
    [f"#{root} paused"],
    [f"#{root} woke"],
    [f"#{held} cancelled"],
    [f"#{root} closed"],
  ]
  assert paragraphs(engine.turns(on=root)) == ["\n".join(a[3]) for a in carrying]
