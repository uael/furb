from furb.engine import Act, Text, act, ask, controlled, differs, get, question, scope, span, tell, tells, transcript


def memory(path: str = ".", on: str = "") -> list[Text]:
  was: dict[str, list[str]] = {}
  for x in transcript(on):
    match x:
      case ("done", about, _, list(texts)) if (
        question(("memory", about)) and get(maker := get(about)[2]) and tells(maker)
      ):
        for one in texts:
          was[one.path] = one.lines
      case ("done", about, _, Text(place, content)) if (
        (question(("read", about)) or question(("write", about))) and get(maker := get(about)[2]) and tells(maker)
      ):
        was[place] = content.splitlines()
  got = ask("memory", on, path)
  assert isinstance(got, list)
  for one in got:
    tell(on, one.path, one, differs(was[one.path]) if one.path in was else span(1, -1))
  return got


def remember(on: str = "") -> Act:
  def ear(id):
    yield "started", id
    memory(on=on)
    while True:
      match (yield {id, "read", "stand"}):
        case ("done", about, _, Text(path)) if (
          question(("read", about)) and scope(about) == scope(id) and "://" not in path
        ):
          memory(path, on)
        case ("done", about, *_) if question(("stand", about)):
          memory(on=on)

  return act("remember", on, controlled(ear))
