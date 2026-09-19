"""Plain, the form of a value on the wire."""

import json

from conftest import lived, plain, relived, sown, wire
from furb import engine

PLAIN = [None, True, 3, 1.5, "a", [1, "b"], {"k": 1}]
"""One value of each form that a value on the wire takes."""


async def test_plain_is_the_form_of_a_value_on_the_wire() -> None:
  """Plain is the form of a value on the wire: None, bool, int, float, str, a list of Plain, or a dict from str to Plain."""
  assert json.loads(json.dumps(PLAIN)) == PLAIN
  sand = sown()
  _, root = await lived(sand)
  wired = wire(sand.record)
  assert json.loads(json.dumps(wired)) == wired
  _, over = await relived(sown(), plain(sand.record))
  assert over == root and engine.modules[over]["__name__"] == root
