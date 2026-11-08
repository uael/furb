from furb.engine import (
  Act,
  Refused,
  act,
  automatic,
  cancel,
  chain,
  controlled,
  operated,
  over,
  peek,
  prompt,
  question,
  scope,
  transcript,
)

naming = """name_session() names the session once, at the first message of the operator. It asks a name of the actor that
automatic() gives, on a chain of its own labelled naming, and with no such actor, or
with no name from it, the name is the first line of that message. The act of name_session() is done with the name.
rename_session(name) says the name that the operator chose, and its name wins over the name of name_session().
A client shows the name of the session that its record holds. The extension plays on the root alone."""
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
    first = next((x for x in transcript(here, "prompt") if operated(x[2])), None)
    while first is None:
      said = yield {id, "prompt"}
      first = said if said[0] == "prompt" and operated(said[2]) else None
    message = str(first[5])
    given, n = None, 0
    # Each actor of automatic work in turn, the next when its model refuses the reply of its prompt.
    while given is None and (actor_of := automatic(n)):
      side = chain("naming", on=here)
      asked = prompt(str, NAME_BRIEF + message, to=actor_of, on=side)
      refused = False
      while not over(asked):
        match (yield {asked, side}):
          case ("done", about, _, Refused()) if question(("reply", about)) and scope(about) == side:
            refused = True
            cancel(asked)
      given = None if refused else peek(asked)
      n += 1
    yield "done", id, (isinstance(given, str) and name_of(given)) or name_of(message)

  return act("name_session", on, controlled(ear))


def rename_session(name: str, on: str = "") -> Act:
  def ear(id):
    yield "done", id, name.strip()

  return act("rename_session", on, ear)
