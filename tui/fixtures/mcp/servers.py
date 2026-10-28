from collections.abc import Iterator

from furb.engine import Act, act

SERVERS = [
  {
    "name": "github",
    "scope": "project",
    "transport": "stdio",
    "target": "npx github-mcp",
    "state": "connected",
    "error": "",
    "version": "1.2.0",
    "instructions": "Use it for the issues of the project.",
    "tools": [
      {
        "name": "create_issue",
        "title": "Create an issue",
        "description": "Create a new issue.",
        "input": {
          "type": "object",
          "properties": {"title": {"type": "string", "description": "The title"}, "body": {"type": "string"}},
          "required": ["title"],
        },
        "output": None,
        "annotations": {"destructiveHint": True},
      }
    ],
    "resources": [{"uri": "repo://readme", "name": "README", "description": "The readme", "mime": "text/markdown"}],
    "templates": [],
    "prompts": [
      {
        "name": "review",
        "description": "Review a pull request",
        "arguments": [{"name": "pr", "description": "The number of the pull request", "required": True}],
      }
    ],
    "binding": "",
  },
  {
    "name": "slack",
    "scope": "user",
    "transport": "stdio",
    "target": "slack-mcp",
    "state": "failed",
    "error": "spawn slack-mcp ENOENT",
    "version": "",
    "instructions": "",
    "tools": [],
    "resources": [],
    "templates": [],
    "prompts": [{"name": "digest", "description": "A digest", "arguments": []}],
    "binding": "",
  },
]


def mcp_servers(on: str = "") -> Act:
  def ear(made: str) -> Iterator[tuple]:
    yield "done", made, SERVERS

  return act("mcp_servers", on, ear)


def mcp_control(action: str, server: str, on: str = "") -> Act:
  def ear(made: str) -> Iterator[tuple]:
    one = next(x for x in SERVERS if x["name"] == server)
    yield "done", made, {**one, "state": "connected", "error": ""}

  return act("mcp_control", on, ear, action, server)


def mcp_prompt(server: str, name: str, arguments: dict | None = None, on: str = "") -> Act:
  def ear(made: str) -> Iterator[tuple]:
    yield "done", made, {"text": f"Review the pull request {(arguments or {}).get('pr')} of {server}.", "messages": []}

  return act("mcp_prompt", on, ear, server, name, arguments or {})


def mcp_call(server: str, tool: str, arguments: dict | None = None, on: str = "") -> Act:
  def ear(made: str) -> Iterator[tuple]:
    yield "done", made, {"text": "Created the issue #7.", "content": [], "structured": None}

  return act("mcp_call", on, ear, server, tool, arguments or {})
