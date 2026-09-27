from furb.engine import HEAD, Show, Text, read


def skills(on: str = "") -> Text:
  return read("skills://", on=on)


def skill(name: str, show: Show = HEAD, on: str = "") -> Text:
  return read(f"skills://{name}", show, on)
