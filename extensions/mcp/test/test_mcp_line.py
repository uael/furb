"""mcp_line, one line for a server."""

from pathlib import Path

from extensions.conftest import mcped, server, tooled
from furb import engine


async def test_mcp_line_is_one_line_for_a_server(tmp_path: Path) -> None:
  """mcp_line(server) is one line for a server: its name and its state, and for a connected server the count of its tools and the names of the first 12, the count of its resources and templates, the count of its prompts, and its instructions cut at 400 characters."""
  root = mcped(tmp_path, {})[1]
  rich = server(
    "github",
    tools=[tooled(f"t{i}") for i in range(14)],
    resources=[{"uri": "a"}, {"uri": "b"}],
    templates=[{"uri": "c"}],
    prompts=[{"name": "p"}] * 3,
    instructions="  Use   it\nwell.  ",
  )
  few = server("few", tools=[tooled("one")])
  long = server("long", instructions="x" * 500)
  got = await engine.rung(f"close([mcp_line(x) for x in {[rich, server('bare'), few, long]!r}])", on=root)
  names = ", ".join(f"t{i}" for i in range(12))
  assert got == [
    f"github connected, 14 tools ({names}, and 2 more), 3 resources, 3 prompts. Instructions: Use it well.",
    "bare connected, no tools",
    "few connected, 1 tools (one)",
    "long connected, no tools. Instructions: " + "x" * 400,
  ]


async def test_a_failed_server_adds_its_error_and_the_others_add_the_call_that_mends_them(tmp_path: Path) -> None:
  """A failed server adds its error, and a server in the state needs-auth or unapproved adds the mcp_control call that mends it."""
  root = mcped(tmp_path, {})[1]
  servers = [
    server("f", state="failed", error="boom"),
    server("g", state="failed"),
    server("n", state="needs-auth"),
    server("u", state="unapproved"),
    server("d", state="disabled"),
    server("p", state="pending"),
  ]
  got = await engine.rung(f"close([mcp_line(x) for x in {servers!r}])", on=root)
  assert got == [
    "f failed: boom",
    "g failed: no reason given",
    "n needs-auth: mcp_control('login', 'n') signs in",
    "u unapproved: mcp_control('approve', 'u') lets it run",
    "d disabled",
    "p pending",
  ]
