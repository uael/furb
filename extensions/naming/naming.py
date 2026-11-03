from furb.engine import Act, act, automatic, chain, controlled, operated, over, peek, prompt, question, scope

naming = """name_session() names the session once, at the first message of the operator. It asks a name of the actor that
automatic() gives, on a chain of its own labelled naming, and with no such actor, or
with no name from it, the name is the first line of that message. The act of name_session() is done with the name,
which the TUI shows in place of the name that it gave the session. The extension plays on the root alone."""
NAME_LONGEST = 60
NAME_BRIEF = """Give a name to a session of work, from the first message of its operator below: at most six words, in the
language of the message, that say what the work is, with no quotes and no period. Close with the name alone, as
close("Fix the login form").

The first message:
"""


def name_of(text: str) -> str:
  lines = [x.strip(" \t\"'`*#.") for x in text.splitlines()]
  return next((x[:NAME_LONGEST].rstrip() for x in lines if x), "")


def name_session(on: str = "") -> Act:
  def ear(id):
    yield "started", id
    here = scope(id)
    first = yield {id, "prompt"}
    while not (question(("prompt", first[1])) and operated(first[2])):
      first = yield {id, "prompt"}
    message = str(first[5])
    actor_of = automatic()
    given = None
    if actor_of:
      asked = prompt(str, NAME_BRIEF + message, to=actor_of, on=chain("naming", on=here))
      while not over(asked):
        yield {asked}
      given = peek(asked)
    yield "done", id, (isinstance(given, str) and name_of(given)) or name_of(message)

  return act("name_session", on, controlled(ear))
