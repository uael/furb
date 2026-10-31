"""gate, whether the word of a rung may run."""

import pytest

from conftest import (
  BAD,
  QUOTES,
  Sand,
  born,
  findings,
  fresh,
  gated,
  gatings,
  paragraphs,
  prompted,
  ran,
  relived,
  said,
  settle,
  waiting,
  written,
  wrote,
)
from furb import engine
from furb.engine import Refused


async def test_whether_the_word_of_a_rung_may_run() -> None:
  """Whether the word of a rung may run: the gate reads it after the program of its chain, and it finds nothing when the word may run."""
  sand, log, root = born()
  laid = engine.rung("k = 1", on=root)
  await laid
  sand.script[root] = ["close(k + 1)"]
  act = engine.prompt(int, "count", on=root)
  assert await act == 2
  await settle()
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  told = [fresh(root, written(laid, "k = 1")), f"{prompted(act, 'int', 'count')}\n\n#{step} advance on {act}"]
  assert gatings(log) == [("k = 1", told[:1]), ("close(k + 1)", [told[0], "k = 1", told[1]])]
  assert findings(log) == [[], []]


async def test_the_word_of_a_rung_runs_only_if_the_gate_accepts_the_word_or_if_the_chain_wrote_it() -> None:
  """The word of a rung runs only if the gate accepts the word, or if the chain wrote it."""
  sand, log, root = born()
  bad = engine.rung("k = BAD", on=root)
  with pytest.raises(Refused):
    await bad
  assert findings(log) == [[BAD]]
  told = [fresh(root, written(bad, "k = BAD"))]
  assert ran(log) == told and "k" not in engine.module(root)
  laid = engine.rung("k = 1", on=root)
  assert await laid is None
  refused = f"#{bad} refused\n{bad}_findings = {BAD!r}"
  told.append(f"{refused}\n\n{written(laid, 'k = 1')}")
  assert ran(log) == [*told, "k = 1"] and engine.module(root)["k"] == 1
  sand.script[root] = ["close(k)"]
  act = engine.prompt(int, "count", on=root)
  assert await act == 1
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  told.append(f"{prompted(act, 'int', 'count')}\n\n#{step} advance on {act}")
  assert [a[5] for a in said(log, "run") if a[4].endswith("_told")] == told
  assert [word for word, _ in gatings(log)] == ["k = BAD", "k = 1", "close(k)"]
  assert ran(log) == [*told[:2], "k = 1", told[2], "close(k)"] and engine.module(root)[act] == act


async def test_the_gate_reads_the_word_after_the_parts_of_the_writings_of_a_rung_with_no_run() -> None:
  """The gate reads the word after the parts that the writings of a rung with no run hold too, so a rung that a word makes while its model writes it is read after what that word ran."""
  _, _, root, act, rung, _ = await waiting()
  defined = "def f() -> int:\n  return 1\n"
  made = "made = rung('y = f()')\nclose(2)\n"
  wrote(rung, defined, 0)
  wrote(rung, made, len(defined))
  await settle()
  assert await act == 2
  assert rung not in engine.program(root)
  assert engine.module(root)["y"] == 1


async def test_the_gate_checks_the_word_of_a_rung_against_the_rungs_before_it_in_record_order() -> None:
  """The gate checks the word of a rung against the rungs before it in record order."""
  _, log, root = born()
  first = engine.rung("a = 1", on=root)
  await first
  second = engine.rung("b = a + 1", on=root)
  await second
  early = engine.rung("c = later", on=root)
  with pytest.raises(Refused):
    await early
  later = engine.rung("later = 3", on=root)
  await later
  again = engine.rung("c = later", on=root)
  await again
  (finding,) = next(one for one in findings(log) if one)
  refused = f"#{early} refused\n{early}_findings = {finding!r}"
  told = [
    fresh(root, written(first, "a = 1")),
    written(second, "b = a + 1"),
    written(early, "c = later"),
    f"{refused}\n\n{written(later, 'later = 3')}",
    written(again, "c = later"),
  ]
  assert gatings(log) == [
    ("a = 1", told[:1]),
    ("b = a + 1", [told[0], "a = 1", told[1]]),
    ("c = later", [told[0], "a = 1", told[1], "b = a + 1", told[2]]),
    ("later = 3", [told[0], "a = 1", told[1], "b = a + 1", told[2], told[3]]),
    ("c = later", [told[0], "a = 1", told[1], "b = a + 1", told[2], told[3], "later = 3", told[4]]),
  ]
  assert [bool(found) for found in findings(log)] == [False, False, True, False, False]


async def test_a_response_that_is_not_python_is_a_finding_like_any_other() -> None:
  """A response that is not python is a finding like any other."""
  _, log, root = born("this is no python at all", "close(1)")
  act = engine.prompt(int, "try", on=root)
  assert await act == 1
  await settle()
  assert gated(log) == ["this is no python at all", "close(1)"]
  (found,), none = findings(log)
  assert found.startswith("line 1: ") and none == []
  step = said(log, "rung")[0][1]
  assert [one for one in paragraphs(engine.turns(on=root)) if one.startswith(f"#{step} ")] == [
    f"#{step} advance on {act}",
    f"#{step} refused\n{step}_findings = {found!r}",
  ]
  (_, last) = [a[1] for a in said(log, "rung") if a[2] == act]
  told = [
    fresh(root, prompted(act, "int", "try"), f"#{step} advance on {act}"),
    f"#{step} refused\n{step}_findings = {found!r}\n\n#{last} advance on {act}",
  ]
  assert ran(log) == [told[0], told[1], "close(1)"]


async def test_the_gate_gives_no_finding_when_the_gate_accepts_the_rung() -> None:
  """The gate gives no finding when the gate accepts the rung."""
  _, _, root = born()
  assert engine.gate("k = 1", on=root) == []
  assert engine.gate("close(1)", on=root) == []
  assert engine.gate("k = BAD", on=root) == [BAD]


async def test_each_finding_names_the_line_and_the_column_of_the_word_it_is_found_on_and_the_rule_and_quotes_that_line() -> (
  None
):
  """Each finding names the line and the column of the word it is found on, and the rule, and quotes that line, cut to 80 characters, so a model reads what the gate read."""
  _, _, root = born()
  long = "k = BAD + " + "1 + " * 30 + "1"
  why = "error[unresolved-reference] (column 5) Name `BAD` used when not defined"
  assert engine.gate("k = 1\nj = BAD  ", on=root) == [f"line 2: {why}, in 'j = BAD'"]
  (found,) = engine.gate("k = 1\n  j = 2", on=root)
  assert found.startswith("line 2: ") and found.endswith(", in 'j = 2'")
  assert engine.gate(long, on=root) == [f"line 1: {why}, in {long[:77] + '...'!r}"]
  for row in QUOTES:
    assert engine.gate(row["word"], on=root) == row["found"], row["word"]


async def test_a_word_that_holds_a_fence_markup_or_prose_finds_the_first_such_line_and_what_it_is() -> None:
  """A word that is not python and that holds a fence of markdown, the markup of a tool call or a sentence of prose finds the first such line and what it is, and that a reply is python and quotes only, with no fence, no markup and no prose, so nothing of this word ran: delete each such line."""
  _, _, root = born()
  stray = (
    "a reply is python and quotes only, with no fence, no markup and no prose, so nothing of this word ran: delete each"
    " such line"
  )
  for word, line, kind in [
    ("```python\nclose(1)\n```", "```python", "a fence"),
    ('<invoke name="bash">\nclose(1)', '<invoke name="bash">', "markup"),
    ("Let me read the file first.\nclose(1)", "Let me read the file first.", "prose"),
    ("close(1)\nNow I check the result.", "Now I check the result.", "prose"),
    ("I'll read the file first.\nclose(1)", "I'll read the file first.", "prose"),
    ("Let me read `x.py` and run it.\nclose(1)", "Let me read `x.py` and run it.", "prose"),
    ("Read the file \u2014 then close it.\nclose(1)", "Read the file \u2014 then close it.", "prose"),
    ("Use a well-known name -- and close it.\nclose(1)", "Use a well-known name -- and close it.", "prose"),
  ]:
    (found,) = engine.gate(word, on=root)
    at = word.split("\n").index(line) + 1
    assert found.startswith(f"line {at}: ") and found.endswith(f"; line {at} is {kind}: {stray}, in {line!r}")
  (found,) = engine.gate("Total - rest - more\nclose(", on=root)
  assert "is prose" not in found
  (found,) = engine.gate("= 1", on=root)
  assert stray not in found and found.endswith(", in '= 1'")


async def test_a_word_that_holds_a_mark_of_a_quote_inside_a_line_finds_the_first_such_mark() -> None:
  """A word that is not python and that holds a mark of a quote inside a line, as write(Text(path, <s:name>, finds the first such mark, and that a quote opens only with <s:name> at the start of a line and closes only with </s:name> at the end of a line, so nothing of this word ran: write the quote on lines of its own before the call, and give the call the name."""
  _, _, root = born()
  stray = "a reply is python and quotes only"
  for word, at, mark in [
    ('write(Text("a.py", <s:body>\nx = 1\n</s:body>))', 1, "<s:body>"),
    ("text = <s:body>\nx = 1</s:body>", 1, "<s:body>"),
    ("text = f(1\n</s:body>))", 2, "</s:body>"),
  ]:
    (found,) = engine.gate(word, on=root)
    assert stray not in found
    assert (
      f"; line {at}: the mark {mark} stands inside a line, and a quote opens only with <s:body> at the start" in found
    )
    assert found.rpartition(", in ")[0].endswith(
      " so nothing of this word ran: write the quote on lines of its own before the call, and give the call the name"
      " body"
    )
  (found,) = engine.gate('k = "a <s:body> b"\nclose(', on=root)
  assert "stands inside" not in found


async def test_a_word_that_is_not_python_on_a_line_that_starts_with_a_lone_equals_finds_that_its_binding_has_no_name() -> (
  None
):
  """A word that is not python on a line that starts with a lone = finds that the line starts with =, so its binding has no name: write the name before =, as in got = 1, and nothing of this word ran."""
  _, _, root = born()
  nameless = (
    "the line starts with =, so its binding has no name: write the name before =, as in got = 1, and nothing of this"
    " word ran"
  )
  for word, line in [("= 1", "= 1"), ("k = 1\n= await bash('ls')", "= await bash('ls')")]:
    (found,) = engine.gate(word, on=root)
    assert found.endswith(f"; {nameless}, in {line!r}")
  (found,) = engine.gate("== 1", on=root)
  assert nameless not in found and found.endswith(", in '== 1'")


async def test_an_open_mark_of_a_quote_that_has_no_close_mark_is_a_finding_of_its_own() -> None:
  """An open mark of a quote that has no close mark is a finding of its own, which names that mark and the close mark it wants."""
  _, _, root = born()
  assert engine.gate("k = 1\n<s:plan>\nstep one\nclose(plan)", on=root) == [
    "line 2: the quote <s:plan> has no close mark </s:plan> at the end of a line, in '<s:plan>'"
  ]
  assert engine.gate("<s:plan>\nstep one</s:plan>\nclose(plan)", on=root) == []
  mark = "the quote <s:edit> has no close mark </s:edit> at the end of a line, in '<s:edit>'"
  for word, line in [
    ("<s:a>\nt\n</s:a>\nk = 1\n<s:edit>\nx = 1\nclose(1)", 5),
    ("<s:edit>\nt\n<s:old>\no\n</s:old>\nclose(1)", 1),
    ("<s:edit>\nt </s:edit> more\nclose(1)", 1),
    ("<s:edit>\nt\n</s:old>\nclose(1)", 1),
    ("if 1:\n  <s:edit>\n  x\nclose(1)", 2),
  ]:
    assert engine.gate(word, on=root) == [f"line {line}: {mark}"]


async def test_an_open_mark_that_stands_inside_a_quote_of_its_own_name_is_a_finding_of_its_own() -> None:
  """An open mark that stands inside a quote of its own name is a finding of its own, which names that mark and the line of the quote that holds it, since one close mark ends both."""
  _, _, root = born()
  why = (
    "the open mark <s:edit> stands inside the quote <s:edit> of line {}, and one close mark </s:edit> ends both:"
    " close the first quote, or delete this mark"
  )
  assert engine.gate("k = 1\n<s:edit>\nA\n<s:edit>\nB\n</s:edit>\nclose(k)", on=root) == [f"line 4: {why.format(2)}"]
  assert engine.gate("<s:edit>\nA\n<s:old>\nB\n</s:old>\n</s:edit>\nclose(1)", on=root) == []
  assert engine.gate("<s:a>\nA\n</s:a>\n<s:a>\nB\n</s:a>\nclose(a)", on=root) == []


async def test_a_word_that_is_one_quote_and_nothing_else_finds_that_it_only_binds_the_name_of_the_quote() -> None:
  """A word that is one quote and nothing else finds that it only binds the name of the quote and nothing of it runs, so write the code as the word itself, with no quote around it, and quote only a text that the code uses."""
  _, _, root = born()
  why = (
    "the word is the quote <s:rung9_word> and nothing else, so it only binds rung9_word and nothing of it runs: write"
    " the code as the word itself, with no quote around it, and quote only a text that the code uses"
  )
  found = engine.gate("<s:rung9_word>\nwrite(text)\n</s:rung9_word>\n", on=root)
  assert found == [f"line 1: {why}, in \"rung9_word = 'write(text)\\\\n'\""]
  assert engine.gate("<s:plan>\nstep one\n</s:plan>\nclose(plan)", on=root) == []
  assert engine.gate("k = 'a'", on=root) == []


async def test_the_journal_keeps_what_the_gate_found() -> None:
  """The journal keeps what the gate found, since the gate is of the outside, so a later life reads the same findings and asks the gate nothing again."""
  sand, log, root = born("close(1)")
  assert await engine.prompt(int, "count", on=root) == 1
  await settle()
  assert gated(log) == ["close(1)"] and findings(log) == [[]]
  assert [fact[1] for fact, *_ in sand.record if fact[0] == "gate"] == ["gate1"]
  again, over = await relived(Sand(), list(sand.record))
  assert over == root and gated(again) == ["close(1)"] and findings(again) == [[]]
  assert [a[2] for a in said(again, "done") if a[1] == "gate1"] == ["journal"]


async def test_a_word_that_binds_a_name_that_the_transcript_binds_finds_that_the_transcript_binds_it() -> None:
  """A word that binds a name that the transcript binds, the name of an act of its chain or a name of a value that a tell of that act binds, as bash12_stdout, finds that the transcript binds that name, also for an act that the chain did not tell yet."""
  _, _, root = born()
  act = engine.rung("k = 1", on=root)
  told = "error[transcript-name] (column {}) the transcript binds `{}`: a word reads it and never binds it"
  word = f"{act}_value = 2"
  assert engine.gate(word, on=root) == [f"line 1: {told.format(1, act + '_value')}, in {word!r}"]
  await act
  assert engine.gate(f"k = 2\nfor {act} in []:\n  pass", on=root) == [
    f"line 2: {told.format(5, act)}, in 'for {act} in []:'"
  ]
  assert engine.gate(f"{act}x = 1\nbash99_stdout = ''", on=root) == []


async def test_a_word_that_binds_a_name_that_a_rung_before_it_declares_finds_the_rung_that_declares_it() -> None:
  """A word that binds a name to a value of another type than the one that a rung before it declares for that name, as a def, a class, an import or a name with a type does, finds the rung that declares that name, so the model gives the value another name."""
  _, _, root = born()
  act = engine.rung("def pr() -> int:\n  return 1", on=root)
  await act
  word = "pr = 'a'"
  assert engine.gate(word, on=root) == [
    'line 1: error[invalid-assignment] (column 6) Object of type `Literal["a"]` is not assignable to `def pr() -> int`;'
    f" the word of {act} declares `pr` with that type, so give this value another name, in {word!r}"
  ]
  assert engine.gate("pr: str = 'a'\nclose(pr)", on=root) == []


async def test_a_word_that_leaves_a_bracket_open_finds_the_line_and_the_column_of_the_innermost_bracket_that_stays_open() -> (
  None
):
  """A word that is not python and that leaves a bracket open finds the line and the column of the innermost bracket that stays open."""
  _, _, root = born()
  (found,) = engine.gate("close((503, 'done')", on=root)
  assert "; the bracket ( of line 1, column 6, stays open," in found
  (found,) = engine.gate("k = 1\nj = foo(1,\n  [2, 3\nclose(j)", on=root)
  assert "; the bracket [ of line 3, column 3, stays open," in found
  (found,) = engine.gate("k = ')'\n# (\nclose((1, 2)", on=root)
  assert "; the bracket ( of line 3, column 6, stays open," in found
  (found,) = engine.gate('k = \'a\\\' (\'\nj = """b\\\n("""\nclose((1, 2)', on=root)
  assert "; the bracket ( of line 4, column 6, stays open," in found


async def test_a_word_that_is_not_python_and_that_holds_a_string_of_one_quote_that_a_newline_ends_finds_that_a_string_of_one_quote_ends_at_the_end_of_its_line_so_write_a_text_of_several_lines_as_a_triple_quote_or_as_a_quote_s_name_on_lines_of_its_own_before_the_call_and_nothing_of_this_word_ran() -> (
  None
):
  """A word that is not python and that holds a string of one quote that a newline ends finds that a string of one quote ends at the end of its line, so write a text of several lines as a triple quote, or as a quote <s:name> on lines of its own before the call, and nothing of this word ran."""
  _, _, root = born()
  (found,) = engine.gate('k = 1\nclose(bash("ls\n-la"))', on=root)
  fix = (
    "a string of one quote ends at the end of its line, so write a text of several lines as a triple quote, or as a"
    " quote <s:name> on lines of its own before the call, and nothing of this word ran"
  )
  assert found.startswith("line 2: error[invalid-syntax] ")
  assert f"; {fix}, in " in found


async def test_a_word_that_starts_with_a_shell_command_finds_that_the_line_is_a_shell_command() -> None:
  """A word that is not python and that starts with a shell command finds that the line is a shell command, so write it as await bash("..."), and nothing of this word ran."""
  _, _, root = born()
  (found,) = engine.gate("git status --short", on=root)
  assert found.endswith(
    "; the line is a shell command, so write it as await bash(\"...\"), and nothing of this word ran, in 'git status --short'"
  )
  (found,) = engine.gate("close(1)\ngh pr view 3", on=root)
  assert "the line is a shell command" in found and found.startswith("line 2: ")


async def test_a_finding_on_a_call_of_a_function_names_the_signature_of_that_function() -> None:
  """A finding on a call of a function names the signature of that function, so a model reads how to call it."""
  _, _, root = born()
  (found,) = engine.gate("close(read())", on=root)
  assert '; the signature of `read` is `read(path: str, show: Show = HEAD, on="") -> Text`, in' in found
  await engine.rung("def twice(n: int, *, by: int = 2) -> int:\n  return n * by", on=root)
  (found,) = engine.gate("close(twice(1, 2))", on=root)
  assert "; the signature of `twice` is `twice(n: int, *, by: int = 2) -> int`, in 'close(twice(1, 2))'" in found


async def test_a_finding_on_an_await_of_a_value_that_is_no_act_says_that_the_value_stands_in_its_name_already() -> None:
  """A finding on an await of a value that is no act says that a value which an await gave stands in its name already, so use the name and await it no more."""
  _, _, root = born()
  (found,) = engine.gate("got = await bash('ls')\nawait got", on=root)
  assert found.startswith("line 2: error[invalid-await] (column 7) ")
  assert (
    "; a value that an await gave stands in the name already, so use the name and await it no more, in 'await got'"
    in found
  )


async def test_a_finding_on_a_text_that_a_word_uses_as_a_str_says_that_a_text_is_no_str() -> None:
  """A finding on a Text that a word uses as a str says that a Text is no str, so use its .content for its text and its .lines for its lines."""
  _, _, root = born()
  for word in ("t = read('a')\nx = t + 'b'", "t = read('a')\nx = t.split()", "x = read('a')\nfor line in x: pass"):
    (found,) = engine.gate(word, on=root)
    assert "; a Text is no str, so use its .content for its text and its .lines for its lines, in " in found


async def test_a_finding_on_a_str_that_a_word_asks_for_the_content_of_a_text_says_that_a_name_of_a_tell_holds_a_str() -> (
  None
):
  """A finding on a str that a word asks for the .content or the .lines of a Text says that a name which a tell binds, as read1_text, holds a str and no Text, so use read(path) to get a Text, whose .content is its text."""
  _, _, root = born()
  for name in ("content", "lines"):
    (found,) = engine.gate(f"x = 'abc'\ny = x.{name}", on=root)
    assert (
      "; a name that a tell binds, as read1_text, holds a str and no Text, so use read(path) to get a Text, whose .content is its text, in "
      in found
    )
