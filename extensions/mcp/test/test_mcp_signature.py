"""mcp_signature, one line for a tool."""

from pathlib import Path

from extensions.conftest import mcped, tooled
from furb import engine

CREATE = tooled(
  "create_issue",
  "Create an issue. It needs a title.",
  {"type": "object", "properties": {"title": {"type": "string"}}, "required": ["title"]},
)
"""CREATE is a tool with one required argument and a description of two sentences."""


async def test_mcp_signature_is_one_line_for_a_tool(tmp_path: Path) -> None:
  """mcp_signature(tool) is one line for a tool: its name, its arguments in parentheses, and after a colon the first sentence of its description, cut at 200 characters."""
  root = mcped(tmp_path, {})[1]
  tools = [CREATE, tooled("ping"), tooled("long", "a" * 250)]
  got = await engine.rung(f"close([mcp_signature(x) for x in {tools!r}])", on=root)
  assert got == ["create_issue(title: str): Create an issue.", "ping()", "long(): " + "a" * 200]


async def test_its_required_arguments_come_first_under_the_original_names(tmp_path: Path) -> None:
  """Its required arguments come first, each as name: type, and the others after them, each as name: type | None = None, under the original names of the schema."""
  root = mcped(tmp_path, {})[1]
  schema = {
    "type": "object",
    "properties": {"body": {"type": "string"}, "repo-name": {"type": "string"}, "n": {"type": "integer"}},
    "required": ["repo-name"],
  }
  tool = tooled("x", schema=schema)
  got = await engine.rung(f"close(mcp_signature({tool!r}))", on=root)
  assert got == "x(repo-name: str, body: str | None = None, n: int | None = None)"
