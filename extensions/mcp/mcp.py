import json

from furb.engine import (
  HEAD,
  Act,
  Refused,
  Show,
  Text,
  act,
  answer,
  bound,
  controlled,
  covers,
  module,
  over,
  peek,
  program,
  rung,
  shown,
  told,
)

mcp = """MCP servers give you tools, resources and prompts. The index that this chain was told lists each server with its
state and its tools, and names the class that holds the tools of a server: await its methods, as
await github.create_issue(...), each of which gives an McpResult, a dict with text, content and structured.
- mcp_tools(server) tells the signature of each tool of a server, and mcp_tool(server, name) tells the whole schema of
  one.
- mcp_use(server) binds the class of a server that the index left unbound. Use the class in your next reply, since this
  reply cannot name it.
- mcp_call(server, tool, arguments) calls a tool with the original names of its arguments. A tool that fails raises
  Refused with its text.
- mcp_read(server, uri) reads a resource, and mcp_prompt(server, name, arguments) expands a prompt. @server:uri in a
  message of the operator names a resource: read it with mcp_read.
- mcp_servers() lists the servers as dicts, and mcp_control(action, server) does reconnect, enable, disable, approve,
  login or logout."""
BIND = 3000

type McpResult = dict


def mcp_type(schema: object) -> str:
  if not isinstance(schema, dict):
    return "Any"
  kind = schema.get("type")
  options = schema.get("anyOf") or schema.get("oneOf")
  if isinstance(options, list) and options:
    found = " | ".join({mcp_type(x): 0 for x in options})
  elif "const" in schema:
    found = f"Literal[{schema['const']!r}]"
  elif isinstance(schema.get("enum"), list) and schema["enum"]:
    found = "Literal[" + ", ".join(repr(x) for x in schema["enum"]) + "]"
  elif isinstance(kind, list):
    found = " | ".join({mcp_type({**schema, "type": x}): 0 for x in kind}) or "Any"
  elif kind == "array":
    found = f"list[{mcp_type(schema.get('items'))}]"
  else:
    names = {
      "string": "str",
      "integer": "int",
      "number": "float",
      "boolean": "bool",
      "null": "None",
      "object": "dict[str, Any]",
    }
    found = names.get(kind, "Any") if isinstance(kind, str) else "Any"
  return found


def mcp_signature(tool: dict) -> str:
  schema = tool.get("input") or {}
  props = schema.get("properties") or {}
  need = schema.get("required") or []
  args = [f"{n}: {mcp_type(p)}" for n, p in props.items() if n in need]
  args += [f"{n}: {mcp_type(p)} | None = None" for n, p in props.items() if n not in need]
  text = " ".join(str(tool.get("description") or "").split())
  cut = text.find(". ")
  first = (text if cut < 0 else text[: cut + 1])[:200]
  line = f"{tool['name']}({', '.join(args)})"
  return f"{line}: {first}" if first else line


def mcp_line(server: dict) -> str:
  name, state = server["name"], server["state"]
  line = f"{name} {state}"
  if state == "failed":
    return f"{line}: {server.get('error') or 'no reason given'}"
  if state == "needs-auth":
    return f"{line}: mcp_control('login', {name!r}) signs in"
  if state == "unapproved":
    return f"{line}: mcp_control('approve', {name!r}) lets it run"
  if state != "connected":
    return line
  tools = [str(x["name"]) for x in server.get("tools") or []]
  more = f", and {len(tools[12:])} more" if tools[12:] else ""
  parts = [f"{len(tools)} tools ({', '.join(tools[:12])}{more})" if tools else "no tools"]
  sorts = (
    (len(server.get("resources") or []) + len(server.get("templates") or []), "resources"),
    (len(server.get("prompts") or []), "prompts"),
  )
  parts += [f"{count} {noun}" for count, noun in sorts if count]
  said = " ".join(str(server.get("instructions") or "").split())
  return f"{line}, {', '.join(parts)}" + (f". Instructions: {said[:400]}" if said else "")


def mcp_closed(got: object, show: Show) -> dict:
  if isinstance(got, list):
    return {"servers": "\n".join(mcp_line(x) for x in got)}
  if isinstance(got, dict) and "name" in got:
    return {"server": mcp_line(got)}
  if isinstance(got, dict):
    images = [f"![image]({x['image']})" for x in got.get("content") or [] if x.get("type") == "image"]
    lines = [shown(Text("", str(got.get("text") or "")), show), *images]
    return {"text": "\n".join(x for x in lines if x)}
  return {}


def mcp_act(kind: str, on: str, of: str, opened: dict, show: Show, *words: object) -> Act:
  def ear(id: str):
    yield told(id, "", bound(id, of), **opened)
    while True:
      match (yield):
        case ("done", about, _, got) if about == id:
          if not isinstance(got, BaseException):
            yield told(id, "done", **mcp_closed(got, show))
          return

  return act(kind, on, controlled(ear, lasting=True), *words)


def mcp_made[T](_shape: type[T], made) -> Act[T]:
  return made


def mcp_known(server: str, on: str = "") -> dict:
  held = answer("mcp_servers", on, [])
  for one in held:
    if one["name"] == server:
      return one
  why = f"No server {server} is known. The servers are: {', '.join(str(x['name']) for x in held) or 'none'}."
  raise Refused(why)


def mcp_named(word: str) -> str:
  at = ("\n" + word).find("\nclass ")
  if at < 0:
    return ""
  end = at + 6
  while end < len(word) and (word[end].isalnum() or word[end] == "_"):
    end += 1
  return word[at + 6 : end]


def mcp_bound(server: dict, on: str = "") -> str:
  word = str(server.get("binding") or "")
  name = mcp_named(word)
  if not name:
    return word
  calls = [f"mcp_call({json.dumps(server['name'])}", "mcp_call('" + str(server["name"]) + "'"]
  held = module(on)
  words = list(program(on).values())

  def own(cls: str) -> bool:
    return any((f"class {cls}:" in w or f"class {cls}(" in w) and any(c in w for c in calls) for w in words)

  free, n = name, 1
  while free in held and not own(free):
    n += 1
    free = f"{name}_{n}"
  at = ("\n" + word).find("\nclass ") + 6
  return word if free == name else word[:at] + free + word[at + len(name) :]


def mcp_servers(on: str = "") -> Act[list[dict]]:
  return mcp_made(list, mcp_act("mcp_servers", on, "list[dict]", {}, HEAD))


def mcp_call(server: str, tool: str, arguments: dict | None = None, show: Show = HEAD, on: str = "") -> Act[McpResult]:
  given = dict(arguments or {})
  opened = {"server": server, "tool": tool, **({"arguments": given} if given and not repr(given)[200:] else {})}
  return mcp_made(dict, mcp_act("mcp_call", on, "McpResult", opened, show, server, tool, given))


def mcp_read(server: str, uri: str, show: Show = HEAD, on: str = "") -> Act[McpResult]:
  return mcp_made(dict, mcp_act("mcp_read", on, "McpResult", {"server": server, "uri": uri}, show, server, uri))


def mcp_prompt(
  server: str, name: str, arguments: dict | None = None, show: Show = HEAD, on: str = ""
) -> Act[McpResult]:
  given = {str(k): str(v) for k, v in (arguments or {}).items()}
  opened = {"server": server, "name": name, **({"arguments": given} if given else {})}
  return mcp_made(dict, mcp_act("mcp_prompt", on, "McpResult", opened, show, server, name, given))


def mcp_control(action: str, server: str, on: str = "") -> Act[dict]:
  if action not in ("reconnect", "enable", "disable", "approve", "login", "logout"):
    why = f"No action {action}: use reconnect, enable, disable, approve, login or logout."
    raise Refused(why)
  return mcp_made(dict, mcp_act("mcp_control", on, "dict", {"action": action, "server": server}, HEAD, action, server))


def mcp_tools(server: str, on: str = "") -> Act[str]:
  def ear(id: str):
    try:
      text = "\n".join(mcp_signature(x) for x in mcp_known(server, on).get("tools") or [])
    except Refused as why:
      yield told(id, "", server=server, error=str(why))
      yield "done", id, why
      return
    yield told(id, "", bound(id, "str"), server=server, tools=text or "none")
    yield "done", id, text

  return mcp_made(str, act("mcp_tools", on, ear, server))


def mcp_tool(server: str, name: str, on: str = "") -> Act[dict]:
  def dump(value: object) -> str | None:
    return json.dumps(value, indent=1) if value else None

  def ear(id: str):
    fault = None
    try:
      tools = mcp_known(server, on).get("tools") or []
    except Refused as why:
      tools, fault = [], why
    tool = next((x for x in tools if x["name"] == name), None)
    fault = fault or (None if tool else Refused(f"No tool {name} on {server}."))
    if fault or tool is None:
      yield told(id, "", server=server, tool=name, error=str(fault))
      yield "done", id, fault
      return
    yield told(
      id,
      "",
      bound(id, "dict"),
      server=server,
      tool=name,
      title=tool.get("title"),
      description=tool.get("description"),
      input=dump(tool.get("input")),
      output=dump(tool.get("output")),
      annotations=dump(tool.get("annotations")),
    )
    yield "done", id, tool

  return mcp_made(dict, act("mcp_tool", on, ear, server, name))


def mcp_use(server: str, on: str = "") -> Act:
  word = mcp_bound(mcp_known(server, on), on)
  if not word:
    why = f"{server} has no tools to bind."
    raise Refused(why)
  return rung(word, on=on)


def mcp_index(on: str = "") -> Act:
  def ear(id: str):
    yield "started", id
    seen: dict[str, tuple] = {}
    made: list[str] = []
    pending = act("mcp_servers", on, None)
    while True:
      if pending and over(pending):
        got, pending = peek(pending), ""
        if isinstance(got, list):
          now = {}
          for x in got:
            line, word = mcp_line(x), str(x.get("binding") or "")
            if x["state"] != "connected":
              word = ""
            elif word and len(word) <= BIND:
              word = mcp_bound(x, on)
              line += f". Class {mcp_named(word)}."
            elif word:
              line += f". mcp_use({x['name']!r}) binds its class."
              word = ""
            now[x["name"]] = (line, word)
          lines = [v[0] for k, v in now.items() if seen.get(k, ("", ""))[0] != v[0]]
          lines += [f"{k} is gone" for k in seen if k not in now]
          if lines:
            yield told(id, "changed" if seen else "", servers="\n".join(lines))
          for name, (_, word) in now.items():
            if word and seen.get(name, ("", ""))[1] != word:
              made.append(rung(word, on=on))
          seen = now
      if not (pending or over(id)) and all(map(over, made)):
        yield "done", id, None
      a = yield {id, "mcp_changed", *made} | ({pending} if pending else set())
      if isinstance(peek(id), BaseException) or (a[0] in ("cancel", "close") and covers(a, id)):
        return
      pending = act("mcp_servers", on, None) if a[0] == "mcp_changed" else pending

  return act("mcp_index", on, controlled(ear, lasting=True))
