"""mcp_type, the Python type that a schema asks."""

from pathlib import Path

from extensions.conftest import mcped
from furb import engine

TYPED = [
  {"type": "string"},
  {"type": "integer"},
  {"type": "number"},
  {"type": "boolean"},
  {"type": "null"},
  {"type": "array", "items": {"type": "integer"}},
  {"type": "object"},
  {"anyOf": [{"type": "string"}, {"type": "null"}, {"type": "string"}]},
  {"oneOf": [{"type": "integer"}, {"type": "boolean"}]},
  {"enum": ["a", 1]},
  {"const": "x"},
  {"type": ["string", "null"]},
]
"""TYPED is a schema of each kind that Python can write."""

UNTYPED = [{"type": "weird"}, {}, {"type": 3}, "text", None]
"""UNTYPED is a schema that asks no type that Python can write."""


async def test_mcp_type_is_the_python_type_that_a_schema_asks(tmp_path: Path) -> None:
  """mcp_type(schema) is the Python type that a JSON Schema asks, written as text: str, int, float, bool or None for a type of that name, list of its items for an array, dict[str, Any] for an object, a union for anyOf, oneOf or a list of types, and Literal of the values for an enum or a const."""
  root = mcped(tmp_path, {})[1]
  got = await engine.rung(f"close([mcp_type(x) for x in {TYPED!r}])", on=root)
  assert got == [
    "str",
    "int",
    "float",
    "bool",
    "None",
    "list[int]",
    "dict[str, Any]",
    "str | None",
    "int | bool",
    "Literal['a', 1]",
    "Literal['x']",
    "str | None",
  ]


async def test_a_schema_that_asks_no_type_that_python_can_write_is_any(tmp_path: Path) -> None:
  """A schema that asks no type that Python can write is Any."""
  root = mcped(tmp_path, {})[1]
  got = await engine.rung(f"close([mcp_type(x) for x in {UNTYPED!r}] + [mcp_type({{'type': 'array'}})])", on=root)
  assert got == ["Any", "Any", "Any", "Any", "Any", "list[Any]"]
