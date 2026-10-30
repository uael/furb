"""web_fetch, a read of a page of the web."""

from pathlib import Path

import pytest

from conftest import of
from extensions.conftest import webbed
from furb import engine
from furb.engine import Refused, Text

URL = "https://rust-lang.org"
"""URL is the address of the page that the web of the suite holds."""

PAGE = Text(URL, "# Rust\n\nA language.")
"""PAGE is what the web of the suite answers a fetch of URL with."""


async def test_web_fetch_is_an_act_that_reads_the_page_at_an_address(tmp_path: Path) -> None:
  """web_fetch(url) is an act that reads the page at an address, which a word awaits for the page: a text at the path of that address, in Markdown when the page is HTML."""
  root, heard = webbed(tmp_path, {URL: PAGE})
  assert await engine.rung(f"close(await web_fetch({URL!r}))", on=root) == PAGE
  assert [a[4:] for a in heard] == [(URL,)]


async def test_its_open_tells_the_address_and_its_done_tells_the_lines_of_the_page(tmp_path: Path) -> None:
  """Its open tells the address, unless the word that made the act holds it, and binds the act to Act[Text], and its done tells under the word text the lines of the page that its show picks."""
  root, _ = webbed(tmp_path, {URL: PAGE})
  await engine.rung(f"await web_fetch('https://' + {URL[8:]!r}, span(-1, -1))", on=root)
  opened = f"#web_fetch1\nweb_fetch1_url = {URL!r}\nweb_fetch1: Act[Text] = Act('web_fetch1')"
  assert of(engine.turns(on=root), "web_fetch") == [opened, "#web_fetch1 done\nweb_fetch1_text = 'A language.'"]


async def test_a_fetch_that_the_world_refuses_completes_with_refused(tmp_path: Path) -> None:
  """A fetch that the World refuses completes with Refused, which an await raises in the caller, and its done tells nothing."""
  root, _ = webbed(tmp_path, {})
  with pytest.raises(Refused, match=r"No page answers https://rust-lang\.org\."):
    await engine.rung(f"await web_fetch({URL!r})", on=root)
  opened = f"#web_fetch1\nweb_fetch1_url = {URL!r}\nweb_fetch1: Act[Text] = Act('web_fetch1')"
  assert of(engine.turns(on=root), "web_fetch") == [opened]
