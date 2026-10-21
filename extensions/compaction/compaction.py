from furb.engine import Act, act, controlled, filled, peek, question, scope, told, transcript

SHARE = 0.6


def compaction(on: str = "") -> Act:
  def ear(id):
    yield "started", id
    here = scope(id)
    while True:
      match (yield):
        case ("done", about, _, (_, _, (int(), *_), _)) if question(("reply", about)) and scope(about) == here:
          if (fill := filled(about)) > SHARE:
            old = [x[1] for x in transcript(here) if x[0] == "prompt" and x[3] == here and peek(x[1], ...) is not ...]
            kept = ", ".join([*map(repr, old), "inside=False"])
            yield told(id, "filled", share=fill, word=f"chain('compacted', source={here!r}, filter=take({kept}))")
            while True:
              yield

  return act("compaction", on, controlled(ear))
