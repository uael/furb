"""prompted, whether a name is the name of a prompt."""

from conftest import born
from furb import engine


async def test_whether_a_name_is_the_name_of_a_prompt_which_its_name_says() -> None:
  """Whether a name is the name of a prompt, which its name says, as prompt3, and not prompt, chain3 or an empty name."""
  _, _, root = born("close(1)")
  act = engine.prompt(int, "one", on=root)
  assert await act == 1
  assert engine.prompted(act) and engine.prompted("prompt3")
  assert not any(engine.prompted(one) for one in ("prompt", "chain3", "", root, "aprompt3"))
