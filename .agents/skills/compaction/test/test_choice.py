"""choice, the model of the chain that chooses what a compaction keeps."""

from pathlib import Path

from conftest import until
from furb import engine
from skills.conftest import compacting


def asked(chain: str) -> tuple:
  """The prompt of choice on a chain."""
  (one,) = [a for a in engine.transcript(chain) if a[0] == "prompt" and a[4] == "tuple"]
  return one


def rungs(chain: str, prompt: str) -> set[str]:
  """The actors of the rungs that a prompt made on a chain."""
  return {str(a[6]) for a in engine.transcript(chain) if a[0] == "rung" and a[2] == prompt}


async def test_choice_asks_the_model_that_wrote_the_last_reply_of_the_chain(tmp_path: Path) -> None:
  """choice() asks the model that wrote the last reply of the chain, with its effort, on the chain itself, which acts of the chain the work still needs, in a prompt of the shape tuple, the ids to keep as list[str] and a summary of what the rest holds, and gives that prompt."""
  sand, root, _ = compacting(tmp_path)
  sand.script[root] = ["got = choice()\nclose('asked')", "close((['x'], 'gist'))"]
  assert await engine.prompt(str, "go", to="n/low", on=root) == "asked"
  got = engine.module(root)["got"]
  await until(lambda: engine.over(got))
  assert engine.peek(got) == (["x"], "gist")
  choice = asked(root)
  assert (choice[1], choice[3], choice[6]) == (got, root, "n/low") and engine.module(root)["actor"] == "m/low"
  assert rungs(root, got) == {"n/low"}
  assert [a for a in engine.transcript(root) if a[0] == "chain"] == []


async def test_when_no_model_wrote_a_reply_choice_asks_the_actor_of_the_module(tmp_path: Path) -> None:
  """When no model wrote a reply of the chain, choice() asks the actor of the module of the chain."""
  sand, root, _ = compacting(tmp_path)
  sand.script[root] = ["close((['x'], 'gist'))"]
  await engine.rung("got = choice()", on=root)
  got = engine.module(root)["got"]
  await until(lambda: engine.over(got))
  assert engine.peek(got) == (["x"], "gist")
  assert asked(root)[6] == "m/low" and rungs(root, got) == {"m/low"}
