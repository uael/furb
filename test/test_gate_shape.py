"""Gate, the question of whether a word may run."""

from conftest import STANDS, Sand, attr, gated, life, ran, refusals, said, settle, sown, tags
from furb import engine


async def test_a_gate_is_the_question_of_whether_a_word_may_run() -> None:
  """A gate is the question of whether a word may run, which the Kernel answers with its findings."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  found = ["BAD in rung, after 0 rungs"]
  assert engine.gate("k = BAD", on=root) == found
  answered = [one for one in said(log, "done") if one[1].startswith("gate://")]
  assert [(one[2], one[3]) for one in answered] == [("kernel", found)]
  assert engine.gate("k = 9", on=root) == []


async def test_the_refused_tag_holds_as_its_body_the_findings_that_refused_the_word_of_a_rung() -> None:
  """The refused tag holds as its body the findings that refused the word of a rung."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["k = BAD", "close(7)"]
  assert await engine.prompt(int, "try", on=root) == 7
  assert [tag[2] for tag in tags(engine.turns(on=root), "refused")] == refusals(log) != []


async def test_the_chain_has_the_kernel_gate_the_word_of_a_rung_before_it_runs() -> None:
  """The chain has the Kernel gate the word of a rung before it runs, and a refused word runs never."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["k = BAD", "close(7)", "close(None)"]
  assert await engine.prompt(int, "try", on=root) == 7
  assert gated(log) == ["k = BAD", "close(7)"] and ran(log) == ["close(7)"]
  assert "k" not in engine.modules[root]


async def test_a_refused_word_of_a_rung_stands_in_the_ladder_of_its_prompt() -> None:
  """A refused word of a rung stands in the ladder of its prompt, which its door shows, and it is no part of the program of the chain, which holds the words that run."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["k = BAD", "close(7)", "close(None)"]
  act = engine.prompt(int, "try", on=root)
  assert await act == 7
  assert engine.read(act, on=root).content == "k = BAD\nclose(7)"
  _, program = engine.ask("program", root)
  assert isinstance(program, dict)
  assert list(program.values()) == ["close(7)"] == ran(log)


async def test_a_gate_says_the_program_of_the_chain_before_that_rung() -> None:
  """A gate says the program of the chain before that rung, whose words the Kernel reads the word after, so the Kernel keeps no ladder of its own and a word of a program made again is read after the rungs that stand."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = [
    "k = 1",
    "x = BAD",
    "y = 2",
    "write(read(get(acting())[2]).replace('y = 2', 'k = 3'))",
    "close(k)",
  ]
  assert await engine.prompt(int, "edit", on=root) == 3
  await settle()
  asked = []
  for one in engine.asked.values():
    match one:
      case ("gate", _, _, _, word, dict(program)):
        asked.append((word, list(program.values())))
  assert asked == [
    ("k = 1", []),
    ("x = BAD", ["k = 1"]),
    ("y = 2", ["k = 1"]),
    ("write(read(get(acting())[2]).replace('y = 2', 'k = 3'))", ["k = 1", "y = 2"]),
    ("k = 1", []),
    ("x = BAD", ["k = 1"]),
    ("k = 3", ["k = 1"]),
    ("close(k)", ["k = 1", "k = 3"]),
  ]


async def test_the_chain_tells_the_findings_that_refused_a_word() -> None:
  """The chain tells the findings that refused a word, ends that rung with a refusal that holds none of them, and the prompt of it asks again as it does for a word that gave no value."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["k = BAD", "close(1)"]
  assert await engine.prompt(int, "work", on=root) == 1
  await settle()
  refused = tags(engine.turns(on=root), "refused")
  assert [tag[2] for tag in refused] == refusals(log) != []
  shut = {value: tag[2] for tag in tags(engine.turns(on=root), "closed") for key, value in tag[1] if key == "over"}
  assert shut[attr(refused[0], "id")] == "Refused()"
  assert ran(log) == ["close(1)"]
