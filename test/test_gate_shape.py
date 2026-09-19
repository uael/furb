"""Gate, the question of whether a word may run."""

from conftest import STANDS, Py, Sand, attr, life, said, settle, sown, tags
from furb import engine


async def test_a_gate_is_the_question_of_whether_a_word_may_run() -> None:
  """A gate is the question of whether a word may run, which the Kernel answers with its findings."""
  sand, py = Sand(stands=STANDS), Py()
  log, root = life(sand, kernel=py)
  found = ["BAD in rung, against int after 0 rungs"]
  assert engine.gate("BAD = 1", "int", on=root) == found
  assert py.gates == [("BAD = 1", [], "int")]
  answered = [one for one in said(log, "done") if one[1].startswith("gate://")]
  assert [(one[2], one[3]) for one in answered] == [("kernel", found)]
  assert engine.gate("k = 9", on=root) == []


async def test_the_refused_tag_holds_as_its_body_the_findings_that_refused_the_word_of_a_rung() -> None:
  """The refused tag holds as its body the findings that refused the word of a rung."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["BAD = 1", "close(7)"]
  assert await engine.prompt(int, "try", on=root) == 7
  assert [tag[2] for tag in tags(engine.turns(on=root), "refused")] == ["BAD in rung, against int after 0 rungs"]


async def test_the_chain_has_the_kernel_gate_the_word_of_a_rung_before_it_runs() -> None:
  """The chain has the Kernel gate the word of a rung before it runs, and a refused word runs never."""
  sand, py = sown(), Py()
  _, root = life(sand, kernel=py)
  sand.script[root] = ["BAD = 1", "close(7)", "close(None)"]
  assert await engine.prompt(int, "try", on=root) == 7
  assert py.gated == ["BAD = 1", "close(7)"] and py.ran == ["close(7)"]
  assert "BAD" not in engine.modules[root]


async def test_a_refused_word_of_a_rung_is_not_part_of_the_program_of_the_chain() -> None:
  """A refused word of a rung is not part of the program of the chain."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["BAD = 1", "close(7)", "close(None)"]
  assert await engine.prompt(int, "try", on=root) == 7
  assert engine.read(root, on=root).content == "close(7)"
  _, program = engine.ask("program", root, root)
  assert isinstance(program, list)
  assert [word for _, word in program] == ["close(7)"]


async def test_the_chain_tells_the_findings_that_refused_a_word() -> None:
  """The chain tells the findings that refused a word, ends that rung with a refusal that holds none of them, and the prompt of it asks again as it does for a word that gave no value."""
  sand, py = sown(), Py()
  _, root = life(sand, kernel=py)
  sand.script[root] = ["BAD = 1", "close(1)"]
  assert await engine.prompt(int, "work", on=root) == 1
  await settle()
  refused = tags(engine.turns(on=root), "refused")
  assert [tag[2] for tag in refused] == ["BAD in rung, against int after 0 rungs"]
  shut = {value: tag[2] for tag in tags(engine.turns(on=root), "closed") for key, value in tag[1] if key == "over"}
  assert shut[attr(refused[0], "id")] == "Refused()"
  assert py.ran == ["close(1)"]
