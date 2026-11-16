"""The binding of the tools of a server that the process of the kit mcp makes: the word of Python that a chain runs so
that each tool is a function with the signature of its schema, and the calls of that word, run in monty, against the
tools of the servers of `tests/mcp-corpus`."""

import importlib.util
import json
from collections.abc import Iterator
from pathlib import Path
from types import ModuleType

import jsonschema
import pytest

import furb
import furb_monty.engine
from conftest import Dead, life, swapped
from furb import engine

ROOT = Path(__file__).resolve().parents[2]
CORPUS = ROOT / "tests" / "mcp-corpus"

SENDS = """import json
McpResult = dict[str, Any]


async def mcp_call(server, tool, arguments):
  return {'server': server, 'tool': tool, 'arguments': arguments}
"""
"""SENDS runs before a binding in the sandbox: a `mcp_call` that gives what a call sends."""

CALLS = 12
"""CALLS is the most calls of one function that the corpus makes."""


def loaded() -> ModuleType:
  """The process of the kit mcp as a module, read from its file, so the word `mcp.py` beside it shadows no package."""
  spec = importlib.util.spec_from_file_location("mcp_process", ROOT / ".agents" / "skills" / "mcp" / "process.py")
  assert spec is not None
  assert spec.loader is not None
  module = importlib.util.module_from_spec(spec)
  spec.loader.exec_module(module)
  return module


process = loaded()


@pytest.fixture(autouse=True)
def on_monty() -> Iterator[None]:
  """Every name this module reads the engine by, bound to the engine of monty for the length of the test."""
  swapped(furb_monty.engine)
  yield
  swapped(furb.python)


async def sent(word: str, calls: list[str]) -> list[object]:
  """What each call sends, in order, when the word of a binding runs in monty and the calls run after it."""
  made = "".join(f"got.append({one})\n" for one in calls)
  root = life(Dead())[1]
  return json.loads(await engine.rung(f"{SENDS}{word}got = []\n{made}close(json.dumps(got))", on=root))


def mapped(schema: object, defs: object) -> tuple[str, str]:
  """The type and the docstring of the function of a tool whose one required argument `x` has a schema."""
  made = process.function("t", {"inputSchema": schema_of(schema, defs)}, [])
  return process.python(made.params[0].shape), made.doc


def schema_of(schema: object, defs: object) -> dict:
  return {"type": "object", "properties": {"x": schema}, "required": ["x"], "$defs": defs}


def test_a_tool_becomes_an_async_function_of_a_class_named_for_its_server() -> None:
  """A tool becomes an async function of a class named for its server, with its required arguments by place."""
  tool = {
    "name": "create_issue",
    "description": "Create a new issue.\nIt opens it on GitHub.",
    "inputSchema": {
      "type": "object",
      "properties": {"owner": {"type": "string"}, "body": {"type": "string"}, "from": {"type": "integer"}},
      "required": ["owner"],
    },
  }
  assert process.binding("my-server", [tool]) == (
    "class my_server:\n"
    "  async def create_issue(owner: str, *, body: str | None = None, from_: int | None = None) -> McpResult:\n"
    "    'Create a new issue.'\n"
    "    return await mcp_call('my-server', 'create_issue', {'owner': owner, **{k: v for k, v in {'body': body, "
    "'from': from_}.items() if v is not None}})\n"
  )


@pytest.mark.parametrize(
  ("schema", "typed"),
  [
    ({"type": "string"}, "str"),
    ({"type": "integer"}, "int"),
    ({"type": "number"}, "float"),
    ({"type": "boolean"}, "bool"),
    ({"type": "null"}, "None"),
    ({"type": "array", "items": {"type": "string"}}, "list[str]"),
    ({"type": "array"}, "list[Any]"),
    ({"type": "object"}, "dict[str, Any]"),
    ({"type": "object", "additionalProperties": {"type": "integer"}}, "dict[str, int]"),
    ({"enum": ["a", "b"]}, "Literal['a', 'b']"),
    ({"enum": ["a", None]}, "Literal['a'] | None"),
    ({"const": 3}, "Literal[3]"),
    ({"const": True}, "Literal[True]"),
    ({"type": ["string", "null"]}, "str | None"),
    ({"anyOf": [{"type": "string"}, {"type": "integer"}]}, "str | int"),
    ({"oneOf": [{"const": "a"}, {"const": "b"}]}, "Literal['a', 'b']"),
    ({"anyOf": [{"type": "string"}, {"const": "a"}]}, "str"),
    ({"type": "string", "nullable": True}, "str | None"),
    ({"$ref": "#/$defs/n"}, "int"),
    ({"allOf": [{"$ref": "#/$defs/n"}]}, "int"),
    ({"type": "string", "anyOf": [{"format": "date"}, {"format": "uri"}]}, "str"),
    ({}, "Any"),
    (True, "Any"),
  ],
)
def test_each_type_of_json_schema_maps_to_its_type_of_python(schema: object, typed: str) -> None:
  """Each type of JSON Schema maps to its type of Python, with nothing loose."""
  assert mapped(schema, {"n": {"type": "integer"}}) == (typed, "")


@pytest.mark.parametrize(
  ("schema", "typed", "why"),
  [
    ({"enum": [1.5]}, "Any", "a Literal of Python cannot hold 1.5"),
    ({"type": "array", "prefixItems": [{"type": "string"}]}, "list[Any]", "its items differ by their place"),
    ({"$ref": "#/$defs/t"}, "list[Any]", "#/$defs/t holds itself"),
    ({"$ref": "#/$defs/none"}, "Any", "#/$defs/none is no part of its schema"),
    ({"not": {"type": "string"}}, "Any", "Python cannot say its not"),
    ({"allOf": [{"type": "string"}, {"type": "integer"}]}, "Any", "its allOf joins schemas of several types"),
    (False, "Any", "its schema takes no value"),
  ],
)
def test_a_part_that_python_cannot_say_maps_to_any_and_the_docstring_says_why(
  schema: object, typed: str, why: str
) -> None:
  """A part that Python cannot say maps to Any, or to a list of Any, and the docstring says why, with its schema."""
  defs = {"t": {"type": "array", "items": {"$ref": "#/$defs/t"}}}
  quoted = json.dumps(schema, separators=(",", ":"))
  assert mapped(schema, defs) == (typed, f"Loose: x is {typed}, since {why}; its schema: {quoted}.")


def test_the_docstring_sketches_each_object_whose_properties_python_cannot_type() -> None:
  """The docstring sketches each object whose properties Python cannot type, and such an argument is a dict."""
  item = {"type": "object", "properties": {"id": {"type": "string"}, "n": {"type": "integer"}}, "required": ["id"]}
  schema = {
    "type": "object",
    "properties": {"items": {"type": "array", "items": {"$ref": "#/$defs/item"}}},
    "required": ["items"],
    "$defs": {"item": item},
  }
  made = process.function("t", {"description": "Add items.", "inputSchema": schema}, [])
  assert made.doc == "Add items. Dicts: items is list[{'id': str, 'n'?: int}]."
  assert process.python(made.params[0].shape) == "list[dict[str, Any]]"


def test_each_name_becomes_safe_and_unique() -> None:
  """Each name becomes a safe name of Python, and the functions of one class take names of their own."""
  names = [process.safe(one, "x", typing=False) for one in ["from", "x-y", "1st", "__init__", ""]]
  assert names == ["from_", "x_y", "_1st", "_init__", "x"]
  assert [process.safe("list", "x", typing=True), process.safe("list", "x", typing=False)] == ["list_", "list"]
  word = process.binding("for", [{"name": "a-b"}, {"name": "a_b"}, {"name": "str"}])
  assert word.splitlines()[0] == "class for_:"
  assert [line.split("(")[0] for line in word.splitlines() if "async def" in line] == [
    "  async def a_b",
    "  async def a_b_",
    "  async def str_",
  ]


async def test_a_call_sends_each_required_argument_and_each_optional_one_that_is_not_none_under_its_name() -> None:
  """A call sends each required argument, and each optional one that is not None, under the name of its schema."""
  schema = {
    "type": "object",
    "properties": {
      "to-x": {"type": "string"},
      "from": {"type": ["integer", "null"]},
      "k": {"type": "integer"},
      "mcp_call": {"type": "boolean"},
    },
    "required": ["from"],
  }
  word = process.binding("srv", [{"name": "go", "inputSchema": schema}])
  calls = ["await srv.go(None)", "await srv.go(from_=2, to_x='a', k=None, mcp_call_=False)"]
  assert await sent(word, calls) == [
    {"server": "srv", "tool": "go", "arguments": {"from": None}},
    {"server": "srv", "tool": "go", "arguments": {"from": 2, "to-x": "a", "mcp_call": False}},
  ]


async def test_a_tool_with_no_arguments_takes_none_and_sends_an_empty_dict() -> None:
  """A tool with no arguments takes none and sends an empty dict, and a server with no tool binds nothing."""
  word = process.binding("s", [{"name": "ping", "inputSchema": {"type": "object"}}, {"name": "pong"}])
  assert "  async def ping() -> McpResult:\n    return await mcp_call('s', 'ping', {})\n" in word
  assert await sent(word, ["await s.pong()"]) == [{"server": "s", "tool": "pong", "arguments": {}}]
  assert process.binding("s", []) == ""


async def test_the_arguments_that_a_schema_takes_and_names_nowhere_are_keywords_of_their_own() -> None:
  """The arguments that a schema takes and names nowhere are keywords of their own, which a call sends."""
  schema = {
    "type": "object",
    "properties": {"a": {"type": "string"}},
    "required": ["a"],
    "additionalProperties": {"type": "integer"},
  }
  word = process.binding("s", [{"name": "t", "inputSchema": schema}])
  assert "async def t(a: str, **arguments: int) -> McpResult:" in word
  assert await sent(word, ["await s.t('x', b=1)"]) == [{"server": "s", "tool": "t", "arguments": {"a": "x", "b": 1}}]


def test_the_forms_of_the_arguments_of_a_tool_are_optional_and_loose() -> None:
  """The arguments of a tool that takes one of several forms are each optional, and the docstring says why."""
  schema = {
    "type": "object",
    "oneOf": [
      {"properties": {"a": {"type": "string"}}, "required": ["a"]},
      {"properties": {"b": {"type": "integer"}}, "required": ["b"]},
    ],
  }
  made = process.function("t", {"inputSchema": schema}, [])
  assert made.python("s").startswith("  async def t(*, a: str | None = None, b: int | None = None) -> McpResult:")
  assert made.doc == "Loose: its arguments take one of 2 forms, and each argument that a form alone names is optional."


def test_a_union_is_flat_with_each_shape_once_and_none_last() -> None:
  """A union is flat, with each shape once, the values of each Literal in one Literal, and None last."""
  shapes = [("None",), ("union", [("str",), ("Literal", [1])]), ("Literal", ["a", 2]), ("str",)]
  assert process.python(process.union(shapes)) == "str | Literal[1, 2] | None"
  assert process.union([("int",), ("Any",)]) == ("Any",)


def corpus() -> Iterator[tuple[str, list[dict]]]:
  """The servers of the corpus, each with its name and its tools."""
  for path in sorted(CORPUS.glob("*.json")):
    held = json.loads(path.read_text())
    yield held["server"], held["tools"]


PLAIN = {"str": "a", "int": 1, "float": 1.5, "bool": True}
"""PLAIN is the value that the corpus gives a parameter of each plain type."""


def samples(shape: tuple) -> list[object]:
  """The values that the corpus gives a parameter of a shape: one of each member and of each value of a Literal."""
  match shape:
    case ("Literal", values):
      return list(values)
    case ("list", item):
      return [[one] for one in samples(item)]
    case ("dict", value):
      return [{"a": one} for one in samples(value)] if value != ("Any",) else [{}]
    case ("union", members):
      return [one for member in members for one in samples(member)]
  return [PLAIN.get(shape[0])]


def calls(server: str, one: object) -> list[tuple[str, dict]]:
  """The calls of a function that the corpus makes from its signature, each with what it must send: one with its
  required parameters alone, by place, then one for each value that a parameter takes, with every parameter, by place
  and by name in turn."""
  pools = [samples(param.shape) for param in one.params]
  many = min(max([len(pool) for pool in pools] + [1]), CALLS)
  made = []
  for at in range(many + 1):
    given, arguments = [], {}
    for param, pool in zip(one.params, pools, strict=True):
      if not param.required and at == 0:
        continue
      value = pool[max(at - 1, 0) % len(pool)]
      given.append(repr(value) if param.required and at % 2 == 0 else f"{param.name}={value!r}")
      if param.required or value is not None:
        arguments[param.key] = value
    named = process.safe(server, "server", typing=True)
    made.append(
      (f"await {named}.{one.name}({', '.join(given)})", {"server": server, "tool": one.tool, "arguments": arguments})
    )
  return made


def functions(tools: list[dict]) -> list:
  taken: list[str] = []
  return [process.function(one["name"], one, taken) for one in tools]


async def test_each_call_of_the_corpus_sends_in_monty_what_its_signature_gives() -> None:
  """Each call of the bound tools of the corpus, run in monty, sends the arguments that its signature gives."""
  for server, tools in corpus():
    made = [call for one in functions(tools) for call in calls(server, one)]
    assert await sent(process.binding(server, tools), [call for call, _ in made]) == [want for _, want in made], server


def test_each_call_of_a_function_whose_docstring_says_nothing_loose_sends_what_its_schema_asks() -> None:
  """Each call of a function of the corpus whose docstring says nothing loose sends arguments that its schema takes."""
  wrong = []
  for server, tools in corpus():
    for one, tool in zip(functions(tools), tools, strict=True):
      if " Loose: " in f" {one.doc}" or " Dicts: " in f" {one.doc}":
        continue
      schema = tool.get("inputSchema", True)
      validator = jsonschema.validators.validator_for(schema)(schema)
      for call, sends in calls(server, one):
        faults = [fault.message for fault in validator.iter_errors(sends["arguments"])]
        wrong += [f"{server} {call}: {faults}"] if faults else []
  assert wrong == []
