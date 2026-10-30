from furb.engine import HEAD, Act, Show

BIND: int
"""BIND is the most characters that the binding of a server may hold for mcp_index to bind its class at once, and it is 3000 unless a word rebinds it."""
mcp: str
"""mcp is the doctrine of the extension, a quote that says what the index tells and what each of mcp_tools, mcp_tool, mcp_use, mcp_call, mcp_read, mcp_prompt, mcp_servers and mcp_control does."""

type McpResult = dict
"""McpResult is what a tool, a resource and a prompt give: a dict whose key text holds the text of its blocks joined, and whose other keys hold the answer of the World, the content, the structured result or the messages."""

def mcp_type(schema: object) -> str:
  """mcp_type(schema) is the Python type that a JSON Schema asks, written as text: str, int, float, bool or None for a type of that name, list of its items for an array, dict[str, Any] for an object, a union for anyOf, oneOf or a list of types, and Literal of the values for an enum or a const.
  A schema that asks no type that Python can write is Any.
  """

def mcp_signature(tool: dict) -> str:
  """mcp_signature(tool) is one line for a tool: its name, its arguments in parentheses, and after a colon the first sentence of its description, cut at 200 characters.
  Its required arguments come first, each as name: type, and the others after them, each as name: type | None = None, under the original names of the schema.
  """

def mcp_line(server: dict) -> str:
  """mcp_line(server) is one line for a server: its name and its state, and for a connected server the count of its tools and the names of the first 12, the count of its resources and templates, the count of its prompts, and its instructions cut at 400 characters.
  A failed server adds its error, and a server in the state needs-auth or unapproved adds the mcp_control call that mends it.
  """

def mcp_closed(got: object, show: Show) -> dict:
  """mcp_closed(got, show) gives the words that the done of an mcp act tells, as a dict, and an empty dict for an answer that is none of the following.
  A list of servers tells under the word servers one line for each server, and a server tells its line under the word server, as mcp_line gives them.
  A result tells under the word text the lines of its text that its show picks, and one line of Markdown for each image block of its content, which shows the image.
  """

def mcp_act(kind: str, on: str, of: str, fields: dict, show: Show, *words: object) -> Act:
  """mcp_act(kind, on, of, fields, show, *words) is the act that every mcp act of the World shares: it puts the words to the World, its open tells the words of fields and binds the act to Act of the type that of names, and its done tells what mcp_closed gives for the answer.
  An act that the World refuses completes with Refused, which an await raises in the caller, and its done tells nothing.
  """

def mcp_made[T](_shape: type[T], made: Act) -> Act[T]:
  """mcp_made(shape, made) gives the act that it is given, typed as an act of the shape that a word awaits for, since the gate reads each act that the engine gives as an act of object."""

def mcp_known(server: str, on: str = "") -> dict:
  """mcp_known(server) gives the dict of a server in the last list of servers that an mcp_servers act on the chain came to.
  A server that the list does not hold raises Refused, which names the servers that it holds.
  """

def mcp_named(word: str) -> str:
  """mcp_named(word) gives the name of the first class that a word defines at the start of a line, and an empty text when the word defines none."""

def mcp_bound(server: dict, on: str = "") -> str:
  """mcp_bound(server) gives the binding of a server, and an empty text when it has none.
  A class name that the module of the chain binds already gets the suffix _2, then _3, in the class line of the binding, unless a word of the chain defined that class as the binding of this server.
  """

def mcp_servers(on: str = "") -> Act[list[dict]]:
  """mcp_servers() is an act that lists the servers, which a word awaits for a list of dicts, one for each server of the configs.
  It puts the question to the World with no words, its open binds the act to Act[list[dict]], and its done tells under the word servers one line for each server.
  A list that the World refuses completes with Refused, and its done tells nothing.
  """

def mcp_call(server: str, tool: str, arguments: dict | None = None, show: Show = HEAD, on: str = "") -> Act[McpResult]:
  """mcp_call(server, tool, arguments) is an act that calls a tool of a server under the original names of its arguments, which a word awaits for an McpResult.
  It puts the server, the tool and the arguments to the World, and the arguments are an empty dict when it is given none.
  Its open tells the server and the tool, and the arguments when their text holds at most 200 characters, and binds the act to Act[McpResult]; its done tells under the word text the lines of the text of the result that its show picks, and a line of Markdown for each image.
  A result with isError, a tool that the configs deny, a server that is not connected and a call past its timeout complete with Refused, which an await raises in the caller, and its done tells nothing.
  """

def mcp_read(server: str, uri: str, show: Show = HEAD, on: str = "") -> Act[McpResult]:
  """mcp_read(server, uri) is an act that reads a resource of a server, which a word awaits for an McpResult.
  It puts the server and the uri to the World, its open tells them and binds the act to Act[McpResult], and its done tells under the word text the lines of the text that its show picks.
  A read that the World refuses completes with Refused, and its done tells nothing.
  """

def mcp_prompt(
  server: str, name: str, arguments: dict | None = None, show: Show = HEAD, on: str = ""
) -> Act[McpResult]:
  """mcp_prompt(server, name, arguments) is an act that expands a prompt of a server, which a word awaits for an McpResult.
  It puts the server, the name and the arguments to the World, each argument as text and an empty dict when it is given none; its open tells them and binds the act to Act[McpResult], and its done tells under the word text the lines of the text that its show picks.
  A prompt that the World refuses completes with Refused, and its done tells nothing.
  """

def mcp_control(action: str, server: str, on: str = "") -> Act[dict]:
  """mcp_control(action, server) is an act that does an action to a server, which a word awaits for the dict of that server after the action: reconnect, enable, disable, approve, login or logout.
  It puts the action and the server to the World, its open tells them and binds the act to Act[dict], and its done tells the line of the server under the word server.
  Any other action raises Refused before the call makes an act.
  An action that the World refuses completes with Refused, and its done tells nothing.
  """

def mcp_tools(server: str, on: str = "") -> Act[str]:
  """mcp_tools(server) is an act that tells the signatures of the tools of a server, one line each as mcp_signature gives it, and completes with the same text.
  It asks the World nothing: it reads the last list of servers that the chain holds, so a server that the list does not hold completes it with Refused, which its open tells.
  """

def mcp_tool(server: str, name: str, on: str = "") -> Act[dict]:
  """mcp_tool(server, name) is an act that tells the title, the description, the input schema, the output schema and the annotations of a tool, and completes with the dict of the tool.
  It asks the World nothing: a server or a tool that the last list does not hold completes it with Refused, which its open tells.
  """

def mcp_use(server: str, on: str = "") -> Act:
  """mcp_use(server) is a rung of the chain that runs the binding of a server as mcp_bound gives it, so the class of its tools stands in the module for the next word.
  A server that the last list does not hold, or that has no binding, raises Refused.
  """

def mcp_index(on: str = "") -> Act:
  """mcp_index() is the life word of the extension, as await mcp_index(): an act on the chain that asks the World for the servers when it is made and at each fact mcp_changed, and that completes once it told the first answer of the World and ran each binding of it, so a word that awaits it holds what it tells, and it goes on after that.
  It tells the lines of the servers, as mcp_line gives them, once, and after a change only the lines that changed, with name is gone for a server that left; it tells nothing when the list is empty or the World refuses it.
  It runs at once, as a rung of the chain, the binding of each connected server whose binding holds at most BIND characters, and the line of that server names its class; the line of a larger connected server names mcp_use.
  A cancel over it ends it, and a pause over it holds it until the wake.
  """
