from furb.engine import HEAD, Act, Show, Text, act, controlled, opened, read, shown, told


def skills(on: str = "") -> Text:
  return read("skills://", on=on)


def skill(name: str = "", show: Show = HEAD, on: str = "") -> Act[Text]:
  def ear(id: str):
    yield opened(id, "Text", name=name)
    while True:
      match (yield):
        case ("done", about, _, got) if about == id:
          if isinstance(got, Text):
            yield told(id, "done", text=shown(got, show))
          return

  return skill_made(Text, act("skill", on, controlled(ear, lasting=True), name))


def skill_made[T](_shape: type[T], made) -> Act[T]:
  return made


async def listed(on: str = "") -> None:
  skills(on)
