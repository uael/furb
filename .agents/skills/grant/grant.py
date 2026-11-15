from furb.engine import Act, Refused, act, close, controlled, filled, opened, pause, question, scope, told, transcript


def grant(usd: float | None = None, share: float | None = None, on: str = "") -> Act:
  def ear(id: str):
    here = scope(id)
    if not ((usd or share) and (usd or 0) >= 0 <= (share or 0) <= 1):
      yield "done", id, Refused(f"{usd}/{share} no ceiling")
      return
    yield "started", id
    spent = 0
    for x in transcript(here, "grant"):
      x[3] != here or x[1] == id or close(None, x[1])
    yield opened(id, "None", usd=usd, share=share)
    while True:
      match (yield {id, here}):
        case ("done", about, _, (_, _, (*_, cost), _)) if question(("reply", about)):
          spent += cost
          yield told(id, "ledger", spent=spent)
          if (usd and spent >= usd) or (share and (filled(about) or 0) >= share):
            pause(here, "ceiling")

  return act("grant", on, controlled(ear), usd, share)
