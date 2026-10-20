from furb.engine import HEAD, HIDDEN, Show, Text, read, tell


def skills(on: str = "") -> Text:
  return read("skills://", on=on)


def skill(name: str, show: Show = HEAD, on: str = "") -> Text:
  return read(f"skills://{name}", show, on)


def counted(on: str = "") -> int:
  got = len(read("skills://", HIDDEN, on).lines)
  tell(on, "skills://", f"{got} skills: skills() lists them")
  return got
