"""Program, the question of the accepted words of a ladder."""

from conftest import WORD, Sand, lived, said, settle, sown
from furb import engine
from furb.engine import OPERATOR


async def test_a_program_is_the_question_of_the_accepted_words_of_a_ladder() -> None:
  """A program is the question of the words of the rungs of a chain, each with the name of its rung, which the chain answers, and which a chain with a source asks of its origin."""
  sand: Sand = sown()
  log, root = await lived(sand)
  steps = [one[1] for one in said(log, "rung")]
  word, got = engine.ask("program", root, root)
  match word:
    case ("program", _, by, on, about):
      asked = (by, on, about)
    case _:
      asked = None
  assert asked == (OPERATOR, root, root)
  assert got == {steps[0]: WORD, steps[1]: "close(None)"}
  twin = engine.chain("twin", source=root)
  await settle()
  theirs = []
  for one in engine.asked.values():
    match one:
      case ("program", _, by, on, about) if by == twin:
        theirs.append((on, about))
  assert theirs == [(root, root)]
