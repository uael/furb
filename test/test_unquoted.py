"""unquoted, what a word is as python."""

from conftest import Sand, life, ran, said
from furb import engine


async def test_what_a_word_is_as_python() -> None:
  """What a word is as python: each quote in it bound as a string."""
  assert engine.unquoted("<S1>\nhello\n</S1>\nx = S1") == "S1 = 'hello\\n'\n\n\nx = S1"
  assert engine.unquoted("x = 1") == "x = 1"
  sand = Sand()
  _, root = life(sand)
  assert await engine.rung("<S1>\nhello\n</S1>\nx = S1", on=root) is None
  assert engine.module(root)["x"] == "hello\n"


async def test_a_quote_is_a_string_a_word_writes_between_two_marks() -> None:
  """A quote is a string a word writes between two marks, <Sn> at the start of a line and </Sn> at the end of a line, n being any number, so nothing in it needs an escape."""
  text = "it's \"quoted\" \\ and 'more'"
  assert engine.unquoted(f"<S12>{text}</S12>") == f"S12 = {text!r}"
  assert engine.unquoted(" <S1>x</S1>") == " <S1>x</S1>"
  assert engine.unquoted("<Sx>y</Sx>") == "<Sx>y</Sx>"
  sand = Sand()
  _, root = life(sand)
  assert await engine.rung(f"<S12>\n{text}\n</S12>\nk = S12", on=root) is None
  assert engine.module(root)["k"] == f"{text}\n"


async def test_the_open_mark_is_looked_for_from_the_top_and_its_close_mark_from_the_last_line_up() -> None:
  """The open mark is looked for line by line from the top, and its close mark from the last line up, so a quote may hold the marks of another."""
  word = "<S1>\n<S2>\nin\n</S2>\n</S1>\n<S2>two</S2>"
  assert engine.unquoted(word) == "S1 = '<S2>\\nin\\n</S2>\\n'\n\n\n\n\nS2 = 'two'"
  assert engine.unquoted("<S1>\na</S1>\nb</S1>") == "S1 = 'a</S1>\\nb'\n\n"


async def test_the_value_of_a_quote_is_the_text_between_its_marks() -> None:
  """The value of a quote is the text between its marks, less a line break just after the open mark."""
  assert engine.unquoted("<S1>\nx\n</S1>") == "S1 = 'x\\n'\n\n"
  assert engine.unquoted("<S1>x</S1>") == "S1 = 'x'"
  assert engine.unquoted("<S1>\n\nx</S1>") == "S1 = '\\nx'\n\n"
  assert engine.unquoted("<S1></S1>") == "S1 = ''"


async def test_a_quote_becomes_sn_bound_to_its_value_on_the_line_of_the_open_mark() -> None:
  """A quote becomes Sn bound to its value as python writes it, on the line of the open mark, and each other line of the quote becomes an empty line, so every line after it keeps its number."""
  word = "a = 1\n<S1>\none\ntwo\n</S1>\nb = nowhere"
  assert engine.unquoted(word).split("\n") == ["a = 1", "S1 = 'one\\ntwo\\n'", "", "", "", "b = nowhere"]
  sand = Sand()
  _, root = life(sand)
  assert [one.split(":")[0] for one in engine.gate(word, on=root)] == ["line 6"]


async def test_an_open_mark_that_has_no_close_mark_stays_as_it_is() -> None:
  """An open mark that has no close mark stays as it is, and the gate reads it as the python it is not."""
  word = "<S1>\nhello"
  assert engine.unquoted(word) == word
  sand = Sand()
  _, root = life(sand)
  found = engine.gate(word, on=root)
  assert len(found) == 1 and found[0].startswith("line 1: ")


async def test_the_engine_unquotes_a_word_before_the_gate_reads_it_and_before_the_kernel_runs_it() -> None:
  """The engine unquotes a word before the gate reads it and before the Kernel runs it, and the door of a ladder and the turns keep the quotes as the word wrote them."""
  sand = Sand()
  log, root = life(sand)
  word = "<S1>\nhi\n</S1>\nclose(len(S1))"
  sand.script[root] = [word]
  asking = engine.prompt(int, "count", on=root)
  assert await asking == 3
  assert ran(log)[-1] == "S1 = 'hi\\n'\n\n\nclose(len(S1))"
  assert [a[3] for a in said(log, "ready") if a[1] == "rung1"] == [word]
  assert engine.read(asking, on=root).content == word
  assert engine.turns(on=root)[1][1] == word
