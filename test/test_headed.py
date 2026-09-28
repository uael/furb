"""headed, the paragraph of an act."""

from conftest import born, paragraphs
from furb import engine


async def test_the_paragraph_of_an_act() -> None:
  """The paragraph of an act: its header, # and the id with no space between, then what happened, and under it the binding of each word, in order."""
  _, _, root = born()
  step = engine.rung("k = 1", on=root)
  await step
  assert engine.headed(step, "raised") == f"#{step} raised"
  assert engine.headed(step) == f"#{step}"
  assert engine.headed(step, word="a\nb") == f"#{step}\n<s:{step}_word>\na\nb</s:{step}_word>"
  assert engine.headed(step, "ledger", spent=0.5, filled=None) == f"#{step} ledger\n{step}_spent = 0.5"
  standing = f"#{root} standing\n{root}_cwd = '/w'\n{root}_actor = 'm/low'"
  assert engine.headed(root, "standing", cwd="/w", actor="m/low") == standing
  asking = engine.prompt(int, "count them\n\nall of them", on=root)
  quote = f"<s:{asking}_message>\ncount them\n\nall of them</s:{asking}_message>"
  assert engine.headed(asking, message="count them\n\nall of them") == f"#{asking}\n{quote}"
  got = paragraphs(engine.turns(on=root))
  assert got[3] == f"#{asking}\n{quote}\n{asking}: Act[int] = Act('{asking}')"
  engine.cancel(asking)


async def test_headed_binds_a_value_by_its_repr_when_that_repr_is_python_that_runs_in_the_chain_of_the_act() -> None:
  """headed binds a value by its repr when that repr is python that runs in the chain of the act, and binds any other value by what reads says, or by a peek of the act when reads says nothing."""
  _, _, root = born()
  word = (
    "class P:\n"
    "  def __repr__(self):\n"
    "    return '<p>'\n"
    "\n"
    "peeks = headed(acting(), 'closed', value=P())\n"
    "reads = headed(acting(), 'closed', 'held[0]', value=P())\n"
    "plain = headed(acting(), 'closed', 'held[0]', value=[1, 'b', Refused('no')])\n"
    "class Q:\n"
    "  def __repr__(self):\n"
    "    return 'Q()'\n"
    "\n"
    "mine = headed(acting(), 'closed', value=Q())\n"
    "theirs = headed('chain2', 'closed', value=Q())"
  )
  two = engine.chain("two")
  step = engine.rung(word, on=root)
  await step
  got = [engine.module(root)[name] for name in ("peeks", "reads", "plain", "mine", "theirs")]
  assert got == [
    f"#{step} closed <p>\n{step}_value = peek({step!r})",
    f"#{step} closed <p>\n{step}_value = held[0]",
    f"#{step} closed\n{step}_value = [1, 'b', Refused('no')]",
    f"#{step} closed\n{step}_value = Q()",
    f"#{two} closed Q()\n{two}_value = peek({two!r})",
  ]


async def test_headed_shows_a_value_as_python_shows_it_less_the_path_of_the_module() -> None:
  """headed shows a value as python shows it, less the path of the module before the name of its class, since a chain binds a class by its name alone."""
  _, _, root = born()
  word = (
    "class Deep:\n"
    "  def __repr__(self):\n"
    "    return 'a.b.Deep()'\n"
    "\n"
    "deep = headed(acting(), 'closed', value=Deep())\n"
    "half = headed(acting(), 'closed', value=0.5)"
  )
  step = engine.rung(word, on=root)
  await step
  assert [engine.module(root)[name] for name in ("deep", "half")] == [
    f"#{step} closed\n{step}_value = Deep()",
    f"#{step} closed\n{step}_value = 0.5",
  ]
