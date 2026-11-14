"""offers, an act put to the ears of the outside."""

from collections.abc import Generator

from conftest import Sand, journaled, keeping, kernel, life, lived, settle, verbs
from furb import engine


def taking(kind: str) -> Generator[tuple | None, tuple]:
  """An ear of the outside that takes every act of one kind it is put, and hears every other fact."""
  while True:
    match a := (yield):
      case (str(one), about, *_) if one == kind and engine.get(about) == a:
        yield "started", about


async def test_offers_puts_an_act_to_each_ear_of_the_outside_until_one_takes_it() -> None:
  """offers puts an act to each ear of the outside in the order that boot was given them, until one takes it, and a chain to each of them, as act does with each act that it makes, and the journal binds it again to answer an act from its record first."""
  before: list[tuple] = []
  after: list[tuple] = []
  root = engine.boot(
    journaled(), **kernel(), before=keeping(before), taker=taking("note"), after=keeping(after), world=Sand().hears()
  )
  await verbs(root, "note")
  await settle()
  note = engine.act("note", root, None, "one")
  made = engine.get(note)
  assert [a for a in before if a == made] == [made] and [a for a in after if a == made] == []
  engine.offers(made)
  assert [a for a in before if a == made] == [made]
  opened = engine.get(root)
  heard = len([a for a in before if a == opened]), len([a for a in after if a == opened])
  engine.offers(opened)
  assert (len([a for a in before if a == opened]), len([a for a in after if a == opened])) == (
    heard[0] + 1,
    heard[1] + 1,
  )
  sand, _, root = await lived()
  record = list(sand.record)
  put: list[tuple] = []
  _, over = life(Sand(), record, last=keeping(put))
  await settle(300)
  assert over == root and [a[0] for a in put if engine.get(a[1]) == a and a[0] in ("read", "bash", "reply")] == []
