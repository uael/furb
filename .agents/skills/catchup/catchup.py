from furb.engine import (
  HIDDEN,
  ROOT,
  Act,
  Refused,
  Text,
  act,
  acting,
  differs,
  get,
  headed,
  listening,
  peek,
  picked,
  question,
  read,
  scope,
  transcript,
)

CAP = 40
BUDGET = 4096


def catchup(on: str = "") -> Act | None:
  def ear(id):
    yield "started", id
    here = scope(id)
    act("clock", here, None)
    while True:
      match (yield {id, "stand"}):
        case ("done", about, *_) if question(("stand", about)) and not question(("rung", get(about)[2])):
          act("clock", here, None)
          clocks = [x for one in transcript(ROOT, "clock") if isinstance(x := peek(one[1]), float)][-2:] or [0.0]
          hours = (clocks[-1] - clocks[0]) / 3600
          # The lines of each path as the catch-up read them again at this stand, or None when the read was refused.
          now: dict[str, list[str] | None] = {}
          for c in [x for x in listening() if question(("chain", x)) and scope(x) == x]:
            if paragraph := caught(c, now, hours):
              yield "tell", c, [paragraph]

  def caught(c: str, now: dict[str, list[str] | None], hours: float) -> str:
    seen: dict[str, object] = {}
    for fact in transcript(c)[::-1]:
      if fact[0] not in ("read", "write") or (fact[0] == "read" and str(fact[4]).endswith(("/", "\\"))):
        continue
      was = peek(fact[1])
      path = was.path if isinstance(was, Text) else str(fact[4])
      if path in seen:
        continue
      seen[path] = None
      if isinstance(was, Text) and "://" not in path and get(path.split("/")[0]) is None:
        if path in now and now[path] == was.lines:
          continue
        try:
          now[path] = lines = read(path, HIDDEN, c).lines
        except Refused:
          now[path] = None
          seen[path] = "gone"
          continue
        differing = {i: lines[i - 1] for i in picked(differs(was.lines), lines)}
        differing |= dict.fromkeys(range(len(lines) + 1, len(was.lines) + 1))
        seen[path] = len(differing) if len(differing) > CAP else differing or None
    changed: dict | list = {key: value for key, value in seen.items() if value}
    kept = len(changed)
    while True:
      paragraph = headed(c, "caught up", hours=round(hours, 1) if hours > 1 else None, changed=changed or None)
      if len(paragraph.encode()) <= BUDGET:
        break
      changed = [*changed][:kept]
      kept -= 1
    return paragraph if "\n" in paragraph else ""

  return act("catchup", on, ear) if scope(on or acting()) == scope(ROOT) else None
