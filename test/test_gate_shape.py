"""Gate, the question of whether a word may run."""

from conftest import BAD, acts, bindings, born, findings, gated, gatings, heads, paragraphs, ran, said, settle
from furb import engine
from furb.engine import OPERATOR, Refused


async def test_a_gate_is_the_question_of_whether_a_word_may_run() -> None:
  """A gate is the question of whether a word may run, which the ear named gate answers with its findings, apart from the Kernel, so a word may ask it while the Kernel runs that word."""
  _, log, root = born()
  assert engine.gate("k = BAD", on=root) == [BAD]
  assert engine.get("gate1") == ("gate", "gate1", OPERATOR, root, "k = BAD")
  assert await engine.rung("close(gate('k = BAD'))", on=root) == [BAD]
  step = said(log, "rung")[0][1]
  assert engine.get("gate3") == ("gate", "gate3", step, root, "k = BAD")
  assert [a for a in said(log, "done") if a[1].startswith("gate")] == [
    ("done", "gate1", "gate", [BAD]),
    ("done", "gate2", "gate", []),
    ("done", "gate3", "gate", [BAD]),
  ]
  assert engine.gate("k = 9", on=root) == []


async def test_a_gate_carries_the_word_alone() -> None:
  """A gate carries the word alone, and the gate reads the program of the chain before that rung when it takes the gate, and the word after it, so the Kernel keeps no ladder of its own, the journal keeps no program, and a word of a program made again is read after the rungs that stand."""
  sand, log, root = born(
    "k = 1", "x = BAD", "y = 2", "write(read(get(acting())[2]).replace('y = 2', 'k = 3'))", "close(k)"
  )
  act = engine.prompt(int, "edit", on=root)
  assert await act == 3
  await settle()
  bind = bindings(root, act, "int", "edit")
  assert [len(e[0]) for e in sand.record if e[0][0] == "gate"] == [5] * 6
  bad = f"rung3_findings = {BAD!r}\nrung3_value = Refused()"
  wrote = (
    "read1_path = 'prompt1'\nread1_text = 'k = 1\\nx = BAD\\ny = 2'\nrung11_word = 'k = 3'\nrung9_value = Refused()"
  )
  assert gatings(log) == [
    ("k = 1", [bind]),
    ("x = BAD", [bind, "k = 1"]),
    ("y = 2", [bind, "k = 1", bad]),
    ("write(read(get(acting())[2]).replace('y = 2', 'k = 3'))", [bind, "k = 1", bad, "y = 2"]),
    ("k = 3", [bind, "k = 1", bad]),
    ("close(k)", [bind, "k = 1", bad, "k = 3", wrote]),
  ]


async def test_the_refused_paragraph_binds_the_findings_that_refused_the_word_of_a_rung() -> None:
  """The refused paragraph binds the findings that refused the word of a rung, one line for each, as rungN_findings."""
  _, log, root = born("k = BAD\nj = WORSE", "close(7)")
  assert await engine.prompt(int, "try", on=root) == 7
  worse = "line 2: error[unresolved-reference] Name `WORSE` used when not defined"
  assert findings(log) == [[BAD, worse], []]
  step = said(log, "rung")[0][1]
  refused = [one for one in paragraphs(engine.turns(on=root)) if one.split("\n", 1)[0].endswith(" refused")]
  assert refused == [f"#{step} refused\n<s:{step}_findings>\n{BAD}\n{worse}</s:{step}_findings>"]


async def test_the_chain_has_the_word_of_a_rung_gated_before_it_runs() -> None:
  """The chain has the word of a rung gated before it runs, but a word it wrote itself, and a refused word runs never."""
  _, log, root = born("k = BAD", "close(7)", "close(None)")
  act = engine.prompt(int, "try", on=root)
  assert await act == 7
  bind = bindings(root, act, "int", "try")
  again = f"{said(log, 'rung')[0][1]}_findings = {BAD!r}\n{said(log, 'rung')[0][1]}_value = Refused()"
  assert [a[4] for a in said(log, "rung") if a[2] == root] == [bind, again]
  assert gated(log) == ["k = BAD", "close(7)"] and findings(log) == [[BAD], []]
  assert ran(log) == [bind, again, "close(7)"]
  assert "k" not in engine.module(root)


async def test_a_refused_word_of_a_rung_stands_in_the_ladder_of_its_prompt() -> None:
  """A refused word of a rung stands in the ladder of its prompt, which its door shows, and it is no part of the program of the chain, which holds the words that run."""
  _, log, root = born("k = BAD", "close(7)", "close(None)")
  act = engine.prompt(int, "try", on=root)
  assert await act == 7
  assert engine.read(act, on=root).content == "k = BAD\nclose(7)"
  program = engine.program(root)
  again = f"rung1_findings = {BAD!r}\nrung1_value = Refused()"
  assert list(program.values()) == [bindings(root, act, "int", "try"), again, "close(7)"] == ran(log)


async def test_a_rung_that_retells_stands_with_the_gate_where_the_one_it_retells_stood() -> None:
  """A rung that retells stands with the gate where the one it retells stood, so the gate reads a word once in a life, and a copy of a refused word is refused again and tells its findings not again."""
  _, log, root = born("k = 1", "x = BAD", "close(k)", "close(None)")
  act = engine.prompt(int, "count", on=root)
  assert await act == 1
  await settle()
  engine.write(engine.read(act, on=root), on=root)
  await settle(300)
  bind = bindings(root, act, "int", "count")
  assert gated(log) == ["k = 1", "x = BAD", "close(k)"]
  assert [a[4] for a in acts(log).values() if a[0] == "gate"] == ["k = 1", "x = BAD", "close(k)"]
  first = [a[1] for a in said(log, "rung") if not a[5]]
  copies = [(a[1], a[4], a[5]) for a in said(log, "rung") if a[5]]
  bad = f"rung3_findings = {BAD!r}\nrung3_value = Refused()"
  assert [(word, donor) for _, word, donor in copies] == [
    (bind, first[1]),
    ("k = 1", first[0]),
    ("x = BAD", first[2]),
    (bad, first[4]),
    ("close(k)", first[3]),
  ]
  kinds = ["NoneType", "NoneType", "Refused", "NoneType", "NoneType"]
  assert [type(engine.peek(one)).__name__ for one, *_ in copies] == kinds
  assert [(a[1], a[3][0].split("\n")[0]) for a in said(log, "tell") if " refused" in a[3][0]] == [
    (first[2], f"#{first[2]} refused")
  ]
  assert engine.read(act, on=root).content == "k = 1\nx = BAD\nclose(k)"
  assert engine.program(root) == {
    copies[0][0]: bind,
    copies[1][0]: "k = 1",
    copies[3][0]: bad,
    copies[4][0]: "close(k)",
  }


async def test_the_chain_tells_the_findings_that_refused_a_word() -> None:
  """The chain tells the findings that refused a word, ends that rung with a refusal that holds none of them, and the prompt of it asks again as it does for a word that gave no value."""
  _, log, root = born("k = BAD", "close(1)")
  act = engine.prompt(int, "work", on=root)
  assert await act == 1
  await settle()
  first, _, again, _ = (a[1] for a in said(log, "rung"))
  assert [one for one in paragraphs(engine.turns(on=root)) if one.startswith(f"#{first} ")] == [
    f"#{first} advance on {act}",
    f"#{first} refused\n{first}_findings = {BAD!r}",
    f"#{first} closed\n{first}_value = Refused()",
  ]
  refusal = engine.peek(first)
  assert isinstance(refusal, Refused) and refusal.args == ()
  assert heads(engine.turns(on=root))[-3:] == [f"#{first} closed", f"#{again} advance on {act}", f"#{act} closed"]
  again = f"{first}_findings = {BAD!r}\n{first}_value = Refused()"
  assert ran(log) == [bindings(root, act, "int", "work"), again, "close(1)"]
