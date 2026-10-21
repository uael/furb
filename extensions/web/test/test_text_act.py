"""text_act, an act of Text."""

from pathlib import Path

from conftest import of
from extensions.conftest import webbed
from furb import engine
from furb.engine import Text

URL = "https://rust-lang.org"
"""URL is the address of the page that the web of the suite holds."""

PAGE = Text(URL, "# Rust\n\nA language.")
"""PAGE is what the web of the suite answers a fetch of URL with."""


async def test_text_act_is_the_act_of_text_that_web_search_and_web_fetch_share(tmp_path: Path) -> None:
  """text_act(kind, on, show, word, *words) is the act of Text that web_search and web_fetch share: it puts the words to the World, its open tells the first of them under the word and binds the act to Act[Text], and its done tells under the word text the lines of the text that its show picks."""
  root, heard = webbed(tmp_path, {URL: PAGE})
  assert await engine.rung(f"close(await text_act('web_fetch', '', span(1, 1), 'page', {URL!r}))", on=root) == PAGE
  assert [a[4:] for a in heard] == [(URL,)]
  opened = f"#web_fetch1\nweb_fetch1_page = {URL!r}\nweb_fetch1: Act[Text] = Act('web_fetch1')"
  assert of(engine.turns(on=root), "web_fetch") == [opened, "#web_fetch1 done\nweb_fetch1_text = '# Rust'"]
