from furb.engine import Act, act, module, question, scope, told, transcript

LOOKING = """Work in two words at least, since you see no file until a word shows it.
Your first word looks: read(path) shows each file of the task whole, and bash(command) runs its check. It ends with no
close, and the chain asks you again with all that it showed.
A later word changes the files from the lines that you saw, runs the check again, and closes.
Never guess what a file holds, and never hide what you must see."""


def look(on: str = "") -> Act:
  def ear(id):
    here = scope(id)
    local = str(module(here).get("actor", "")).startswith("local:")
    if local and not any(x[0] == "tell" and question(("look", x[1])) for x in transcript(here)):
      yield told(id, "", lore=LOOKING)
    yield "done", id, None

  return act("look", on, ear)
