"""bound_by, the names that a word of the chain bound."""

from pathlib import Path

from extensions.conftest import extended
from furb import engine


async def test_bound_by_gives_the_names_that_the_words_of_a_chain_bind_at_their_top_level_a_def_a_class_an_assignment_or_a_quote_each_with_the_rung_whose_word_bound_it_last_and_it_asks_nothing(
  tmp_path: Path,
) -> None:
  """bound_by gives the names that the words of a chain bind at their top level, a def, a class, an assignment or a quote, each with the rung whose word bound it last, and it asks nothing."""
  _, root = extended("ouroboros", tmp_path, lives=True)
  first = engine.rung("x = 1\ndef f() -> int:\n  return x", on=root)
  second = engine.rung("class K:\n  pass\nx = 2\n<s:q>\nhi\n</s:q>", on=root)
  await second
  before = len(engine.transcript(on=root))
  got = await engine.rung("close(bound_by())", on=root)
  assert isinstance(got, dict)
  assert {name: got[name] for name in ("f", "K", "x", "q")} == {"f": first, "K": second, "x": second, "q": second}
  assert {a[0] for a in engine.transcript(on=root)[before:] if engine.question(a)} == {"rung", "gate", "run"}


async def test_the_words_of_the_extensions_that_play_on_the_chain_count_each_under_the_rung_that_played_it_and_a_name_that_the_engine_defines_is_not_there_unless_a_word_binds_it_again(
  tmp_path: Path,
) -> None:
  """The words of the extensions that play on the chain count, each under the rung that played it, and a name that the engine defines is not there unless a word binds it again."""
  _, root = extended("ouroboros", tmp_path, lives=True)
  played = [by for by, text in engine.program(root).items() if "def extend(" in text]
  got = await engine.rung("close(bound_by())", on=root)
  assert isinstance(got, dict)
  assert played and got["extend"] == played[-1] and "read" not in got
  await engine.rung("read = 1", on=root)
  again = await engine.rung("close(bound_by())", on=root)
  assert isinstance(again, dict) and "read" in again
