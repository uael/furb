from furb.engine import HEAD, Act, Show

web: str
"""web is the doctrine of the extension, a quote that says what web_search and web_fetch do, and that each gives an act of Text to await."""

def text_act(kind: str, on: str, show: Show, word: str, *words: str) -> Act:
  """text_act(kind, on, show, word, *words) is the act of Text that web_search and web_fetch share: it puts the words to the World, its open tells the first of them under the word and binds the act to Act[Text], and its done tells under the word text the lines of the text that its show picks."""

def web_search(query: str, to: str = "", show: Show = HEAD, on: str = "") -> Act:
  """web_search(query) is an act that searches the web for the query, which a word awaits for the answer: a text in Markdown that ends with its sources.
  It puts the query and the actor to the World: to, or else the actor of its chain, so the World searches with the native search of the provider of that actor, or with a free search when that provider has none.
  Its open tells the query and binds the act to Act[Text], and its done tells under the word text the lines of the answer that its show picks.
  A search that the World refuses completes with Refused, which an await raises in the caller, and its done tells nothing.
  """

def web_fetch(url: str, show: Show = HEAD, on: str = "") -> Act:
  """web_fetch(url) is an act that reads the page at an address, which a word awaits for the page: a text at the path of that address, in Markdown when the page is HTML.
  Its open tells the address and binds the act to Act[Text], and its done tells under the word text the lines of the page that its show picks.
  A fetch that the World refuses completes with Refused, which an await raises in the caller, and its done tells nothing.
  """
