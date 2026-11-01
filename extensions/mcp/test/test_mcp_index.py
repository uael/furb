"""mcp_index, the life word of the extension."""

from pathlib import Path

from conftest import of, settle, world_says
from extensions.conftest import bindings, mcped, quoted, server, tooled
from furb import engine
from furb.engine import Refused


async def changed(root: str, name: str = "a") -> None:
  """Say that a server changed, as the ear of the MCP does, and give the loop room."""
  world_says("mcp_changed", root, name)
  await settle()


def told(root: str) -> list[str]:
  """The paragraphs that the index told of servers, which leave out the controls over it."""
  return [x for x in of(engine.turns(on=root), "mcp_index") if "_servers" in x]


async def test_mcp_index_asks_the_world_for_the_servers_when_made_and_at_each_change_and_completes_after_the_first(
  tmp_path: Path,
) -> None:
  """mcp_index() is the life word of the extension, as await mcp_index(): an act on the chain that asks the World for the servers when it is made and at each fact mcp_changed, and that completes once it told the first answer of the World and ran each binding of it, so a word that awaits it holds what it tells, and it goes on after that."""
  root, heard = mcped(tmp_path, {"mcp_servers": [server("a")]})[1:]
  assert await engine.rung(
    "await mcp_index()\nclose([x[1] for x in transcript() if x[0] == 'tell' and x[1].startswith('mcp')])", on=root
  ) == ["mcp_index1"]
  index = await engine.rung("close(mcp_index())", on=root)
  assert isinstance(index, str)
  await settle()
  assert [a[0] for a in heard] == ["mcp_servers", "mcp_servers"]
  assert engine.peek(index, "living") is None
  await changed(root)
  assert [a[0] for a in heard] == ["mcp_servers", "mcp_servers", "mcp_servers", "mcp_servers"]


async def test_it_tells_the_lines_once_then_only_the_lines_that_changed_and_nothing_for_an_empty_list(
  tmp_path: Path,
) -> None:
  """It tells the lines of the servers, as mcp_line gives them, once, and after a change only the lines that changed, with name is gone for a server that left; it tells nothing when the list is empty or the World refuses it."""
  one, bad, good = server("a"), server("b", state="failed", error="boom"), server("b")
  states = [Refused("No config."), [], [one, bad], [one, good], [one, good], [one]]
  root = mcped(tmp_path, {"mcp_servers": lambda _: states.pop(0)})[1]
  await engine.rung("mcp_index()", on=root)
  await settle()
  counts = []
  for _ in range(5):
    counts.append(len(told(root)))
    await changed(root)
  assert counts == [0, 0, 1, 2, 2]
  assert of(engine.turns(on=root), "mcp_index") == [
    "#mcp_index1\n" + quoted("mcp_index1_servers", "a connected, no tools\nb failed: boom"),
    "#mcp_index1 changed\nmcp_index1_servers = 'b connected, no tools'",
    "#mcp_index1 changed\nmcp_index1_servers = 'b is gone'",
  ]


async def test_it_binds_at_once_the_class_of_each_small_connected_server_and_names_mcp_use_for_a_large_one(
  tmp_path: Path,
) -> None:
  """It runs at once, as a rung of the chain, the binding of each connected server whose binding holds at most BIND characters, and the line of that server names its class; the line of a larger connected server names mcp_use."""
  small = server("github", tools=[tooled("ping")], binding=bindings("github"))
  large = server("big", tools=[tooled("x")], binding="class big:\n  pass\n# " + "x" * 3100 + "\n")
  down = server("down", state="failed", error="boom", binding=bindings("down"))
  root = mcped(tmp_path, {"mcp_servers": [small, large, down]})[1]
  await engine.rung("await mcp_index()", on=root)
  lines = "github connected, 1 tools (ping). Class github.\nbig connected, 1 tools (x). mcp_use('big') binds its class."
  lines += "\ndown failed: boom"
  assert of(engine.turns(on=root), "mcp_index") == ["#mcp_index1\n" + quoted("mcp_index1_servers", lines)]
  assert await engine.rung("close([hasattr(github, 'ping'), 'big' in module(), 'down' in module()])", on=root) == [
    True,
    False,
    False,
  ]


async def test_a_cancel_over_it_ends_it_and_a_pause_over_it_holds_it_until_the_wake(tmp_path: Path) -> None:
  """A cancel over it ends it, and a pause over it holds it until the wake."""
  states = [[server("a")], [server("a"), server("b")], [server("a"), server("b"), server("c")]]
  root, heard = mcped(tmp_path, {"mcp_servers": lambda _: states.pop(0)})[1:]
  index = await engine.rung("close(mcp_index())", on=root)
  assert isinstance(index, str)
  await settle()
  engine.pause(index)
  await changed(root)
  assert len(told(root)) == 1
  engine.wake(index)
  await settle()
  assert len(told(root)) == 2
  engine.cancel(index)
  await changed(root)
  assert len(told(root)) == 2
  assert len(heard) == 2
