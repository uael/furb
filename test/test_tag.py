"""Tag, one thing a turn says."""

import json

from conftest import STANDS, Sand, life, said, settle, shown, sown, tags, wire
from furb import engine
from furb.engine import Text

EVERY = (
  "read('a.txt')\n"
  "write(Text('b.txt', 'x'))\n"
  "peek(__name__)\n"
  "turns()\n"
  "clock()\n"
  "chance()\n"
  "gate('k = 9')\n"
  "cd('/x')\n"
  "cwd()\n"
  "debug(t'{1}')\n"
  "close(1)\n"
)
NAMES = {
  "opened",
  "closed",
  "shown",
  "raised",
  "debugged",
  "refused",
  "paused",
  "woke",
  "cancelled",
  "ledger",
  "read",
  "write",
  "peek",
  "turns",
  "clock",
  "chance",
  "cd",
  "gate",
  "cwd",
}


async def test_one_thing_a_turn_says() -> None:
  """One thing a turn says: its name, what it holds of its own, and what it is of, which is a text and its show when a text is what it shows."""
  sand = Sand(files={"/w/n.txt": "one\ntwo\n"}, stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["read('n.txt', span(1, 1))\nclose(1)"]
  assert await engine.prompt(int, "read it", on=root) == 1
  carried = [tag for a in said(log, "tell") for tag in a[3] if tag[0] == "read"]
  assert [tag[:2] for tag in carried] == [("read", [("path", "n.txt")])]
  ((text, show),) = carried[0][2]
  assert text == Text("/w/n.txt", "one\ntwo\n") and show(text.lines) == [1]
  told = tags(engine.turns(on=root), "read")[0]
  assert (told[0], told[1]) == ("read", [("path", "n.txt")])
  assert shown(told) == [("shown", [("path", "/w/n.txt"), ("known", 0)], "1 one")]


async def test_a_tag_is_a_name_attributes_as_pairs_of_a_name_and_a_value_and_a_body() -> None:
  """A tag is a name, attributes as pairs of a name and a value that the World makes plain, and a body."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  act = engine.rung("k = 1", on=root)
  assert await act is None
  told = tags(engine.turns(on=root))
  assert told[2] == ("opened", [("id", act)], "k = 1")
  assert all(isinstance(key, str) for tag in told for key, _ in tag[1])
  made = [wire(tag) for tag in told]
  assert json.loads(json.dumps(made)) == made


async def test_each_tag_of_a_user_turn_that_is_of_an_act_names_the_act_by_its_id() -> None:
  """Each tag of a user turn that is of an act names the act by its id among its attributes, under the name id for what an act tells and over for a control, and a tag of a query stands at the place in the run where the query was asked."""
  sand = Sand(files={"/w/n.txt": "one\n"}, stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["read('n.txt')\nclose(1)"]
  assert await engine.prompt(int, "read it", on=root) == 1
  _, step, *_ = said(log, "rung")[0]
  told = tags([turn for turn in engine.turns(on=root) if turn[0] == "user"])
  assert [tag[0] for tag in told if not {"id", "over"} & {key for key, _ in tag[1]}] == ["read"]
  mine = [i for i, tag in enumerate(told) if ("id", step) in tag[1]]
  assert mine[0] < [tag[0] for tag in told].index("read") < [tag[0] for tag in told].index("closed")
  engine.pause(root)
  assert [tag[1] for tag in tags(engine.turns(on=root), "paused")] == [[("over", root)]]


async def test_the_tags_of_the_file() -> None:
  """The tags of the file are opened, closed, shown, raised, debugged, refused, paused, woke, cancelled, ledger, and one for each query the model may say: read, write, peek, turns, clock, chance, gate, cd and cwd."""
  sand = sown()
  _, root = life(sand)
  ceiling = engine.grant(usd=10.0, on=root)
  await settle()
  sand.script[root] = ["BAD = 1", "raise ValueError('boom')", EVERY]
  assert await engine.prompt(int, "everything", on=root) == 1
  await settle()
  engine.pause(root)
  engine.wake(root)
  engine.cancel(ceiling)
  await settle()
  got = engine.turns(on=root)
  assert {tag[0] for tag in tags(got)} | {one[0] for tag in tags(got) for one in shown(tag)} == NAMES
