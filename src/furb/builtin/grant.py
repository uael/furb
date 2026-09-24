WINDOW = 200000


def grant(usd: float | None = None, share: float | None = None, on: str = "") -> Act[None]:
  def ear(id):
    here = scope(id)
    if not (usd or share) or (usd or 0) < 0 or not 0 <= (share or 0) <= 1:
      yield "done", id, Refused(f"{usd}/{share} no ceiling")
      return
    spent = 0.0
    for old in [x for x in acts if x != id and question(("grant", x)) and scope(x) == here]:
      close(None, old)
    told(id, f"usd={usd} share={share}", bound(id, "None"))
    while True:
      match (yield):
        case ("answer", about, _, (_, _, (tokens, *_, dollars), _)) if scope(about) == here:
          spent += dollars
          filled = tokens / (offered(ask("stand", here, about)[1], acts[about][6]) or WINDOW)
          told(about, f"ledger spent={spent} filled={filled}")
          if (usd is not None and spent >= usd) or (share is not None and filled >= share):
            pause(here)

  return act("grant", on, ending(ear), usd, share)
