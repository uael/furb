"""Gate, the question of whether a word may run."""

from conftest import (
  BAD,
  STANDS,
  Sand,
  acts,
  bindings,
  findings,
  gated,
  gatings,
  heads,
  life,
  paragraphs,
  ran,
  said,
  settle,
  sown,
)
from furb import engine
from furb.engine import OPERATOR, Refused


async def test_a_gate_is_the_question_of_whether_a_word_may_run() -> None:
  """A gate is the question of whether a word may run, which the ear named gate answers with its findings, apart from the Kernel, so a word may ask it while the Kernel runs that word."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
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
  """A gate carries the word alone, and the gate reads the program of the chain before that rung when it takes the gate, and the word after it, so the Kernel keeps no ladder of its own, the record keeps no program, and a word of a program made again is read after the rungs that stand."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = [
    "k = 1",
    "x = BAD",
    "y = 2",
    "write(read(get(acting())[2]).replace('y = 2', 'k = 3'))",
    "close(k)",
  ]
  act = engine.prompt(int, "edit", on=root)
  assert await act == 3
  await settle()
  bind = bindings(root, act, "int")
  assert [len(e[0]) for e in sand.record if e[0][0] == "gate"] == [5] * 6
  assert gatings(log) == [
    ("k = 1", [bind]),
    ("x = BAD", [bind, "k = 1"]),
    ("y = 2", [bind, "k = 1"]),
    ("write(read(get(acting())[2]).replace('y = 2', 'k = 3'))", [bind, "k = 1", "y = 2"]),
    ("k = 3", [bind, "k = 1"]),
    ("close(k)", [bind, "k = 1", "k = 3"]),
  ]


async def test_the_refused_paragraph_holds_the_findings_that_refused_the_word_of_a_rung() -> None:
  """The refused paragraph holds the findings that refused the word of a rung, one comment for each."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["k = BAD\nj = WORSE", "close(7)"]
  assert await engine.prompt(int, "try", on=root) == 7
  worse = "line 2: error[unresolved-reference] Name `WORSE` used when not defined"
  assert findings(log) == [[BAD, worse], []]
  step = said(log, "rung")[0][1]
  refused = [one for one in paragraphs(engine.turns(on=root)) if one.split("\n", 1)[0].endswith(" refused")]
  assert refused == [f"#{step} refused\n# {BAD}\n# {worse}"]


async def test_the_chain_has_the_word_of_a_rung_gated_before_it_runs() -> None:
  """The chain has the word of a rung gated before it runs, but a word it wrote itself, and a refused word runs never."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["k = BAD", "close(7)", "close(None)"]
  act = engine.prompt(int, "try", on=root)
  assert await act == 7
  bind = bindings(root, act, "int")
  assert [a[4] for a in said(log, "rung") if a[2] == root] == [bind]
  assert gated(log) == ["k = BAD", "close(7)"] and findings(log) == [[BAD], []]
  assert ran(log) == [bind, "close(7)"]
  assert "k" not in engine.module(root)


async def test_a_refused_word_of_a_rung_stands_in_the_ladder_of_its_prompt() -> None:
  """A refused word of a rung stands in the ladder of its prompt, which its door shows, and it is no part of the program of the chain, which holds the words that run."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["k = BAD", "close(7)", "close(None)"]
  act = engine.prompt(int, "try", on=root)
  assert await act == 7
  assert engine.read(act, on=root).content == "k = BAD\nclose(7)"
  program = engine.program(root)
  assert list(program.values()) == [bindings(root, act, "int"), "close(7)"] == ran(log)


async def test_a_rung_that_retells_stands_with_the_gate_where_the_one_it_retells_stood() -> None:
  """A rung that retells stands with the gate where the one it retells stood, so the gate reads a word once in a life, and a copy of a refused word is refused again and tells its findings not again."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["k = 1", "x = BAD", "close(k)", "close(None)"]
  act = engine.prompt(int, "count", on=root)
  assert await act == 1
  await settle()
  engine.write(engine.read(act, on=root), on=root)
  await settle(300)
  bind = bindings(root, act, "int")
  assert gated(log) == ["k = 1", "x = BAD", "close(k)"]
  assert [a[4] for a in acts(log).values() if a[0] == "gate"] == ["k = 1", "x = BAD", "close(k)"]
  first = [a[1] for a in said(log, "rung") if not a[5]]
  copies = [(a[1], a[4], a[5]) for a in said(log, "rung") if a[5]]
  assert [(word, donor) for _, word, donor in copies] == [
    (bind, first[1]),
    ("k = 1", first[0]),
    ("x = BAD", first[2]),
    ("close(k)", first[3]),
  ]
  assert [type(engine.peek(one)).__name__ for one, *_ in copies] == ["NoneType", "NoneType", "Refused", "NoneType"]
  assert [(a[1], a[3][0]) for a in said(log, "tell") if a[3][0].endswith(" refused")] == [
    (first[2], f"#{first[2]} refused")
  ]
  assert engine.read(act, on=root).content == "k = 1\nx = BAD\nclose(k)"
  assert engine.program(root) == {copies[0][0]: bind, copies[1][0]: "k = 1", copies[3][0]: "close(k)"}


async def test_the_chain_tells_the_findings_that_refused_a_word() -> None:
  """The chain tells the findings that refused a word, ends that rung with a refusal that holds none of them, and the prompt of it asks again as it does for a word that gave no value."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["k = BAD", "close(1)"]
  act = engine.prompt(int, "work", on=root)
  assert await act == 1
  await settle()
  first, _, again = (a[1] for a in said(log, "rung"))
  assert [one for one in paragraphs(engine.turns(on=root)) if one.startswith(f"#{first} ")] == [
    f"#{first} advance on {act}",
    f"#{first} refused\n# {BAD}",
    f"#{first} closed Refused()",
  ]
  refusal = engine.peek(first)
  assert isinstance(refusal, Refused) and refusal.args == ()
  assert heads(engine.turns(on=root))[-3:] == [
    f"#{first} closed Refused()",
    f"#{again} advance on {act}",
    f"#{act} closed 1",
  ]
  assert ran(log) == [bindings(root, act, "int"), "close(1)"]
