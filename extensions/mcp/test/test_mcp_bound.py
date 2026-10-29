"""mcp_bound, the binding of a server."""

from pathlib import Path

from extensions.conftest import bindings, mcped, server
from furb import engine


async def test_mcp_bound_gives_the_binding_of_a_server_and_nothing_when_it_has_none(tmp_path: Path) -> None:
  """mcp_bound(server) gives the binding of a server, and an empty text when it has none."""
  root = mcped(tmp_path, {})[1]
  plain = server("plain", binding="x = 1\n")
  fresh = server("fresh", binding=bindings("fresh"))
  got = await engine.rung(f"close([mcp_bound(x) for x in {[server('none'), plain, fresh]!r}])", on=root)
  assert got == ["", "x = 1\n", bindings("fresh")]


async def test_a_class_name_that_the_module_binds_gets_a_suffix_unless_the_server_defined_it(tmp_path: Path) -> None:
  """A class name that the module of the chain binds already gets the suffix _2, then _3, in the class line of the binding, unless a word of the chain defined that class as the binding of this server."""
  first = server("read", binding=bindings("read"))
  second = server("reader", binding=bindings("read").replace('"read"', '"reader"'))
  root = mcped(tmp_path, {"mcp_servers": [first, second]})[1]
  renamed = bindings("read").replace("class read:", "class read_2:")
  # A value that a rung closes with is told, and a told paragraph is a word of the chain, so the words close with names.
  word = "await mcp_servers()\nclose([mcp_named(mcp_bound(mcp_known(x))) for x in ['read', 'reader']])"
  assert await engine.rung(word, on=root) == ["read_2", "read_2"]
  await engine.rung(renamed, on=root)
  assert await engine.rung(word, on=root) == ["read_2", "read_3"]
  rest = "close(mcp_bound(mcp_known('reader')).count('async def ping() -> McpResult:'))"
  assert await engine.rung(rest, on=root) == 1
