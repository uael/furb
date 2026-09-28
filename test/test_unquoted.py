"""unquoted, what a word is as python."""

from conftest import born, ran, said
from furb import engine


async def test_what_a_word_is_as_python() -> None:
  """What a word is as python: each quote in it bound as a string."""
  assert engine.unquoted("<s:hi>\nhello\n</s:hi>\nx = hi") == "hi = 'hello\\n'\n\n\nx = hi"
  assert engine.unquoted("x = 1") == "x = 1"
  _, _, root = born()
  assert await engine.rung("<s:hi>\nhello\n</s:hi>\nx = hi", on=root) is None
  assert engine.module(root)["x"] == "hello\n"


async def test_a_quote_is_a_string_a_word_writes_between_two_marks() -> None:
  """A quote is a string a word writes between two marks, <s:name> at the start of a line and </s:name> at the end of a line, name being any python name, so nothing in it needs an escape."""
  text = "it's \"quoted\" \\ and 'more'"
  assert engine.unquoted(f"<s:said>{text}</s:said>") == f"said = {text!r}"
  assert engine.unquoted(" <s:a>x</s:a>") == " <s:a>x</s:a>"
  assert engine.unquoted("<s:a>y</s:b>") == "<s:a>y</s:b>"
  assert engine.unquoted("<S1>y</S1>") == "<S1>y</S1>"
  _, _, root = born()
  assert await engine.rung(f"<s:said>\n{text}\n</s:said>\nk = said", on=root) is None
  assert engine.module(root)["k"] == f"{text}\n"


async def test_the_open_mark_is_looked_for_from_the_top_and_its_close_mark_from_the_open_mark_down() -> None:
  """The open mark is looked for line by line from the top, and its close mark from the open mark down, so a quote may hold the marks of a quote of another name."""
  word = "<s:a>\n<s:b>\nin\n</s:b>\n</s:a>\n<s:b>two</s:b>"
  assert engine.unquoted(word) == "a = '<s:b>\\nin\\n</s:b>\\n'\n\n\n\n\nb = 'two'"
  assert engine.unquoted("<s:a>\na</s:a>\nb</s:a>") == "a = 'a'\n\nb</s:a>"
  assert engine.unquoted("<s:a>\nx</s:a>\n<s:a>\ny</s:a>") == "a = 'x'\n\na = 'y'\n"


async def test_the_value_of_a_quote_is_the_text_between_its_marks() -> None:
  """The value of a quote is the text between its marks, less a line break just after the open mark."""
  assert engine.unquoted("<s:a>\nx\n</s:a>") == "a = 'x\\n'\n\n"
  assert engine.unquoted("<s:a>x</s:a>") == "a = 'x'"
  assert engine.unquoted("<s:a>\n\nx</s:a>") == "a = '\\nx'\n\n"
  assert engine.unquoted("<s:a></s:a>") == "a = ''"


async def test_a_quote_becomes_its_name_bound_to_its_value_on_the_line_of_the_open_mark() -> None:
  """A quote becomes its name bound to its value as python writes it, on the line of the open mark, and each other line of the quote becomes an empty line, so every line after it keeps its number."""
  word = "a = 1\n<s:two>\none\ntwo\n</s:two>\nb = nowhere"
  assert engine.unquoted(word).split("\n") == ["a = 1", "two = 'one\\ntwo\\n'", "", "", "", "b = nowhere"]
  _, _, root = born()
  assert [one.split(":")[0] for one in engine.gate(word, on=root)] == ["line 6"]


async def test_an_open_mark_that_has_no_close_mark_stays_as_it_is() -> None:
  """An open mark that has no close mark stays as it is, and the gate reads it as the python it is not."""
  word = "<s:hi>\nhello"
  assert engine.unquoted(word) == word
  _, _, root = born()
  found = engine.gate(word, on=root)
  assert len(found) == 1 and found[0].startswith("line 1: ")


async def test_the_engine_unquotes_a_word_before_the_gate_reads_it_and_before_the_kernel_runs_it() -> None:
  """The engine unquotes a word before the gate reads it and before the Kernel runs it, and the door of a ladder and the turns keep the quotes as the word wrote them."""
  sand, log, root = born()
  word = "<s:hi>\nhi\n</s:hi>\nclose(len(hi))"
  sand.script[root] = [word]
  asking = engine.prompt(int, "count", on=root)
  assert await asking == 3
  assert ran(log)[-1] == "hi = 'hi\\n'\n\n\nclose(len(hi))"
  assert [a[3] for a in said(log, "ready") if a[1] == "rung1"] == [word]
  assert engine.read(asking, on=root).content == word
  assert engine.turns(on=root)[1][1] == word
