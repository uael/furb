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
