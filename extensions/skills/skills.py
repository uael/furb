from furb.engine import HEAD, Show, Text, module, read, rung


def skills(on: str = "") -> Text:
  return read("skills://", on=on)


def skill(name: str, show: Show = HEAD, on: str = "") -> Text:
  return read(f"skills://{name}", show, on)


async def listed(on: str = "") -> None:
  await rung("read('skills://')", to=str(module(on).get("actor") or ""), on=on)
