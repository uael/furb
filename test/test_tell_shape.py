"""Tell, the fact that carries the tags of an act."""

from conftest import STANDS, Sand, life, plain, relived, said, tags
from furb import engine


async def test_a_tell_carries_tags_about_the_act_it_is_about() -> None:
  """A tell carries tags about the act it is about, and the turns are folded from them."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  act = engine.rung("k = 1", on=root)
  assert await act is None
  told = [a for a in said(log, "tell") if a[1] == act]
  assert [a[3] for a in told] == [[("opened", [("id", act)], "k = 1")], [("closed", [("id", act)], None)]]
  assert [tag for a in told for tag in a[3]] == tags(engine.turns(on=root))[2:]


async def test_what_an_act_tells_is_the_telling_of_its_chain() -> None:
  """What an act tells is the telling of its chain, so the record keeps none of it, and a later life tells it again."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  assert await engine.rung("k = 1", on=root) is None
  assert [engine.scope(a[1]) for a in said(log, "tell")] == [root] * len(said(log, "tell"))
  assert [entry for entry in sand.record if entry[1][0] == "tell"] == []
  was = tags(engine.turns(on=root))
  again, over = await relived(Sand(stands=STANDS), plain(sand.record))
  assert over == root and tags(engine.turns(on=over)) == was
  assert said(again, "tell") != []


async def test_a_fact_that_carries_tags_is_what_the_turns_are_folded_from() -> None:
  """A fact that carries tags is what the turns are folded from: a tell, and a control, which carries the tag it tells."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  act = engine.rung("k = 1", on=root)
  assert await act is None
  engine.pause(root)
  engine.wake(root)
  engine.cancel(act)
  engine.close(None, root)
  carrying = [a for a in log if a[0] in ("tell", "pause", "wake", "cancel", "close")]
  assert [a[0] for a in carrying if a[0] != "tell"] == ["pause", "wake", "cancel", "close"]
  assert [tag for a in carrying for tag in (a[4] if a[0] == "close" else a[3])] == tags(engine.turns(on=root))
