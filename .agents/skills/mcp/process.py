"""The World part of the kit mcp: the process that holds the servers of the Model Context Protocol of a life and
answers each act of the kit, on the official SDK, `mcp`, which the extra `furb[mcp]` installs.

The configs define the servers in three scopes: the user scope, `mcp.json` of the config folder of the user; the
project scope, `.mcp.json` of the project, in the form of Claude Code; and the local scope, `.furb/mcp.json` of the
project, which holds the choices of the operator too. A later scope wins for a name, and the server keeps the place
where a scope first named it. A server of the project scope runs only once the operator approved it: `"approved"` and
`"approveAll"` of the local scope approve, `"disabled"` turns servers off, and `"enabled"` turns on a server whose
definition says `"disabled": true`. A server reaches furb over stdio, the streamable HTTP or the older HTTP with SSE,
and a remote server logs in by OAuth 2.1, whose tokens stand in the Keychain of macOS, and elsewhere in
`mcp-tokens.json` of the config folder, which only the user reads.

A server connects when an act first needs it, and its first connect gives its state: connected, failed, or needs-auth
when it asks for a login. A connection that ends connects again after a wait that doubles from one second, at most
thirty, and a server that fails five times in a row stays failed until a control reconnects it. The process says the
fact `("mcp_changed", "chain1", server)` when a server changes with no act that asked: a list that the server says has
changed, a connection that ends, and one that comes back. The end of the process ends each server that it started.

The acts, each of words after the chain that it is on, and what each comes to:

- `mcp_servers()`: the dict of each server, once each enabled and approved server connected or failed, or the bound of
  a connect passed: `{"name", "scope", "transport", "target", "state", "error", "version", "instructions", "tools",
  "resources", "templates", "prompts", "binding"}`, where `binding` is the word of Python that binds its tools, or "".
  A tool that the config denies is in no dict.
- `mcp_call(server, tool, arguments)`: `{"text", "content", "structured"}`. An image block is kept in the folder of the
  images of the life and named as `furb-image://<sha256>.<extension>`, and a text past 100000 characters is cut.
- `mcp_read(server, uri)`: `{"text", "content"}`. `mcp_prompt(server, name, arguments)`: `{"text", "messages"}`.
- `mcp_control(action, server)`, for reconnect, enable, disable, approve, login and logout: the dict of the server
  after it. enable, disable and approve write `.furb/mcp.json`; login runs OAuth 2.1 in the browser of the system, and
  logout forgets the tokens of the server.
"""

import asyncio
import base64
import contextlib
import dataclasses
import hashlib
import json
import keyword
import os
import subprocess
import sys
import tempfile
import time
import urllib.parse
import webbrowser
from collections.abc import AsyncIterator, Awaitable, Callable, Iterator
from pathlib import Path
from typing import IO, NoReturn

from furb.skill import Skill

try:
  import anyio
  import httpx2
  from mcp import types
  from mcp.client import Client, ClientRequestContext, IncomingMessage, Transport
  from mcp.client.auth import OAuthClientProvider
  from mcp.client.sse import sse_client
  from mcp.client.stdio import StdioServerParameters, stdio_client
  from mcp.client.streamable_http import streamable_http_client
  from mcp.shared.auth import AuthorizationCodeResult, OAuthClientInformationFull, OAuthClientMetadata, OAuthToken
  from mcp.shared.exceptions import MCPError
  from mcp.shared.message import SessionMessage
  from pydantic import BaseModel
except ImportError:
  sys.exit("The kit mcp needs the package mcp, which the extra furb[mcp] installs: pip install 'furb[mcp]'")

KINDS = ("mcp_servers", "mcp_call", "mcp_read", "mcp_prompt", "mcp_control")
CHANGED = "mcp_changed"
ROOT = "chain1"
"""The fact of a change of a server is about the root chain, so its transcript holds it."""

CALL = 60.0
CONNECT = 30.0
LOGIN = 300.0
RETRIES = 5
PAGES = 100
LONGEST = 100_000
LARGEST = 20 * 1024 * 1024
SERVICE = "furb-mcp"
KEYCHAIN = "/usr/bin/security"

type Json = bool | int | float | str | list[Json] | dict[str, Json] | None


class Refused(Exception):
  """Why an act comes to nothing: its text is what the act is refused with."""


class Login(Exception):
  """A server that asks for a login, which a connect that no person watches does not run."""


def field(one: Json, key: str) -> Json:
  return one.get(key) if isinstance(one, dict) else None


def text(one: Json, key: str) -> str:
  got = field(one, key)
  return got if isinstance(got, str) else ""


def items(one: Json) -> list[Json]:
  return one if isinstance(one, list) else []


def quoted(name: str) -> str:
  return json.dumps(name)


def seconds(span: float) -> str:
  return str(int(span)) if span == int(span) else repr(span)


def compact(value: Json) -> str:
  return json.dumps(value, separators=(",", ":"), ensure_ascii=False)


def dumped(model: BaseModel) -> dict[str, Json]:
  return model.model_dump(mode="json", by_alias=True, exclude_none=True)


# The config of the servers.


@dataclasses.dataclass(frozen=True)
class Run:
  """How a server that can run runs: its transport, what it reaches, how long a call of a tool may take, and the
  names of its tools that no chain may call."""

  transport: str
  target: str
  timeout: float
  deny: tuple[str, ...]
  command: str = ""
  args: tuple[str, ...] = ()
  env: tuple[tuple[str, str], ...] = ()
  cwd: str | None = None
  url: str = ""
  headers: tuple[tuple[str, str], ...] = ()
  client: str | None = None
  port: int | None = None


@dataclasses.dataclass(frozen=True)
class Server:
  """One server as the configs define it, after the choices of the operator; `run` is why it cannot run when it is
  a text."""

  name: str
  scope: str
  run: Run | str
  disabled: bool = False
  approved: bool = True


class Places:
  """The files of the three scopes."""

  def __init__(self, project: Path, config: Path) -> None:
    self.project, self.config = project, config

  def file(self, scope: str) -> Path:
    return {
      "user": self.config / "mcp.json",
      "project": self.project / ".mcp.json",
      "local": self.project / ".furb" / "mcp.json",
    }[scope]


def expand(said: str, variables: Callable[[str], str | None]) -> str:
  """A text with each `${VAR}` replaced by the value of the variable, and each `${VAR:-default}` by the default when
  the variable is not set or is empty."""
  out, rest = [], said
  while (at := rest.find("${")) >= 0:
    out.append(rest[:at])
    inside = rest[at + 2 :]
    end = inside.find("}")
    if end < 0:
      return "".join([*out, rest[at:]])
    name, sep, default = inside[:end].partition(":-")
    value = variables(name)
    if value is None or (sep and not value):
      if not sep:
        why = f"the variable {name} is not set"
        raise Refused(why)
      value = default
    out.append(value)
    rest = inside[end + 1 :]
  return "".join([*out, rest])


def read(path: Path) -> dict[str, Json] | None:
  """The map of a file of a scope, None when there is no file, or why it is not of the form."""
  try:
    held = json.loads(path.read_text())
  except FileNotFoundError:
    return None
  except (OSError, ValueError) as no:
    why = f"{path}: {getattr(no, 'strerror', None) or no}"
    raise Refused(why) from no
  if not isinstance(held, dict):
    why = f"{path}: the config is no map"
    raise Refused(why)
  return held


class Fields:
  """The fields of a definition, read with each variable expanded."""

  def __init__(self, one: dict[str, Json], variables: Callable[[str], str | None]) -> None:
    self.one, self.variables = one, variables

  def shape(self, key: str, kind: str) -> NoReturn:
    why = f"{key} is no {kind}"
    raise Refused(why)

  def text(self, key: str) -> str | None:
    held = self.one.get(key)
    if held is not None and not isinstance(held, str):
      self.shape(key, "string")
    return None if held is None else expand(held, self.variables)

  def pairs(self, key: str) -> tuple[tuple[str, str], ...]:
    held = self.one.get(key)
    if held is not None and not isinstance(held, dict):
      self.shape(key, "map")
    pairs = []
    for name, value in (held or {}).items():
      if not isinstance(value, str):
        self.shape(f"{key}.{name}", "string")
      pairs.append((name, expand(value, self.variables)))
    return tuple(pairs)

  def run(self, project: Path) -> Run:
    kind = self.text("type") or ""
    timeout, deny = self.timeout(), self.deny()
    if kind in ("", "stdio"):
      command = self.text("command")
      if not command:
        why = "the server names no command"
        raise Refused(why)
      held = self.one.get("args")
      if held is not None and not isinstance(held, list):
        self.shape("args", "list")
      if not all(isinstance(one, str) for one in held or []):
        why = "args holds a value that is no string"
        raise Refused(why)
      args = tuple(expand(str(one), self.variables) for one in held or [])
      cwd = self.text("cwd")
      return Run(
        "stdio",
        " ".join([command, *args]),
        command=command,
        args=args,
        env=self.pairs("env"),
        cwd=None if cwd is None else str(project / Path(cwd).expanduser()),
        timeout=timeout,
        deny=deny,
      )
    if kind not in ("http", "streamable-http", "sse"):
      why = f"the type {kind} is none of stdio, http and sse"
      raise Refused(why)
    url = self.text("url")
    if not url:
      why = "the server names no url"
      raise Refused(why)
    parts = urllib.parse.urlsplit(url)
    if not (parts.scheme and parts.netloc):
      why = f"the url {url} is not valid"
      raise Refused(why)
    oauth = self.one.get("oauth")
    if oauth is not None and not isinstance(oauth, dict):
      self.shape("oauth", "map")
    port = field(oauth, "callbackPort")
    if port is not None and (isinstance(port, bool) or port not in range(1, 1 << 16)):
      self.shape("oauth.callbackPort", "port")
    transport = "sse" if kind == "sse" else "http"
    client = text(oauth, "clientId") or None
    headers = self.pairs("headers")
    return Run(transport, url, url=url, headers=headers, client=client, port=port, timeout=timeout, deny=deny)  # ty: ignore[invalid-argument-type]

  def timeout(self) -> float:
    held = self.one.get("timeout")
    if held is None:
      return CALL
    if isinstance(held, bool) or not isinstance(held, int | float) or held <= 0:
      self.shape("timeout", "number of seconds above 0")
    return float(held)

  def deny(self) -> tuple[str, ...]:
    held = self.one.get("deny")
    if held is not None and not isinstance(held, list):
      self.shape("deny", "list")
    return tuple(one for one in held or [] if isinstance(one, str))


def defined(name: str, scope: str, one: Json, project: Path, variables: Callable[[str], str | None]) -> Server:
  """A server of a definition, or one that cannot run with why."""
  if not isinstance(one, dict):
    return Server(name, scope, "the server is no map")
  try:
    return Server(name, scope, Fields(one, variables).run(project))
  except Refused as no:
    return Server(name, scope, str(no))


def servers(places: Places, variables: Callable[[str], str | None]) -> list[Server]:
  """The servers that the configs define, in the order they first name them, and, as a server with the path of its
  file as its name, each file that is not of the form, whose error says why."""
  files: list[tuple[str, Path, dict[str, Json] | Refused | None]] = []
  for scope in ("user", "project", "local"):
    try:
      files.append((scope, places.file(scope), read(places.file(scope))))
    except Refused as no:
      files.append((scope, places.file(scope), no))
  local = files[2][2] if isinstance(files[2][2], dict) else {}

  def listed(key: str) -> list[Json]:
    return items(local.get(key))

  found: dict[str, Server] = {}
  for scope, path, held in files:
    if held is None:
      continue
    defines = held if isinstance(held, Refused) else held.get("mcpServers")
    if isinstance(defines, Refused) or (defines is not None and not isinstance(defines, dict)):
      why = str(defines) if isinstance(defines, Refused) else f"{path}: mcpServers is no map"
      found[str(path)] = Server(str(path), scope, why)
      continue
    for name, one in (defines or {}).items():
      off = field(one, "disabled") is True
      server = defined(name, scope, one, places.project, variables)
      found[name] = dataclasses.replace(
        server,
        disabled=name in listed("disabled") or (off and name not in listed("enabled")),
        approved=scope != "project" or local.get("approveAll") is True or name in listed("approved"),
      )
  return list(found.values())


def choose(places: Places, choice: str, name: str) -> None:
  """A choice of the operator about a server, written to the local scope with every other key of that file kept."""
  path = places.file("local")
  held = read(path) or {}
  lists = {"enable": ("disabled", "enabled"), "disable": ("enabled", "disabled"), "approve": ("", "approved")}
  out, into = lists[choice]
  for key in (out, into):
    if key:
      kept = [one for one in items(held.get(key)) if one != name]
      held[key] = [*kept, name] if key == into else kept
  try:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(held, indent=2) + "\n")
  except OSError as no:
    why = f"{path}: {no.strerror}"
    raise Refused(why) from no


# The binding of the tools of a server: a word of Python made from their schemas, which a chain runs so that each tool
# is a function with the signature of its schema. The word is one class under the name of the server, with one async
# function for each tool, which takes no self and has no decorator: the sandbox binds no `staticmethod`. The name of
# the class stands on the first line of the word alone, so a word that finds the name taken renames the class with one
# replace. The word imports nothing: it names `Any` and `Literal`, which the module of each chain holds from the
# engine, and `McpResult` and `mcp_call`, which the word of the kit binds.

SENTENCE = 200
QUOTED = 160
SKETCHED = 300
DEPTH = 4
FOLLOWED = 32
TYPES = ("Any", "Literal", "McpResult", "str", "int", "float", "bool", "list", "dict")
SAYS = ("type", "enum", "const", "$ref", "anyOf", "oneOf", "allOf", "properties", "items")
PLAIN = {"string": "str", "integer": "int", "number": "float", "boolean": "bool", "null": "None"}

type Shape = tuple
"""A type of Python that a schema maps to: `(name,)` for Any, str, int, float, bool and None, `("list", item)`,
`("dict", value)`, `("Literal", values)` and `("union", members)`."""
ANY: Shape = ("Any",)
NONE: Shape = ("None",)


def python(shape: Shape) -> str:
  match shape:
    case ("list", item):
      return f"list[{python(item)}]"
    case ("dict", value):
      return f"dict[str, {python(value)}]"
    case ("Literal", values):
      return "Literal[" + ", ".join(map(repr, values)) + "]"
    case ("union", members):
      return " | ".join(map(python, members))
  return shape[0]


def typed(one: Json) -> tuple[str, Json]:
  """A value of JSON with its type, since True equals 1 in Python and differs from it in JSON."""
  return type(one).__name__, one


def flat(parts: list[Shape]) -> Iterator[Shape]:
  for one in parts:
    if one[0] == "union":
      yield from flat(one[1])
    else:
      yield one


def union(parts: list[Shape]) -> Shape:
  """The union of shapes: flat, each shape once, the values of each Literal in one Literal less those that a plain
  type of the union takes, None last, and `Any` when a member is `Any` or no member stands."""
  kept: list[Shape] = []
  values: list[Json] = []
  for one in flat(parts):
    if one == ANY:
      return ANY
    if one[0] == "Literal":
      kept += [] if ("Literal",) in kept else [("Literal",)]
      values += [x for x in one[1] if typed(x) not in map(typed, values)]
    elif one not in kept:
      kept.append(one)
  taken = {"str": ("str",), "bool": ("bool",), "int": ("int",), "float": ("float",)}
  values = [x for x in values if taken[type(x).__name__] not in kept and not (type(x) is int and ("float",) in kept)]
  shapes = [("Literal", values) if one == ("Literal",) else one for one in kept if one != ("Literal",) or values]
  shapes = [one for one in shapes if one != NONE] + [NONE] * (NONE in shapes)
  return ANY if not shapes else shapes[0] if len(shapes) == 1 else ("union", shapes)


def safe(name: str, empty: str, *, typing: bool) -> str:
  """A name made safe for Python: each character that is no letter, digit or underscore of ASCII becomes an
  underscore, a name with two underscores or more at its start keeps one, a name that starts with a digit takes an
  underscore before it, a keyword takes one after it, and an empty name is `empty`. A name of a class or of a function
  of a class takes an underscore after it too when an annotation reads it as a type."""
  made = "".join(one if one.isascii() and (one.isalnum() or one == "_") else "_" for one in name)
  made = "_" + made.lstrip("_") if made.startswith("__") else made or empty
  made = "_" + made if made[0].isdigit() else made
  return made + "_" if keyword.iskeyword(made) or (typing and made in TYPES) else made


def unique(name: str, taken: list[str]) -> str:
  while name in taken:
    name += "_"
  taken.append(name)
  return name


def cut_to(said: str, most: int) -> str:
  return said if len(said) <= most else said[: max(most - 3, 0)].rstrip() + "..."


def sentence(said: str) -> str:
  """The first sentence of the first paragraph of a description, on one line, cut."""
  line = " ".join(said.strip().split("\n\n")[0].split())
  ends = [at + 1 for at, one in enumerate(line) if one in ".!?" and line[at + 1 : at + 2] == " "]
  return cut_to(line[: ends[0] if ends else len(line)], SENTENCE)


def pointer(root: Json, path: str) -> Json:
  """The part of a schema at a JSON pointer, or None."""
  at = root
  for step in path.split("/")[1:] if path else []:
    part = step.replace("~1", "/").replace("~0", "~")
    if isinstance(at, dict) and part in at:
      at = at[part]
    elif isinstance(at, list) and part.isdigit() and int(part) < len(at):
      at = at[int(part)]
    else:
      return None
  return at


def says(member: Json) -> bool:
  """Whether a member of a union says a type, and is no mere constraint on the type of its parent."""
  return any(key in member for key in SAYS) if isinstance(member, dict) else True


@dataclasses.dataclass
class Param:
  name: str
  key: str
  shape: Shape
  required: bool


@dataclasses.dataclass
class Function:
  """The function of one tool: the name of the tool and of the function, its docstring, its parameters with the
  required ones first, and the name and the shape of the arguments that the schema takes and names nowhere."""

  tool: str
  name: str
  doc: str
  params: list[Param]
  rest: tuple[str, Shape] | None

  def python(self, server: str) -> str:
    names = [one.name for one in self.params] + ([self.rest[0]] if self.rest else [])
    key, value = unique("k", names), unique("v", names)
    required = [one for one in self.params if one.required]
    optional = [one for one in self.params if not one.required]
    signature = [f"{one.name}: {python(one.shape)}" for one in required]
    signature += ["*"] if optional else []
    signature += [f"{one.name}: {python(union([one.shape, NONE]))} = None" for one in optional]
    signature += [f"**{self.rest[0]}: {python(self.rest[1])}"] if self.rest else []
    sent = [f"{one.key!r}: {one.name}" for one in required]
    if optional:
      given = ", ".join(f"{one.key!r}: {one.name}" for one in optional)
      sent.append(f"**{{{key}: {value} for {key}, {value} in {{{given}}}.items() if {value} is not None}}")
    sent += [f"**{self.rest[0]}"] if self.rest else []
    arguments = sent[0][2:] if len(sent) == 1 and sent[0].startswith("**") else "{" + ", ".join(sent) + "}"
    doc = f"    {self.doc!r}\n" if self.doc else ""
    return (
      f"  async def {self.name}({', '.join(signature)}) -> McpResult:\n{doc}"
      f"    return await mcp_call({server!r}, {self.tool!r}, {arguments})\n"
    )


class Mapper:
  """What maps the schemas of one tool: the schema of its arguments, which each reference resolves in, the references
  that the shape now stands in, why each part mapped loosely, and how many objects whose properties Python cannot type
  it mapped."""

  def __init__(self, root: Json) -> None:
    self.root, self.seen, self.notes, self.opaque = root, list[str](), list[str](), 0

  def note(self, why: str) -> Shape:
    self.notes.append(why)
    return ANY

  def followed(self, at: Json) -> Json:
    """The schema that a schema refers to at its top, as far as references go, or None when one refers to a part that
    the root does not hold, or they never end."""
    for _ in range(FOLLOWED):
      target = field(at, "$ref")
      if not isinstance(target, str):
        return at
      at = pointer(self.root, target[1:]) if target.startswith("#") else None
      if at is None:
        return None
    return None

  def fields(self) -> tuple[list[list], Shape | None]:
    """The arguments of the tool, each as its key, its schema and whether the tool requires it, and the shape of the
    arguments that the schema takes and names nowhere, when it takes them."""
    one = self.arguments()
    if not isinstance(one, dict):
      return [], one[1]
    fields: list[list] = []
    parts = [one, *(x for x in map(self.followed, items(one.get("allOf"))) if isinstance(x, dict))]
    for part in parts:
      listed(part, fields)
    for key in ("anyOf", "oneOf"):
      before = len(fields)
      for member in (x for x in map(self.followed, items(one.get(key))) if isinstance(x, dict)):
        listed(member, fields)
      if len(fields) > before:
        forms = len(items(one.get(key)))
        self.notes.append(
          f"its arguments take one of {forms} forms, and each argument that a form alone names is optional"
        )
    for key in (x for part in parts for x in items(part.get("required")) if isinstance(x, str)):
      found = next((x for x in fields if x[0] == key), None)
      if found is None:
        self.notes.append(f"{key} is required and has no schema")
        fields.append([key, True, True])
      else:
        found[2] = True
    more = one.get("additionalProperties")
    if more is True or (not isinstance(more, dict) and "patternProperties" in one):
      return fields, ANY
    return fields, self.shape(more) if isinstance(more, dict) else None

  def arguments(self) -> dict[str, Json] | tuple[str, Shape | None]:
    """The object of the schema of the arguments, or, when the schema holds no object, the shape of the arguments
    that it takes and names nowhere."""
    found = self.followed(self.root)
    if found is None:
      return "rest", self.note("the schema of its arguments refers to a part that it does not hold")
    if found is True:
      return "rest", None
    if not isinstance(found, dict):
      return "rest", self.note(f"the schema of its arguments is {cut_to(compact(found), QUOTED)}")
    kind = found.get("type")
    if "type" in found and kind != "object" and "object" not in items(kind):
      return "rest", self.note(f"its arguments are of the type {compact(kind)}, and not an object")
    return found

  def sketch(self, schema: Json, depth: int) -> str:
    """The sketch of a schema for a docstring: an object that names its properties shows each, as `"key": shape`,
    with `?` after the key of a property that it does not require, and a list shows its items, to `depth` levels."""
    at = self.followed(schema)
    at = schema if at is None else at
    if not isinstance(at, dict):
      return "Any"
    named = at.get("properties")
    if isinstance(named, dict) and named:
      required = items(at.get("required"))
      parts = [f"{k!r}{'' if k in required else '?'}: {self.sketch(x, depth - 1)}" for k, x in named.items()]
      return "dict" if depth == 0 else "{" + ", ".join(parts) + "}"
    for key in ("anyOf", "oneOf"):
      if any(map(says, items(at.get(key)))):
        return "dict" if depth == 0 else " | ".join(dict.fromkeys(self.sketch(x, depth - 1) for x in items(at[key])))
    if (at.get("type") == "array" or "items" in at) and isinstance(at.get("items"), dict):
      return "list" if depth == 0 else f"list[{self.sketch(at['items'], depth - 1)}]"
    notes, opaque = len(self.notes), self.opaque
    got = python(self.shape(at))
    del self.notes[notes:]
    self.opaque = opaque
    return got

  def shape(self, schema: Json) -> Shape:
    if isinstance(schema, dict):
      got = self.core(schema)
      return union([got, NONE]) if schema.get("nullable") is True else got
    if schema is True:
      return ANY
    if schema is False:
      return self.note("its schema takes no value")
    return self.note(f"its schema {cut_to(compact(schema), QUOTED)} is no schema")

  def core(self, one: dict[str, Json]) -> Shape:
    """The shape of a schema that is an object, before `nullable`."""
    if isinstance(one.get("$ref"), str):
      return self.referred(str(one["$ref"]))
    if "const" in one or "enum" in one:
      values = [one["const"]] if "const" in one else items(one["enum"])
      return self.literal(values) if values else self.note("its enum holds no value")
    for key in ("anyOf", "oneOf"):
      if any(map(says, items(one.get(key)))):
        return union([self.shape(member) for member in items(one[key])])
    joined = self.joined(items(one.get("allOf")))
    return joined if joined is not None else self.typed(one)

  def typed(self, one: dict[str, Json]) -> Shape:
    """The shape of a schema by its type, or by the keywords that say one."""
    kind = one.get("type")
    if isinstance(kind, str):
      return self.kind(kind, one)
    if isinstance(kind, list):
      return union(
        [
          self.kind(x, one) if isinstance(x, str) else self.note(f"its type {compact(x)} is no type of JSON")
          for x in kind
        ]
      )
    if "type" in one:
      return self.note(f"its type {compact(kind)} is no type of JSON")
    if any(key in one for key in ("properties", "additionalProperties", "patternProperties")):
      return self.object(one)
    if "items" in one or "prefixItems" in one:
      return self.kind("array", one)
    said = next((key for key in ("not", "if") if key in one), None)
    return self.note(f"Python cannot say its {said}") if said else ANY

  def referred(self, at: str) -> Shape:
    if at in self.seen:
      return self.note(f"{at} holds itself")
    found = pointer(self.root, at[1:]) if at.startswith("#") else None
    if found is None:
      return self.note(f"{at} is no part of its schema")
    self.seen.append(at)
    got = self.shape(found)
    self.seen.pop()
    return got

  def joined(self, members: list[Json]) -> Shape | None:
    """The shape that the members of an `allOf` join to, None when no member has a type."""
    shapes = [self.shape(one) for one in members if says(one)]
    if not shapes:
      return None
    if all(one[0] == "dict" for one in shapes):
      return ("dict", ANY)
    return (
      shapes[0] if all(one == shapes[0] for one in shapes) else self.note("its allOf joins schemas of several types")
    )

  def kind(self, kind: str, one: dict[str, Json]) -> Shape:
    if kind in PLAIN:
      return (PLAIN[kind],)
    if kind == "array" and ("prefixItems" in one or isinstance(one.get("items"), list)):
      return ("list", self.note("its items differ by their place"))
    if kind == "array":
      return ("list", self.shape(one["items"]) if "items" in one else ANY)
    if kind == "object":
      return self.object(one)
    return self.note(f"its type {json.dumps(kind)} is no type of JSON")

  def object(self, one: dict[str, Json]) -> Shape:
    """A dict whose values have the shape of the schema of its additional properties, or any value when it names its
    properties, which Python cannot type in the sandbox."""
    if field(one, "properties") or "patternProperties" in one:
      self.opaque += 1
      return ("dict", ANY)
    more = one.get("additionalProperties")
    return ("dict", self.shape(more) if isinstance(more, dict) else ANY)

  def literal(self, values: list[Json]) -> Shape:
    """The shape of the values of an enum or a const: a Literal of each str, int and bool, and None for null."""
    held = [one for one in values if one is not None]
    lost = next((one for one in held if not isinstance(one, str | bool | int)), None)
    if lost is not None:
      return self.note(f"a Literal of Python cannot hold {cut_to(compact(lost), QUOTED)}")
    return union(([("Literal", held)] if held else []) + ([NONE] if None in values else []))


def listed(part: dict[str, Json], fields: list[list]) -> None:
  """Each property of a schema that the fields do not hold yet, as optional."""
  named = part.get("properties")
  for key, schema in named.items() if isinstance(named, dict) else []:
    if not any(x[0] == key for x in fields):
      fields.append([key, schema, False])


def function(tool: str, about: dict[str, Json], taken: list[str]) -> Function:
  """The function of one tool, whose name the functions before it did not take."""
  name = unique(safe(tool, "tool", typing=True), taken)
  mapper = Mapper(about["inputSchema"] if "inputSchema" in about else about.get("input", True))
  fields, rest = mapper.fields()
  said, mapper.notes = mapper.notes, []
  names, params, sketches = ["mcp_call"], list[Param](), list[str]()
  for key, part, required in fields:
    objects = mapper.opaque
    shape = mapper.shape(part)
    reasons, mapper.notes = mapper.notes, []
    param = unique(safe(key, "argument", typing=False), names)
    if reasons:
      why = ", ".join(reasons)
      said.append(f"{param} is {python(shape)}, since {why}; its schema: {cut_to(compact(part), QUOTED)}")
    if mapper.opaque > objects:
      sketches.append(f"{param} is {cut_to(mapper.sketch(part, DEPTH), SKETCHED)}")
    params.append(Param(param, key, shape, required))
  params.sort(key=lambda one: not one.required)
  told = [about.get("description"), about.get("title"), field(about.get("annotations"), "title")]
  doc = next((sentence(x) for x in told if isinstance(x, str) and x.strip()), "")
  doc += "." if doc and not doc.endswith((".", "!", "?")) else ""
  doc += f" Dicts: {'; '.join(sketches)}." if sketches else ""
  doc += f" Loose: {'; '.join(said)}." if said else ""
  more = (unique("arguments", names), rest) if rest is not None else None
  return Function(tool, name, doc.lstrip(), params, more)


def binding(server: str, tools: list[Json]) -> str:
  """The word that binds the tools of a server, each as MCP lists it: its name, its description, and the schema of
  its arguments. A server with no tool binds nothing, and its word is empty."""
  taken: list[str] = []
  functions = [function(text(one, "name"), one, taken) for one in tools if isinstance(one, dict) and text(one, "name")]
  word = "".join(one.python(server) for one in functions)
  return f"class {safe(server, 'server', typing=True)}:\n{word}" if word else ""


# The answers of a server as the acts give them: the blocks of a result, each as furb keeps it, and their texts joined.
# An image goes to the folder of the images of the life, as the images of a turn do, and its text names it, so a model
# that reads the text sees it.

IMAGES = (
  ("image/png", "png", b"\x89PNG\r\n\x1a\n"),
  ("image/jpeg", "jpg", b"\xff\xd8\xff"),
  ("image/gif", "gif", b"GIF8"),
  ("image/webp", "webp", b"RIFF"),
)


def cut(said: str) -> str:
  """A text cut at the longest that an answer holds, which then says so."""
  return (
    said
    if len(said) <= LONGEST
    else f"{said[:LONGEST]}\n\n[furb cut this text at {LONGEST} of its {len(said)} characters]"
  )


def pictured(data: bytes) -> tuple[str, str] | None:
  """The media type and the extension of an image by its first bytes."""
  gif, webp = data[4:6] in (b"7a", b"9a"), data[8:12] == b"WEBP"
  return next(
    (
      (media, suffix)
      for media, suffix, head in IMAGES
      if data.startswith(head) and (suffix != "gif" or gif) and (suffix != "webp" or webp)
    ),
    None,
  )


def decoded(data: str) -> bytes | None:
  try:
    return base64.b64decode(data.strip(), validate=True)
  except ValueError:
    return None


class Content:
  """What turns the answers of a server into what the acts give, with the folder of the images of the life."""

  def __init__(self, images: Path | None) -> None:
    self.images = images

  def image(self, data: str, mime: str) -> tuple[str, dict[str, Json]]:
    """The text of an image, which names the image as the folder of the images keeps it, and its block."""
    raw = decoded(data)
    found = pictured(raw or b"")
    why = (
      "its data is no base64"
      if raw is None
      else "this life keeps no directory of images to show it"
      if self.images is None
      else "An image must be a file of at most 20 MiB."
      if len(raw) > LARGEST
      else "Choose a PNG, JPEG, GIF, or WebP image."
      if found is None
      else self.kept(raw, found[1])
    )
    if raw is None or self.images is None or found is None or why:
      return f"[an image of type {mime}: {why}]", {"type": "image", "image": None, "mime": mime}
    uri = f"furb-image://{hashlib.sha256(raw).hexdigest()}.{found[1]}"
    return f"![image]({uri})", {"type": "image", "image": uri, "mime": found[0]}

  def kept(self, raw: bytes, suffix: str) -> str:
    """The bytes of an image kept in the folder of the images under their digest, or why they are not."""
    folder = self.images or Path()
    at = folder / f"{hashlib.sha256(raw).hexdigest()}.{suffix}"
    # An image is named by the digest of its bytes, and lands whole, so a file that stands under that name is it.
    if at.exists():
      return ""
    try:
      folder.mkdir(parents=True, exist_ok=True)
      with tempfile.NamedTemporaryFile(dir=folder, delete=False) as out:
        out.write(raw)
      Path(out.name).chmod(0o600)
      Path(out.name).replace(at)
    except OSError as no:
      return str(no)
    return ""

  def embedded(self, resource: Json) -> tuple[str, dict[str, Json]]:
    """The text of a resource that a server gives whole, and its block: its text, the image it holds, or what it
    holds when it is other bytes."""
    uri, mime = text(resource, "uri"), text(resource, "mimeType")
    if isinstance(field(resource, "text"), str):
      return text(resource, "text"), {"type": "resource", "uri": uri, "mime": mime, "text": cut(text(resource, "text"))}
    raw = decoded(text(resource, "blob")) or b""
    if pictured(raw):
      return self.image(text(resource, "blob"), mime)
    said = f"[the resource {uri} holds {len(raw)} bytes of {mime}, which furb shows no further]"
    return said, {"type": "resource", "uri": uri, "mime": mime, "size": len(raw)}

  def block(self, block: Json) -> tuple[str, dict[str, Json]]:
    """The text of a block and the block as furb keeps it, its own text cut."""
    sort, mime = text(block, "type"), text(block, "mimeType")
    if sort == "text":
      return text(block, "text"), {"type": "text", "text": cut(text(block, "text"))}
    if sort == "image":
      return self.image(text(block, "data"), mime)
    if sort == "audio":
      return f"[audio of type {mime}, which furb does not play]", {"type": "audio", "mime": mime}
    if sort == "resource":
      return self.embedded(field(block, "resource"))
    if sort == "resource_link":
      kept: dict[str, Json] = {
        "type": sort,
        **{x: text(block, x) for x in ("uri", "name", "description")},
        "mime": mime,
      }
      return f"[the resource {text(block, 'name')} at {text(block, 'uri')}]", kept
    return f"[a block of the kind {sort}, which furb does not show]", {"type": sort}

  def blocks(self, blocks: list[Json]) -> tuple[str, list[Json]]:
    pairs = [self.block(one) for one in blocks]
    return cut("\n".join(one[0] for one in pairs)), [one[1] for one in pairs]

  def called(self, got: dict[str, Json]) -> dict[str, Json]:
    """What a call of a tool gives: its text, its blocks and its structured content; or the refusal of a result that
    is an error, with its text."""
    said, content = self.blocks(items(got.get("content")))
    structured = got.get("structuredContent")
    if got.get("isError") is True:
      raise Refused(said or "the tool failed and said nothing")
    if not content and structured is not None:
      said = cut(compact(structured))
    return {"text": said, "content": content, "structured": structured}

  def read(self, got: dict[str, Json]) -> dict[str, Json]:
    pairs = [self.embedded(one) for one in items(got.get("contents"))]
    return {"text": cut("\n".join(one[0] for one in pairs)), "content": [one[1] for one in pairs]}

  def prompted(self, got: dict[str, Json]) -> dict[str, Json]:
    pairs = [(text(one, "role") or "user", self.block(field(one, "content"))) for one in items(got.get("messages"))]
    messages: list[Json] = [{"role": role, "content": kept} for role, (_, kept) in pairs]
    return {"text": cut("\n\n".join(said for _, (said, _) in pairs)), "messages": messages}


# The login of a remote server and the store of its tokens.


class Vault:
  """The store of the tokens of this machine: the Keychain of macOS, under the service furb-mcp, and else a file of
  the config folder that only the user reads. A secret goes to the Keychain on the stdin of its program, so it stands
  in the arguments of no process."""

  def __init__(self, config: Path) -> None:
    self.file = config / "mcp-tokens.json"
    self.keychain = sys.platform == "darwin" and Path(KEYCHAIN).exists()

  @staticmethod
  def account(key: str) -> str:
    return "".join(one if one.isascii() and (one.isalnum() or one in ":/._@-") else "_" for one in key)

  def held(self) -> dict[str, Json]:
    try:
      held = json.loads(self.file.read_text())
    except OSError, ValueError:
      return {}
    return held if isinstance(held, dict) else {}

  def wrote(self, held: dict[str, Json]) -> None:
    self.file.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile("w", dir=self.file.parent, delete=False) as out:
      out.write(json.dumps(held, indent=2))
    Path(out.name).chmod(0o600)
    Path(out.name).replace(self.file)

  def secured(self, *args: str, given: str = "") -> subprocess.CompletedProcess[str]:
    return subprocess.run([KEYCHAIN, *args], input=given, capture_output=True, text=True, check=False)  # noqa: S603

  def get(self, key: str) -> str | None:
    if not self.keychain:
      held = self.held().get(key)
      return held if isinstance(held, str) else None
    got = self.secured("find-generic-password", "-s", SERVICE, "-a", self.account(key), "-w")
    return got.stdout.removesuffix("\n") if got.returncode == 0 else None

  def put(self, key: str, value: str) -> None:
    if not self.keychain:
      self.wrote({**self.held(), key: value})
      return
    given = f"add-generic-password -U -s {SERVICE} -a {self.account(key)} -X {value.encode().hex()}\n"
    got = self.secured("-i", given=given)
    if got.returncode != 0 or got.stderr:
      why = f"the Keychain kept nothing: {got.stderr.strip()}"
      raise Refused(why)

  def forget(self, key: str) -> None:
    if self.keychain:
      self.secured("delete-generic-password", "-s", SERVICE, "-a", self.account(key))
    elif (held := self.held()).pop(key, None) is not None:
      self.wrote(held)


class Storage:
  """The tokens and the client of one server in the vault, as the SDK reads and writes them. A token keeps when it
  ends, so a later process refreshes it before it serves. A login takes a client of its redirect alone, else it
  registers one; a connect that no person watches takes any client, so it registers none before it asks for a
  login."""

  def __init__(self, vault: Vault, key: str, run: Run, redirect: str, *, asking: bool) -> None:
    self.vault, self.key, self.client, self.redirect, self.asking = vault, key, run.client, redirect, asking

  async def held(self) -> dict[str, Json]:
    try:
      held = json.loads(await asyncio.to_thread(self.vault.get, self.key) or "{}")
    except ValueError:
      return {}
    return held if isinstance(held, dict) else {}

  async def kept(self, key: str, value: dict[str, Json]) -> None:
    await asyncio.to_thread(self.vault.put, self.key, json.dumps({**await self.held(), key: value}))

  async def get_tokens(self) -> OAuthToken | None:
    tokens = (await self.held()).get("tokens")
    if not isinstance(tokens, dict):
      return None
    ends = tokens.pop("ends", None)
    left = int(ends - time.time()) if isinstance(ends, int | float) else None
    return OAuthToken.model_validate({**tokens, "expires_in": None if left is None else max(left, 0)})

  async def set_tokens(self, tokens: OAuthToken) -> None:
    kept = dumped(tokens)
    if tokens.expires_in is not None:
      kept["ends"] = time.time() + tokens.expires_in
    kept.pop("expires_in", None)
    await self.kept("tokens", kept)

  async def get_client_info(self) -> OAuthClientInformationFull | None:
    held = (await self.held()).get("client")
    if isinstance(held, dict):
      client = OAuthClientInformationFull.model_validate(held)
      if not self.asking or self.redirect in map(str, client.redirect_uris or []):
        return client
    if self.client or not self.asking:
      return OAuthClientInformationFull.model_validate(
        {"client_id": self.client or "furb", "redirect_uris": [self.redirect]}
      )
    return None

  async def set_client_info(self, client_info: OAuthClientInformationFull) -> None:
    await self.kept("client", dumped(client_info))


def browse(url: str) -> None:
  """The address opened in the browser of the system."""
  if not webbrowser.open(url):
    why = f"the browser did not open: open {url} to log in"
    raise Refused(why)


PAGE = b"<!doctype html><title>furb</title><p>The login of furb is done. You can close this page.</p>"


async def listened(port: int | None) -> tuple[asyncio.Server, asyncio.Future[dict[str, str]]]:
  """A listener on 127.0.0.1 that takes the query of the one redirect of a login to /callback."""
  came: asyncio.Future[dict[str, str]] = asyncio.get_running_loop().create_future()

  async def served(reader: asyncio.StreamReader, writer: asyncio.StreamWriter) -> None:
    line = (await reader.readline()).decode(errors="replace").split(" ")
    target = urllib.parse.urlsplit(line[1] if len(line) > 1 else "/")
    if target.path == "/callback" and not came.done():
      came.set_result(dict(urllib.parse.parse_qsl(target.query)))
    head = f"HTTP/1.1 200 OK\r\ncontent-type: text/html; charset=utf-8\r\ncontent-length: {len(PAGE)}\r\n"
    writer.write(f"{head}connection: close\r\n\r\n".encode() + PAGE)
    with contextlib.suppress(OSError):
      await writer.drain()
    writer.close()

  return await asyncio.start_server(served, "127.0.0.1", port or 0), came


# The servers of a life.


@dataclasses.dataclass(eq=False)
class Entry:
  """One server of the life: its definition, its state and why, what it said of itself, what it offers, the client
  of its connection, and the keeper that connects it."""

  server: Server
  state: str = "pending"
  why: str = ""
  version: str = ""
  instructions: str = ""
  offered: dict[str, list[Json]] = dataclasses.field(default_factory=dict)
  client: Client | None = None
  keeper: asyncio.Task[None] | None = None


def leaves(no: BaseException) -> Iterator[BaseException]:
  """Each exception that an exception holds, itself first, through its groups and its causes."""
  yield no
  for one in no.exceptions if isinstance(no, BaseExceptionGroup) else ():
    yield from leaves(one)
  if no.__cause__ is not None:
    yield from leaves(no.__cause__)


def said(no: BaseException) -> str:
  """Why a connect or a request failed, in words: the first exception that a group holds."""
  while isinstance(no, BaseExceptionGroup) and no.exceptions:
    no = no.exceptions[0]
  if isinstance(no, MCPError):
    return f"{no.message} (error {no.code})"
  if isinstance(no, TimeoutError):
    return f"it took more than {seconds(CONNECT)} seconds"
  return str(no) or type(no).__name__


def stderr(errors: IO[str]) -> str:
  """The last lines that a process of a server wrote on its stderr, after a sign, or nothing."""
  with contextlib.suppress(OSError, ValueError):
    errors.flush()
    errors.seek(0)
    lines = errors.read().splitlines()[-20:]
    return "; its stderr: " + "\n".join(line[:1000] for line in lines) if lines else ""
  return ""


@contextlib.asynccontextmanager
async def watched(transport: Transport, news: asyncio.Queue[str]) -> AsyncIterator[Transport]:
  """A transport whose end, as the end of the process of a server, puts `ended` on the news."""
  async with transport as (read, write):
    send, receive = anyio.create_memory_object_stream[SessionMessage | Exception](0)

    async def relayed() -> None:
      async with send:
        with contextlib.suppress(anyio.ClosedResourceError, anyio.BrokenResourceError):
          async for one in read:
            await send.send(one)
      news.put_nowait("ended")

    @contextlib.asynccontextmanager
    async def streams() -> AsyncIterator[tuple]:
      yield receive, write

    async with anyio.create_task_group() as group:
      group.start_soon(relayed)
      try:
        yield streams()
      finally:
        group.cancel_scope.cancel()


class Hub:
  """The servers of a life, which tell each change that no act asked for to `told`."""

  def __init__(self, project: Path, config: Path, images: Path | None, told: Callable[[str], object]) -> None:
    self.places, self.content, self.told, self.vault = Places(project, config), Content(images), told, Vault(config)
    self.entries: list[Entry] = []
    self.loaded = False
    self.moved = asyncio.Event()
    self.variables: Callable[[str], str | None] = os.environ.get

  def entry(self, name: str) -> Entry:
    found = next((one for one in self.entries if one.server.name == name), None)
    if found is None:
      why = f"there is no server {name}"
      raise Refused(why)
    return found

  def notify(self) -> None:
    self.moved.set()
    self.moved = asyncio.Event()

  def set(self, entry: Entry, state: str, why: str = "") -> bool:
    """A state of an entry while it stands among the entries, and whether it did."""
    if entry not in self.entries:
      return False
    entry.state, entry.why = state, why
    self.notify()
    return True

  def tell(self, entry: Entry) -> None:
    if entry in self.entries:
      self.told(entry.server.name)

  def reload(self) -> None:
    """The servers read again: a server whose definition did not change keeps its connection, a new or changed one
    connects when it may run, and one that the configs no longer define ends."""
    self.loaded = True
    held, self.entries = self.entries, []
    for server in servers(self.places, self.variables):
      kept = next((one for one in held if one.server == server), None)
      if kept is None:
        kept = Entry(server)
        if isinstance(server.run, str):
          kept.state, kept.why = "failed", server.run
        elif server.disabled or not server.approved:
          kept.state = "disabled" if server.disabled else "unapproved"
        else:
          self.launch(kept)
      else:
        held.remove(kept)
      self.entries.append(kept)
    for one in held:
      if one.keeper is not None:
        one.keeper.cancel()
    self.notify()

  def launch(self, entry: Entry) -> None:
    """A keeper started for an entry, which connects it, in place of the keeper that it had."""
    if entry.keeper is not None:
      entry.keeper.cancel()
    entry.state, entry.why, entry.client = "pending", "", None
    entry.keeper = asyncio.ensure_future(self.keep(entry))

  async def settled(self, names: list[str]) -> None:
    """Once no server of these names, or no server at all when it names none, is pending, or the bound passed."""
    with contextlib.suppress(TimeoutError):
      async with asyncio.timeout(CONNECT + 5):
        while any(one.state == "pending" and (not names or one.server.name in names) for one in self.entries):
          await self.moved.wait()

  async def end(self) -> None:
    """Every server ended: each connection closed, and each process of a server ended with each process that it
    started."""
    keepers = [one.keeper for one in self.entries if one.keeper is not None]
    for one in keepers:
      one.cancel()
    await asyncio.gather(*keepers, return_exceptions=True)

  def described(self, entry: Entry) -> dict[str, Json]:
    """The dict of a server, as the act `mcp_servers` gives it. A tool that the config denies is not in it."""
    server, run = entry.server, entry.server.run
    deny = run.deny if isinstance(run, Run) else ()
    allowed = [one for one in entry.offered.get("tools", []) if text(one, "name") not in deny]
    why = entry.why
    if entry.state == "needs-auth":
      why = f"the server asks for a login: mcp_control('login', {quoted(server.name)})"

    def picked(one: Json, **keys: str) -> dict[str, Json]:
      return {key: text(one, at) for key, at in keys.items()}

    tools: list[Json] = [
      {
        **picked(one, name="name", description="description"),
        "title": text(one, "title") or text(field(one, "annotations"), "title"),
        "input": field(one, "inputSchema") or {"type": "object"},
        "output": field(one, "outputSchema"),
        "annotations": field(one, "annotations") or {},
      }
      for one in allowed
    ]
    prompts: list[Json] = [
      {
        **picked(one, name="name", description="description"),
        "arguments": [
          {**picked(x, name="name", description="description"), "required": field(x, "required") is True}
          for x in items(field(one, "arguments"))
        ],
      }
      for one in entry.offered.get("prompts", [])
    ]
    return {
      "name": server.name,
      "scope": server.scope,
      "transport": run.transport if isinstance(run, Run) else "",
      "target": run.target if isinstance(run, Run) else "",
      "state": entry.state,
      "error": why,
      "version": entry.version,
      "instructions": entry.instructions,
      "tools": tools,
      "resources": [
        picked(x, uri="uri", name="name", description="description", mime="mimeType")
        for x in entry.offered.get("resources", [])
      ],
      "templates": [
        picked(x, uri="uriTemplate", name="name", description="description") for x in entry.offered.get("templates", [])
      ],
      "prompts": prompts,
      "binding": binding(server.name, allowed) if entry.state == "connected" else "",
    }

  async def answer(self, kind: str, words: list[Json]) -> Json:
    """What an act of the kit comes to, or a `Refused` with why it comes to nothing."""
    first, second, third = [*words, None, None, None][:3]
    server, name = first if isinstance(first, str) else "", second if isinstance(second, str) else ""
    arguments = third if isinstance(third, dict) else {}
    if kind == "mcp_servers":
      self.reload()
      await self.settled([])
      return [self.described(one) for one in self.entries]
    if kind == "mcp_control":
      return await self.control(server, name)
    client, run = await self.session(server)
    if kind == "mcp_call":
      if name in run.deny:
        why = f"the config of the server {server} denies its tool {name}"
        raise Refused(why)
      return self.content.called(await self.asked(client.call_tool(name, arguments), f"{server}.{name}", run.timeout))
    if kind == "mcp_read":
      return self.content.read(await self.asked(client.read_resource(name), f"{server} {name}", run.timeout))
    given = {key: one if isinstance(one, str) else compact(one) for key, one in arguments.items()}
    got = await self.asked(client.get_prompt(name, given), f"the prompt {name} of {server}", run.timeout)
    return self.content.prompted(got)

  async def asked(self, request: Awaitable[BaseModel], what: str, bound: float) -> dict[str, Json]:
    """What a server answers a request with, within the timeout of its server."""
    try:
      async with asyncio.timeout(bound):
        return dumped(await request)
    except TimeoutError:
      why = f"{what} took more than {seconds(bound)} seconds, so furb cancelled it"
      raise Refused(why) from None
    except MCPError as no:
      why = f"{what} failed: {said(no)}"
      raise Refused(why) from no

  async def session(self, name: str) -> tuple[Client, Run]:
    """The client of a connected server and its definition, once it settled, or why it has none."""
    if not self.loaded:
      self.reload()
    await self.settled([name])
    entry = self.entry(name)
    if entry.state == "connected" and entry.client is not None and isinstance(entry.server.run, Run):
      return entry.client, entry.server.run
    if entry.state == "needs-auth":
      why = f"the server {name} asks for a login: mcp_control('login', {quoted(name)})"
    else:
      why = f"the server {name} is not connected, it is {entry.state}" + (
        f": {entry.why}" if entry.state == "failed" else ""
      )
    raise Refused(why)

  def restart(self, name: str) -> None:
    """A server that may run, connected again from the start."""
    server = self.entry(name).server
    if isinstance(server.run, str):
      why = f"the server {name} cannot run: {server.run}"
    elif server.disabled:
      why = f"the server {name} is disabled: mcp_control('enable', {quoted(name)})"
    elif not server.approved:
      why = f"the server {name} of the project waits for approval: mcp_control('approve', {quoted(name)})"
    else:
      self.launch(self.entry(name))
      return
    raise Refused(why)

  async def control(self, action: str, name: str) -> dict[str, Json]:
    """What a control of the operator does to a server, and its dict after it, once it settled."""
    if not self.loaded:
      self.reload()
    run = self.entry(name).server.run
    if action in ("login", "logout") and not (isinstance(run, Run) and run.url):
      why = f"the server {name} runs on this machine and takes no login"
      raise Refused(why)
    if action in ("enable", "disable", "approve"):
      choose(self.places, action, name)
      self.reload()
    elif action in ("reconnect", "login", "logout") and isinstance(run, Run):
      if action == "login":
        await self.login(name, run)
      if action == "logout":
        await asyncio.to_thread(self.vault.forget, f"{name} {run.url}")
      self.restart(name)
    else:
      why = f"the action {action} is none of reconnect, enable, disable, approve, login and logout"
      raise Refused(why)
    await self.settled([name])
    return self.described(self.entry(name))

  def oauth(
    self, name: str, run: Run, redirect: str, came: asyncio.Future[dict[str, str]] | None
  ) -> OAuthClientProvider:
    """The OAuth 2.1 of a remote server: with no future of a redirect, as at a connect, a server that asks for a login
    raises `Login`."""

    async def redirected(url: str) -> None:
      if came is None:
        raise Login(name)
      browse(url)

    async def called() -> AuthorizationCodeResult:
      try:
        async with asyncio.timeout(LOGIN):
          query = await came if came is not None else {}
      except TimeoutError:
        why = f"no login came back within {seconds(LOGIN)} seconds"
        raise Refused(why) from None
      if "error" in query:
        why = f"the login failed, {query['error']}" + (
          f": {query['error_description']}" if query.get("error_description") else ""
        )
        raise Refused(why)
      if "code" not in query:
        why = "the login came back with no code"
        raise Refused(why)
      return AuthorizationCodeResult(code=query["code"], state=query.get("state"))

    metadata = OAuthClientMetadata.model_validate(
      {
        "client_name": "furb",
        "redirect_uris": [redirect],
        "grant_types": ["authorization_code", "refresh_token"],
        "token_endpoint_auth_method": "none",
      }
    )
    storage = Storage(self.vault, f"{name} {run.url}", run, redirect, asking=came is not None)
    return OAuthClientProvider(run.url, metadata, storage, redirected, called)

  async def login(self, name: str, run: Run) -> None:
    """A login by OAuth 2.1 in the browser of the system, whose tokens the vault keeps."""
    listener, came = await listened(run.port)
    port = listener.sockets[0].getsockname()[1]
    auth = self.oauth(name, run, f"http://127.0.0.1:{port}/callback", came)
    try:
      async with httpx2.AsyncClient(auth=auth, headers=dict(run.headers), timeout=LOGIN) as client:
        # A request that the server lets in after the login ends the flow: its answer serves no more.
        await client.post(run.url, json={"jsonrpc": "2.0", "id": 0, "method": "ping"})
    except Exception as no:
      raise Refused(said(no)) from no
    finally:
      listener.close()

  async def keep(self, entry: Entry) -> None:
    """The keeper of a server: it connects the server, keeps what the server offers as the server changes it, and
    connects it again when its connection ends. A keeper whose entry left the servers ends."""
    failures, quiet = 0, True
    while isinstance(run := entry.server.run, Run):
      try:
        why = await self.connected(entry, run, quiet=quiet)
        failures, quiet, wait = 0, False, 1
      except Exception as no:
        if any(isinstance(one, Login) for one in leaves(no)):
          if self.set(entry, "needs-auth") and not quiet:
            self.tell(entry)
          return
        failures += 1
        why, wait = said(no), min(1 << min(failures, 5), 30)
        if quiet or failures >= RETRIES:
          gave = (
            ""
            if quiet
            else f"; furb gave up after {RETRIES} tries: mcp_control('reconnect', {quoted(entry.server.name)})"
          )
          if self.set(entry, "failed", why + gave) and not quiet:
            self.tell(entry)
          return
      if not self.set(entry, "failed", f"{why}; furb connects again in {wait} s"):
        return
      self.tell(entry)
      await asyncio.sleep(wait)

  async def connected(self, entry: Entry, run: Run, *, quiet: bool) -> str:
    """A connection to a server, held while it lasts: what the server offers is kept as the server changes it, and
    the result is why the connection ended."""
    news: asyncio.Queue[str] = asyncio.Queue()

    async def heard(message: IncomingMessage) -> None:
      method = getattr(message, "method", "")
      if isinstance(method, str) and method.endswith("/list_changed"):
        news.put_nowait(method)

    async def roots(context: ClientRequestContext) -> types.ListRootsResult:
      del context
      project = self.places.project.resolve()
      return types.ListRootsResult(roots=[types.Root(uri=project.as_uri() + "/", name=project.name)])

    with tempfile.TemporaryFile("w+") as errors:
      async with contextlib.AsyncExitStack() as stack:
        ended = await stack.enter_async_context(watched(self.transport(entry, run, errors), news))
        try:
          async with asyncio.timeout(CONNECT):
            client = await stack.enter_async_context(
              Client(ended, list_roots_callback=roots, message_handler=heard, cache=None)
            )
            offered = await self.offered(client)
        except Exception as no:
          if any(isinstance(one, Login) for one in leaves(no)):
            raise
          why = said(no) + stderr(errors)
          raise Refused(why) from no
        info = client.server_info
        entry.version, entry.instructions = (info.version if info else "") or "", client.instructions or ""
        entry.offered, entry.client = offered, client
        if not self.set(entry, "connected"):
          return "the server left the configs"
        if not quiet:
          self.tell(entry)
        while (method := await news.get()) != "ended":
          with contextlib.suppress(Exception):
            async with asyncio.timeout(CONNECT):
              entry.offered = {**entry.offered, **await self.offered(client, method)}
            self.notify()
            self.tell(entry)
        entry.client = None
        return "the connection closed" + stderr(errors)

  def transport(self, entry: Entry, run: Run, errors: IO[str]) -> Transport:
    """The transport of the SDK that reaches a server."""
    if run.transport == "stdio":
      params = StdioServerParameters(command=run.command, args=list(run.args), env=dict(run.env), cwd=run.cwd)
      return stdio_client(params, errlog=errors)  # ty: ignore[invalid-argument-type]
    auth = self.oauth(entry.server.name, run, "http://127.0.0.1/callback", None)
    if run.transport == "sse":
      return sse_client(run.url, headers=dict(run.headers), auth=auth)
    client = httpx2.AsyncClient(headers=dict(run.headers), auth=auth, timeout=httpx2.Timeout(CONNECT, read=300))
    return streamable_http_client(run.url, http_client=client)

  async def offered(self, client: Client, changed: str = "") -> dict[str, list[Json]]:
    """What a server offers, each list that its capabilities name, or the list that changed: a server that lists no
    templates offers none."""
    lists = {
      "tools": ("tools", client.list_tools, "tools"),
      "resources": ("resources", client.list_resources, "resources"),
      "templates": ("resources", client.list_resource_templates, "resourceTemplates"),
      "prompts": ("prompts", client.list_prompts, "prompts"),
    }
    got: dict[str, list[Json]] = {}
    for name, (capability, listing, key) in lists.items():
      if changed and capability not in changed:
        continue
      got[name] = []
      if getattr(client.server_capabilities, capability, None) is None:
        continue
      cursor = None
      for _ in range(PAGES):
        try:
          page = dumped(await listing(cursor=cursor))
        except MCPError:
          if name == "templates":
            break
          raise
        got[name] += items(page.get(key))
        cursor = page.get("nextCursor")
        if not isinstance(cursor, str) or not cursor:
          break
    return got


async def main() -> None:
  skill = await Skill.attach(kinds=[])
  folders = skill.connection.welcome.get("folders") or {}
  changes: asyncio.Queue[str] = asyncio.Queue()
  images = folders.get("images")
  hub = Hub(Path(folders["project"]), Path(folders["config"]), Path(images) if images else None, changes.put_nowait)
  for kind in KINDS:
    skill.answers(kind, lambda fact, kind=kind: hub.answer(kind, fact[4:]))  # ty: ignore[invalid-argument-type]

  async def told() -> None:
    while True:
      await skill.fact(CHANGED, ROOT, await changes.get())

  telling = asyncio.ensure_future(told())
  try:
    await skill.closed
  finally:
    telling.cancel()
    await hub.end()


if __name__ == "__main__":
  asyncio.run(main())
