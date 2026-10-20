from furb.engine import WINDOW, Act, act, controlled, get, offered, peek, question, scope, standing, told, transcript

SHARE = 0.6


def compaction(on: str = "") -> Act:
  def ear(id):
    yield "started", id
    here = scope(id)
    while True:
      match (yield):
        case ("done", about, _, (_, _, (int() as tokens, *_), _)) if (
          question(("reply", about)) and scope(about) == here
        ):
          roster = standing()[0]
          assert isinstance(roster, list)
          fill = tokens / (offered(roster, str(get(about)[4])) or WINDOW)
          if fill > SHARE:
            old = [x[1] for x in transcript(here) if x[0] == "prompt" and x[3] == here and peek(x[1], ...) is not ...]
            kept = ", ".join([*map(repr, old), "inside=False"])
            yield told(
              id, "filled", share=round(fill, 2), word=f"chain('compacted', source={here!r}, filter=take({kept}))"
            )
            while True:
              yield

  return act("compaction", on, controlled(ear))
