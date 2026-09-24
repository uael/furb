"""Stood, what a chain stands on from its place in the record on."""

from conftest import STANDS, Sand, life, paragraphs, plain, relived, said, settle, stood
from furb import engine

LATER = [[["operator", [], 200000], ["o", ["low"], 200000]], "/z", "o/low"]
"""What a second World offers: another roster, another directory and another default actor."""

THIRD = [[["operator", [], 200000], ["p", ["low"], 200000]], "/y", "p/low"]
"""What a third World offers."""


async def test_a_stood_says_what_a_chain_stands_on_from_its_place_in_the_record_on() -> None:
  """A stood says what a chain stands on from its place in the record on, and the journal keeps it, so a later life says it again at that place."""
  first = Sand(stands=STANDS)
  _, root = life(first)
  first.script[root] = ["close(1)"]
  assert await engine.prompt(int, "one", on=root) == 1
  await settle()
  second = Sand(stands=LATER)
  await relived(second, plain(first.record))
  second.script[root] = ["close(2)"]
  assert await engine.prompt(int, "two", on=root) == 2
  await settle()
  assert [e[0][:2] for e in second.record][:1] == [("stood", root)]
  third = Sand(stands=THIRD)
  heard, _ = await relived(third, [*plain(first.record), *plain(second.record)])
  stood = said(heard, "stood")
  assert [(a[1], a[2]) for a in stood] == [(root, "record"), (root, "journal")]
  asked = [a[6] for a in said(heard, "rung") if not a[4]]
  assert asked == ["m/low", "o/low"] and said(heard, "ask") == []
  assert heard.index(stood[0]) < heard.index([a for a in said(heard, "rung") if not a[4]][1])
  assert engine.modules[root]["actor"] == "p/low" and engine.cwd(on=root) == "/y"


async def test_a_chain_that_hears_a_stood_binds_that_standing() -> None:
  """A chain that hears a stood binds that standing and its default actor, and tells it there under the headers roster, cwd and actor, so the transcript grows at one end."""
  first = Sand(stands=STANDS)
  _, root = life(first)
  assert await engine.rung("k = 1", on=root) is None
  await settle()
  was = engine.turns(on=root)
  _, over = await relived(Sand(stands=LATER), plain(first.record))
  now = engine.turns(on=over)
  assert now[0][1][: len(was[0][1])] == was[0][1]
  assert (
    paragraphs(now)[-1]
    == stood(root, LATER)
    == "\n".join(
      ["#chain1 roster [['operator', [], 200000], ['o', ['low'], 200000]]", "#chain1 cwd /z", "#chain1 actor o/low"]
    )
  )
  assert engine.modules[root]["actor"] == "o/low"
