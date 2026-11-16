"""The World part of the kit web: its process reads a page of a server of this machine for a life of the engine of
monty, as a fresh install runs it, on uv from the header of its module. The free search asks an address that no config
names, so its answers are proven on the function that makes them, with the module read from its file."""

import importlib.util
import json
import threading
from collections.abc import AsyncIterator, Iterator
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

import pytest

import furb
import furb_monty.engine
from conftest import STANDS, Dead, swapped
from furb import engine
from furb.engine import Refused, Text
from furb_monty import _monty

ROOT = Path(__file__).resolve().parents[2]
OFF = ("catchup", "memory", "skills", "compaction", "naming", "mcp")
"""OFF is each official kit that the life of a test turns off, so it runs the kit web alone."""

PAGES = {
  "/page": (
    "200 OK",
    "text/html",
    b"<html><head><title>T</title><style>p{}</style></head><body><h1>Rust</h1>\n\n\n"
    b"<p>Is <b>fast</b>.</p><script>x()</script></body></html>",
  ),
  "/bare": ("200 OK", "", b"<p>one</p><div><br><br><br></div><p>two</p>"),
  "/json": ("200 OK", "application/json", b'{"a": 1}'),
  "/plain": ("200 OK", "text/plain", b"plain  \n"),
  "/image": ("200 OK", "image/png", b"\x89PNG"),
}
"""PAGES is each page of the server of the tests by its path: its status, its kind and its body."""


class Pages(BaseHTTPRequestHandler):
  """The server of the tests: each page of PAGES, a page of text that tells no length and never ends at `/endless`,
  and 404 for any other path."""

  protocol_version = "HTTP/1.1"

  def do_GET(self) -> None:
    if self.path == "/endless":
      self.send_response(200)
      self.send_header("content-type", "text/plain")
      self.send_header("transfer-encoding", "chunked")
      self.end_headers()
      chunk = b"100000\r\n" + b"a" * (1 << 20) + b"\r\n"
      try:
        while True:
          self.wfile.write(chunk)
      except OSError:
        # The client is gone: the connection takes no next request, whose read Windows refuses with a reset.
        self.close_connection = True
        return
    status, kind, body = PAGES.get(self.path, ("404 Not Found", "text/plain", b"none"))
    self.send_response(int(status.split()[0]))
    if kind:
      self.send_header("content-type", kind)
    self.send_header("content-length", str(len(body)))
    self.send_header("connection", "close")
    self.end_headers()
    self.wfile.write(body)

  def log_message(self, format: str, *args: object) -> None:
    del format, args


@pytest.fixture(autouse=True)
def on_monty() -> Iterator[None]:
  """Every name this module reads the engine by, bound to the engine of monty for the length of the test."""
  swapped(furb_monty.engine)
  yield
  swapped(furb.python)


@pytest.fixture
def served() -> Iterator[str]:
  """The address of the server of the tests, which serves until the end of the test."""
  server = ThreadingHTTPServer(("127.0.0.1", 0), Pages)
  server.daemon_threads = True
  threading.Thread(target=server.serve_forever, daemon=True).start()
  yield f"http://127.0.0.1:{server.server_address[1]}"
  server.shutdown()
  server.server_close()


@pytest.fixture
async def root(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> AsyncIterator[str]:
  """The root of a life on the ears of the crate, with the kit web alone, whose ears let go at the end of the test."""
  # The kit runs as on a fresh install: on uv, from the header of its module, and on no Python that the door names.
  monkeypatch.delenv("FURB_PYTHON", raising=False)
  config, project = tmp_path / "config", tmp_path / "project"
  config.mkdir()
  project.mkdir()
  skills = {**dict.fromkeys(OFF, False), "web": True}
  (config / "config.json").write_text(json.dumps({"skills": skills}), encoding="utf-8")
  stored, ears = _monty.opened(
    directory=str(project),
    record=None,
    keeps=False,
    config=str(config),
    images=str(tmp_path / "images"),
    claude=str(tmp_path / "no-claude"),
    roster=[],
  )
  world = Dead(stands=[STANDS[0], str(project), STANDS[2]]).hears()
  ears = list(ears)
  yield furb_monty.engine.boot(list(stored), **dict(ears), world=world)
  for _, one in ears:
    one.dispose()


async def fetched(root: str, url: str) -> Text:
  """The text that a fetch of an address on the root comes to."""
  got = await engine.rung(f"close(await act('fetch', '', None, {url!r}))", on=root)
  assert isinstance(got, Text), got
  return got


async def refused(root: str, url: str) -> str:
  """Why a fetch of an address on the root is refused."""
  with pytest.raises(Refused) as no:
    await fetched(root, url)
  return str(no.value)


async def test_a_page_of_html_is_its_markdown_other_text_is_itself_and_a_page_of_no_text_is_refused(
  served: str, root: str
) -> None:
  assert await fetched(root, f"{served}/page") == Text(f"{served}/page", "# Rust\n\nIs **fast**.")
  assert await fetched(root, f"{served}/json") == Text(f"{served}/json", '{"a": 1}')
  assert await fetched(root, f"{served}/plain") == Text(f"{served}/plain", "plain  \n")
  assert await refused(root, f"{served}/image") == f"{served}/image is no text: image/png"


async def test_a_page_of_no_kind_that_opens_with_a_tag_is_html_with_no_run_of_empty_lines(
  served: str, root: str
) -> None:
  assert await fetched(root, f"{served}/bare") == Text(f"{served}/bare", "one\n\ntwo")


async def test_a_page_that_is_none_or_larger_than_the_bound_is_refused_with_why(served: str, root: str) -> None:
  assert await refused(root, f"{served}/gone") == f"{served}/gone answered 404 Not Found"
  assert await refused(root, f"{served}/endless") == f"{served}/endless is larger than {10 << 20} bytes"


def loaded() -> object:
  """The process of the kit web as a module, read from its file."""
  spec = importlib.util.spec_from_file_location("web_process", ROOT / ".agents" / "skills" / "web" / "process.py")
  assert spec is not None
  assert spec.loader is not None
  module = importlib.util.module_from_spec(spec)
  spec.loader.exec_module(module)
  return module


def test_the_free_search_answers_each_result_under_its_title_and_its_address_with_its_excerpts_cut() -> None:
  found = loaded().found  # ty: ignore[unresolved-attribute]
  listed = {
    "results": [
      {"url": "https://one", "title": "One", "excerpts": ["First.", "Second."]},
      {"url": "https://two", "title": "", "excerpts": ["x" * 2000]},
    ]
  }
  cut = "## [https://two](https://two)\n\n" + "x" * 1500 + "…"
  assert found(json.dumps(listed)) == f"## [One](https://one)\n\nFirst.\n\nSecond.\n\n{cut}"
  assert found('{"results": []}') == "No result."


def test_the_free_search_gives_a_text_that_is_no_list_as_it_is() -> None:
  assert loaded().found("Plain.") == "Plain."  # ty: ignore[unresolved-attribute]
