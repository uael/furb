"""web_search, a search of the web."""

from pathlib import Path

import pytest

from conftest import of
from extensions.conftest import webbed
from furb import engine
from furb.engine import Refused, Text

ANSWER = Text("rust", "Rust is fast.\n\nSources:\n- [Rust](https://rust-lang.org)")
"""ANSWER is what the web of the suite answers a search for rust with."""


async def test_web_search_is_an_act_that_searches_the_web_for_the_query(tmp_path: Path) -> None:
  """web_search(query) is an act that searches the web for the query, which a word awaits for the answer: a text in Markdown that ends with its sources."""
  root, _ = webbed(tmp_path, {"rust": ANSWER})
  assert await engine.rung('close(await web_search("rust"))', on=root) == ANSWER


async def test_it_puts_the_query_and_the_actor_to_the_world(tmp_path: Path) -> None:
  """It puts the query and the actor to the World: to, or else the actor of its chain, so the World searches with the native search of the provider of that actor, or with a free search when that provider has none."""
  root, heard = webbed(tmp_path, {"rust": ANSWER})
  await engine.rung('await web_search("rust")\nawait web_search("rust", "n")', on=root)
  assert [a[4:] for a in heard] == [("rust", "m/low"), ("rust", "n")]


async def test_its_open_tells_the_query_and_its_done_tells_the_lines_of_the_answer(tmp_path: Path) -> None:
  """Its open tells the query, unless the word that made the act holds it, and binds the act to Act[Text], and its done tells under the word text the lines of the answer that its show picks."""
  root, _ = webbed(tmp_path, {"rust": ANSWER})
  await engine.rung('await web_search("ru" + "st", show=span(1, 1))', on=root)
  assert of(engine.turns(on=root), "web_search") == [
    "#web_search1\nweb_search1_query = 'rust'\nweb_search1: Act[Text] = Act('web_search1')",
    "#web_search1 done\nweb_search1_text = 'Rust is fast.'",
  ]


async def test_a_search_that_the_world_refuses_completes_with_refused(tmp_path: Path) -> None:
  """A search that the World refuses completes with Refused, which an await raises in the caller, and its done tells nothing."""
  root, _ = webbed(tmp_path, {})
  with pytest.raises(Refused, match=r"No page answers tea\."):
    await engine.rung('await web_search("tea")', on=root)
  assert of(engine.turns(on=root), "web_search") == ["#web_search1\nweb_search1: Act[Text] = Act('web_search1')"]
