from furb.engine import Act, Text, act, ask, ending, pausing, question, scope, span, tell


def memory(path: str = ".", on: str = "") -> list[Text]:
  got = ask("memory", on, path)
  assert isinstance(got, list)
  for one in got:
    tell("memory", one.path, (one, span(1, -1)))
  return got


def remember(on: str = "") -> Act:
  def ear(id):
    yield "started", id
    memory(on=on)
    while True:
      match (yield):
        case ("done", about, _, Text(path)) if (
          question(("read", about)) and scope(about) == scope(id) and "://" not in path
        ):
          memory(path, on)
        case ("done", about, *_) if question(("stand", about)):
          memory(on=on)

  return act("remember", on, pausing(ending(ear)))
