"""listening, the ears that still hear."""

from conftest import born, settle, world_says
from furb import engine
from furb.engine import OPERATOR

PINGED = """def pinged(on=''):
  def ear(id):
    yield 'started', id
    yield 'done', id, None
    while True:
      if (yield {id, 'ping'})[0] == 'ping':
        yield told(id, 'pinged')
  return act('pinged', on, controlled(ear, lasting=True))

def brief(on=''):
  def ear(id):
    yield 'started', id
    yield 'done', id, None
    while True:
      yield {id, 'ping'}
  return act('brief', on, controlled(ear))

lasting, short = pinged(), brief()"""


async def test_listening_gives_the_ear_of_each_act_and_of_each_outside_that_still_hears_by_its_name_in_the_order_that_boot_drove_them() -> (
  None
):
  """listening gives the ear of each act and of each outside that still hears, by its name, in the order that boot drove them."""
  _, _, root = born()
  asked = engine.prompt(str, "Which?", to=OPERATOR, on=root)
  await settle()
  names = list(engine.listening())
  assert names[0] == OPERATOR and names.index("journal") < names.index(asked)
  engine.close("this one", asked)
  await settle()
  world_says("ping", root)
  await settle()
  assert asked not in engine.listening()


async def test_an_ear_lives_from_its_drive_until_it_returns_a_lasting_ear_lives_past_the_done_of_its_act_as_the_ear_of_mcp_index_hears_mcp_changed_after_it() -> (
  None
):
  """An ear lives from its drive until it returns: a lasting ear lives past the done of its act, as the ear of mcp_index hears mcp_changed after it."""
  _, _, root = born()
  await engine.rung(PINGED, on=root)
  await settle()
  lasting, short = engine.module(root)["lasting"], engine.module(root)["short"]
  world_says("ping", root)
  await settle()
  assert engine.over(lasting) and engine.over(short)
  assert lasting in engine.listening() and short not in engine.listening()
  assert [a for a in engine.transcript(root, "tell") if a[1] == lasting]


async def test_it_gives_a_new_copy_each_time_so_a_word_that_makes_an_act_while_it_loops_on_it_goes_through_each_ear_that_it_found() -> (
  None
):
  """It gives a new copy each time, so a word that makes an act while it loops on it goes through each ear that it found."""
  _, _, root = born()
  await engine.rung("n = len(listening())\nseen = 0\nfor x in listening():\n  chain('more')\n  seen += 1", on=root)
  await settle()
  assert engine.module(root)["seen"] == engine.module(root)["n"]


async def test_a_word_that_writes_into_what_it_read_leaves_each_ear_alive() -> None:
  """A word that writes into what it read leaves each ear alive."""
  _, _, root = born()
  before = list(engine.listening())
  await engine.rung("held = listening()\nheld.clear()", on=root)
  await settle()
  assert list(engine.listening()) == before
