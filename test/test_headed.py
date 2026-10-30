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
  assert engine.headed("stand2", actor="m/low") == "#stand2\nstand2_actor = 'm/low'"
  asking = engine.prompt(int, "count them\n\nall of them", on=root)
  quote = f"<s:{asking}_message>\ncount them\n\nall of them</s:{asking}_message>"
  assert engine.headed(asking, message="count them\n\nall of them") == f"#{asking}\n{quote}"
  got = paragraphs(engine.turns(on=root))
  assert got[2] == f"#{asking}\n{quote}\n{asking}: Act[int] = Act('{asking}')"
  engine.cancel(asking)


async def test_headed_binds_each_value_by_its_repr_with_no_check() -> None:
  """headed binds each value by its repr with no check, since every value that reaches it is a wire value, and the repr of a wire value is python."""
  _, _, root = born()
  word = (
    "class P:\n"
    "  def __repr__(self):\n"
    "    return '<p>'\n"
    "\n"
    "live = headed(acting(), 'closed', value=P())\n"
    "wire = headed(acting(), 'closed', value=[1, 'b', Refused('no')])"
  )
  step = engine.rung(word, on=root)
  await step
  assert [engine.module(root)[name] for name in ("live", "wire")] == [
    f"#{step} closed\n{step}_value = <p>",
    f"#{step} closed\n{step}_value = [1, 'b', Refused('no')]",
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


async def test_an_open_tells_no_string_that_the_word_that_made_it_holds() -> None:
  """An open of an act that the word of a rung on its chain made tells no string that the word holds as a whole literal, between the quotes that python writes it with, between double quotes or as a quote, since the turn of the model shows that word, and an act that tells a text tells each of its words."""
  _, _, root = born(
    "<s:note>\nrun\nthem\n</s:note>\nx = bash('echo hi')\ny = bash('echo ' + 'bye')\nz = bash(note)\nread('a.txt')\n"
    "s = 'say echo it twice'\nu = bash(\"echo yo\")\nv = bash('echo' + ' it')\nclose(1)"
  )
  assert await engine.prompt(int, "go", on=root) == 1
  got = paragraphs(engine.turns(on=root))
  assert "#bash1\nbash1: Act[Exit] = Act('bash1')" in got
  assert "#bash2\nbash2_command = 'echo bye'\nbash2: Act[Exit] = Act('bash2')" in got
  assert "#bash3\nbash3: Act[Exit] = Act('bash3')" in got
  assert "#read1\nread1_path = 'a.txt'\n<s:read1_text>\none\ntwo</s:read1_text>" in got
  assert "#bash4\nbash4: Act[Exit] = Act('bash4')" in got
  assert "#bash5\nbash5_command = 'echo it'\nbash5: Act[Exit] = Act('bash5')" in got
