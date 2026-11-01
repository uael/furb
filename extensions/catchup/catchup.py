from furb.engine import (
  HIDDEN,
  ROOT,
  Act,
  Refused,
  Text,
  act,
  differs,
  get,
  headed,
  peek,
  question,
  read,
  scope,
  transcript,
)

CAP = 40
BUDGET = 4096


def catchup(on: str = "") -> Act:
  def ear(id):
    yield "started", id
    here = scope(id)
    if here == ROOT:
      act("clock", here, None)
    while True:
      match (yield {id, "stand"}):
        case ("done", about, *_) if question(("stand", about)) and not question(("rung", get(about)[2])):
          if here == ROOT:
            act("clock", here, None)
          seen: dict[str, object] = {}
          for fact in transcript(here)[::-1]:
            if fact[0] not in ("read", "write"):
              continue
            was = peek(fact[1])
            path = was.path if isinstance(was, Text) else str(fact[4])
            if path in seen:
              continue
            seen[path] = None
            if isinstance(was, Text) and "://" not in path and get(path.split("/")[0]) is None:
              try:
                now = read(path, HIDDEN, here).lines
              except Refused:
                seen[path] = "gone"
                continue
              lines = {i: now[i - 1] for i in differs(was.lines)(now)}
              lines |= dict.fromkeys(range(len(now) + 1, len(was.lines) + 1))
              seen[path] = len(lines) if len(lines) > CAP else lines or None
          clocks = [x for one in transcript(ROOT, "clock") if isinstance(x := peek(one[1]), float)][-2:] or [0.0]
          hours = (clocks[-1] - clocks[0]) / 3600
          changed: dict | list = {key: value for key, value in seen.items() if value}
          kept = len(changed)
          while True:
            paragraph = headed(here, "caught up", hours=round(hours, 1) if hours > 1 else None, changed=changed or None)
            if len(paragraph.encode()) <= BUDGET:
              break
            changed = [*changed][:kept]
            kept -= 1
          if "\n" in paragraph:
            yield "tell", here, [paragraph]

  return act("catchup", on, ear)
