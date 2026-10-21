from furb.engine import HEAD, Act, Show, Text, act, bound, controlled, module, shown, told

web = """web_search(query) searches the web with the search of the provider of your actor, or with a free search when
it has none, and web_fetch(url) reads a page, as Markdown when it is HTML. Each gives an act of Text to await, which
tells its text with its show when it is done, as a read does."""


def text_act(kind: str, on: str, show: Show, word: str, *words: str) -> Act:
  def ear(id: str):
    yield told(id, "", bound(id, "Text"), **{word: words[0]})
    while True:
      match (yield):
        case ("done", about, _, got) if about == id:
          if isinstance(got, Text):
            yield told(id, "done", text=shown(got, show))
          return

  return act(kind, on, controlled(ear, lasting=True), *words)


def web_search(query: str, to: str = "", show: Show = HEAD, on: str = "") -> Act:
  return text_act("web_search", on, show, "query", query, to or str(module(on).get("actor", "")))


def web_fetch(url: str, show: Show = HEAD, on: str = "") -> Act:
  return text_act("web_fetch", on, show, "url", url)
