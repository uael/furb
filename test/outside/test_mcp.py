"""The kit mcp in a life of the engine of the crate: its word, the ear of the kits, and its process on the official
SDK, against the servers of the tests over stdio, over the streamable HTTP and over the older HTTP with SSE: each act
and each state, the controls that write the local scope, the changes that the process tells, a login by OAuth, and a
later life that replays its record and asks no server.
"""

import base64
import json
import re
import sys
from collections.abc import Iterator
from pathlib import Path

import pytest

import furb
import furb_monty.engine
from conftest import STANDS, Dead, swapped
from furb import engine
from furb.engine import Refused
from furb_monty import _monty

SERVER = r"""
import base64, os, sys
from mcp.server import MCPServer
from mcp.server.mcpserver import Context
from mcp.types import ImageContent, ToolAnnotations

if os.environ.get("STARTS"):
  with open(os.environ["STARTS"], "a") as starts:
    starts.write(f"start {os.getpid()}\n")
app = MCPServer("fixture", instructions="Use the fixture.", version="1.2.3")
PICTURE = base64.b64encode(b"\x89PNG\r\n\x1a\nfixture").decode()


@app.tool(title="Echo", description="Echoes a text.", annotations=ToolAnnotations(readOnlyHint=True))
def echo(text: str, ctx: Context) -> dict[str, str]:
  if ctx.headers is not None and os.environ.get("HEADERS"):
    with open(os.environ["HEADERS"], "a") as heard:
      heard.write(ctx.headers.get("x-test", "") + "\n")
  return {"text": text}


@app.tool(description="Gives a picture.")
def picture() -> ImageContent:
  return ImageContent(type="image", data=PICTURE, mimeType="image/png")


@app.tool(description="Gives a long text.")
def long() -> str:
  return "a" * 150_000


@app.tool(description="Fails.")
def fail() -> str:
  raise ValueError("it failed")


@app.tool(description="Waits.")
async def slow(seconds: float) -> str:
  import anyio
  await anyio.sleep(seconds)
  return "late"


@app.tool(description="Adds a tool.")
async def change(ctx: Context) -> str:
  app.add_tool(lambda: "added", name="added", description="Added.")
  await ctx.notify_tools_changed()
  return "changed"


@app.tool(description="Ends the server.")
def crash() -> str:
  os._exit(3)


@app.tool(description="A secret.")
def secret() -> str:
  return "secret"


for name in open(os.environ["TOOLS"]).read().split() if os.environ.get("TOOLS") else []:
  app.add_tool(lambda name=name: f"called {name}", name=name, description=f"The tool {name}.")


@app.resource("fixture://readme", name="readme", description="The readme.", mime_type="text/plain")
def readme() -> str:
  return "Read me."


@app.resource("fixture://file/{name}", name="file", description="A file.")
def file(name: str) -> str:
  return name


@app.prompt(description="Greets.")
def greet(who: str) -> str:
  return f"Greet {who}."


if __name__ == "__main__":
  app.run(sys.argv[1] if len(sys.argv) > 1 else "stdio")
"""
"""SERVER is the server of the tests, a module of the official SDK, which runs over stdio, or over HTTP when it is
given a transport."""

PICTURE = base64.b64encode(b"\x89PNG\r\n\x1a\nfixture").decode()
"""PICTURE is the image that the tool picture of the server of the tests gives, in base64."""

OFF = ("catchup", "memory", "skills", "compaction", "naming")
"""OFF are the extensions that a life of the tests turns off, so that the kit mcp alone plays."""


@pytest.fixture(autouse=True)
def on_monty() -> Iterator[None]:
  """Every name this module reads the engine by, bound to the engine of monty for the length of the test."""
  swapped(furb_monty.engine)
  yield
  swapped(furb.python)


class Place:
  """The folder of one test: the server, the configs of the user and of the project, and the record."""

  def __init__(self, at: Path) -> None:
    self.at = at
    self.config = at / "config"
    self.project = at / "project"
    self.project.mkdir()
    (at / "server.py").write_text(SERVER, encoding="utf-8")
    self.write(self.config / "config.json", {"skills": {**dict.fromkeys(OFF, False), "mcp": True}})

  def write(self, path: Path, value: object) -> None:
    """A file of JSON, written with its folder."""
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2), encoding="utf-8")

  def stdio(self, **more: object) -> dict[str, object]:
    """The definition of the server of the tests over stdio, which writes a line at each start, with more keys."""
    env = {"STARTS": str(self.at / "starts"), **more.pop("env", {})}  # ty: ignore[invalid-argument-type]
    return {"command": sys.executable, "args": [str(self.at / "server.py")], "env": env, **more}

  def user(self, servers: dict[str, object]) -> None:
    """The servers of the scope of the user."""
    self.write(self.config / "mcp.json", {"mcpServers": servers})

  def starts(self) -> list[str]:
    """The pid of each start of the server of the tests over stdio."""
    starts = self.at / "starts"
    return [line.split()[1] for line in starts.read_text().splitlines()] if starts.exists() else []

  def opened(self, *, record: bool = False) -> str:
    """A life on the ears of the crate in the project, with the kit mcp alone, and the id of its root."""
    stored, ears = _monty.opened(
      directory=str(self.project),
      record=str(self.at / "record.jsonl") if record else None,
      keeps=record,
      config=str(self.config),
      images=str(self.at / "images"),
      claude=str(self.at / "no-claude"),
      roster=[],
    )
    world = Dead(stands=[STANDS[0], str(self.project), STANDS[2]]).hears()
    self.ears = [one for _, one in ears]
    return furb_monty.engine.boot(list(stored), **dict(ears), world=world)

  def end(self) -> None:
    """The ears of the life let go, so each process of the kit ends."""
    for one in self.ears:
      one.dispose()


@pytest.fixture
def place(tmp_path: Path) -> Iterator[Place]:
  one = Place(tmp_path)
  yield one
  if hasattr(one, "ears"):
    one.end()


async def asked(root: str, kind: str, words: str = "") -> object:
  """What an act of the kit on the root comes to, with its words in Python."""
  return await engine.rung(f"close(await act({kind!r}, '', None, {words}))", on=root)


async def refused(root: str, kind: str, words: str = "") -> str:
  """Why an act of the kit on the root is refused."""
  with pytest.raises(Refused) as no:
    await asked(root, kind, words)
  return str(no.value)


async def server(root: str, name: str) -> dict:
  """The dict of one server."""
  servers = await asked(root, "mcp_servers")
  assert isinstance(servers, list)
  return next(one for one in servers if one["name"] == name)


async def answers_each_act(place: Place, root: str) -> None:
  """The acts that the server of the tests over stdio answers, with the config of its tests: a timeout of 1 second and
  the tool secret denied."""
  echo = {"text": '{\n  "text": "hi"\n}', "content": [{"type": "text", "text": '{\n  "text": "hi"\n}'}]}
  assert await asked(root, "mcp_call", "'fixture', 'echo', {'text': 'hi'}") == {**echo, "structured": {"text": "hi"}}
  picture = await asked(root, "mcp_call", "'fixture', 'picture'")
  assert isinstance(picture, dict)
  uri = picture["content"][0]["image"]
  assert picture == {
    "text": f"![image]({uri})",
    "content": [{"type": "image", "image": uri, "mime": "image/png"}],
    "structured": None,
  }
  stored = place.at / "images" / uri.removeprefix("furb-image://")
  assert stored.read_bytes() == base64.b64decode(PICTURE)
  long = await asked(root, "mcp_call", "'fixture', 'long'")
  assert isinstance(long, dict)
  assert long["text"] == "a" * 100_000 + "\n\n[furb cut this text at 100000 of its 150000 characters]"
  assert await refused(root, "mcp_call", "'fixture', 'fail'") == "Error executing tool fail"
  assert (
    await refused(root, "mcp_call", "'fixture', 'secret'") == "the config of the server fixture denies its tool secret"
  )
  slow = await refused(root, "mcp_call", "'fixture', 'slow', {'seconds': 3}")
  assert slow == "fixture.slow took more than 1 seconds, so furb cancelled it"
  assert await asked(root, "mcp_read", "'fixture', 'fixture://readme'") == {
    "text": "Read me.",
    "content": [{"type": "resource", "uri": "fixture://readme", "mime": "text/plain", "text": "Read me."}],
  }
  assert await asked(root, "mcp_prompt", "'fixture', 'greet', {'who': 'Ada'}") == {
    "text": "Greet Ada.",
    "messages": [{"role": "user", "content": {"type": "text", "text": "Greet Ada."}}],
  }
  assert await refused(root, "mcp_call", "'nobody', 'echo'") == "there is no server nobody"
  assert len(place.starts()) == 1, "one process served every act"
  answered = {
    a[2] for a in engine.transcript(on=root) if a[0] == "done" and re.fullmatch(r"mcp_(call|read|prompt)\d+", str(a[1]))
  }
  assert answered == {"kits"}, "the process of the kit answered each act"


async def test_a_server_over_stdio_offers_what_it_lists_but_its_denied_tools_and_answers_each_act(place: Place) -> None:
  """A server over stdio offers what it lists but its denied tools, and one process of it answers each act."""
  place.user({"fixture": place.stdio(deny=["secret"], timeout=1)})
  root = place.opened()
  one = await server(root, "fixture")
  assert one["state"] == "connected", one
  assert (one["scope"], one["transport"], one["error"]) == ("user", "stdio", "")
  assert one["target"] == f"{sys.executable} {place.at / 'server.py'}"
  assert (one["version"], one["instructions"]) == ("1.2.3", "Use the fixture.")
  names = [tool["name"] for tool in one["tools"]]
  assert names == ["echo", "picture", "long", "fail", "slow", "change", "crash"], "every tool, less the denied one"
  assert one["tools"][0] == {
    "name": "echo",
    "description": "Echoes a text.",
    "title": "Echo",
    "input": {
      "type": "object",
      "properties": {"text": {"title": "Text", "type": "string"}},
      "required": ["text"],
      "title": "echoArguments",
    },
    "output": {"additionalProperties": {"type": "string"}, "title": "echoDictOutput", "type": "object"},
    "annotations": {"readOnlyHint": True},
  }
  assert one["tools"][1] == {
    "name": "picture",
    "description": "Gives a picture.",
    "title": "",
    "input": {"type": "object", "properties": {}, "title": "pictureArguments"},
    "output": None,
    "annotations": {},
  }
  assert one["resources"] == [
    {"uri": "fixture://readme", "name": "readme", "description": "The readme.", "mime": "text/plain"}
  ]
  assert one["templates"] == [{"uri": "fixture://file/{name}", "name": "file", "description": "A file."}]
  assert one["prompts"] == [
    {"name": "greet", "description": "Greets.", "arguments": [{"name": "who", "description": "", "required": True}]}
  ]
  assert isinstance(one["binding"], str)
  await answers_each_act(place, root)


async def controls_write_the_local_scope(place: Place, root: str, local: Path) -> None:
  """What a control writes in the local config, and what it refuses."""

  def written() -> object:
    return json.loads(local.read_text())

  assert (await asked(root, "mcp_control", "'approve', 'shared'"))["state"] == "connected"
  assert written() == {"theme": "kept", "approved": ["shared"]}
  assert (await asked(root, "mcp_control", "'disable', 'shared'"))["state"] == "disabled"
  assert written() == {"theme": "kept", "approved": ["shared"], "enabled": [], "disabled": ["shared"]}
  assert (await asked(root, "mcp_control", "'enable', 'off'"))["state"] == "connected"
  assert written() == {"theme": "kept", "approved": ["shared"], "enabled": ["off"], "disabled": ["shared"]}
  assert (await asked(root, "mcp_control", "'reconnect', 'off'"))["state"] == "connected"
  assert len(place.starts()) == 3, "the approval, the enable and the reconnect each started a process"
  assert (
    await refused(root, "mcp_control", "'reconnect', 'shared'")
    == "the server shared is disabled: mcp_control('enable', \"shared\")"
  )
  assert (
    await refused(root, "mcp_control", "'login', 'off'") == "the server off runs on this machine and takes no login"
  )
  assert await refused(root, "mcp_control", "'approve', 'nobody'") == "there is no server nobody"
  assert (
    await refused(root, "mcp_control", "'fly', 'off'")
    == "the action fly is none of reconnect, enable, disable, approve, login and logout"
  )


async def test_each_server_has_its_state_and_a_control_writes_the_choice_of_the_operator_to_the_local_scope(
  place: Place,
) -> None:
  """Each server has its state, and a control writes the choice of the operator to the local scope."""
  place.user(
    {
      "off": place.stdio(disabled=True),
      "unset": {"command": "${MISSING}"},
      "absent": {"command": "/nonexistent/furb-mcp-server"},
      "pigeon": {"type": "carrier-pigeon", "url": "http://127.0.0.1"},
    }
  )
  place.write(place.project / ".mcp.json", {"mcpServers": {"shared": place.stdio()}})
  local = place.project / ".furb" / "mcp.json"
  place.write(local, {"theme": "kept"})
  root = place.opened()
  servers = await asked(root, "mcp_servers")
  assert isinstance(servers, list)
  assert [(one["name"], one["state"], one["scope"]) for one in servers] == [
    ("off", "disabled", "user"),
    ("unset", "failed", "user"),
    ("absent", "failed", "user"),
    ("pigeon", "failed", "user"),
    ("shared", "unapproved", "project"),
  ]
  assert (await server(root, "unset"))["error"] == "the variable MISSING is not set"
  assert (await server(root, "absent"))["error"].startswith("/nonexistent/furb-mcp-server did not start")
  assert (await server(root, "pigeon"))["error"] == "the type carrier-pigeon is none of stdio, http and sse"
  assert await refused(root, "mcp_call", "'shared', 'echo'") == "the server shared is not connected, it is unapproved"
  await controls_write_the_local_scope(place, root, local)
