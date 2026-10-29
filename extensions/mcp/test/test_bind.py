"""BIND, the most characters of a binding that the index binds at once."""

from pathlib import Path

from extensions.conftest import mcped
from furb import engine


async def test_bind_is_the_most_characters_of_a_binding_that_the_index_binds_at_once(tmp_path: Path) -> None:
  """BIND is the most characters that the binding of a server may hold for mcp_index to bind its class at once, and it is 3000 unless a word rebinds it."""
  root = mcped(tmp_path, {})[1]
  assert await engine.rung("close(BIND)", on=root) == 3000
