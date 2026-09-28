"""doctrine, the quote after the engine in the system prompt."""

from conftest import born
from furb import engine
from furb_monty._monty import SYSTEM


async def test_doctrine_is_the_quote_after_the_engine_in_the_system_prompt() -> None:
  """doctrine is the quote after the engine in the system prompt, which says how a model works in furb and holds no law."""
  assert SYSTEM.endswith(f"\n<s:doctrine>\n{engine.doctrine}</s:doctrine>")
  _, _, root = born()
  assert engine.module(root)["doctrine"] == engine.doctrine
