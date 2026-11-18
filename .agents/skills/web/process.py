# /// script
# requires-python = ">=3.14"
# dependencies = ["mcp>=2.3.0", "httpx2>=2.10", "markdownify>=1.2", "beautifulsoup4>=4.13"]
# ///
"""The World part of the kit web: the process that answers each fetch of a page, and each search that the provider of
its actor leaves, since that provider has no search of its own. uv installs what it needs from the header above, as
PEP 723 says.

A fetch answers the page at an address as a text at the path of that address: as Markdown when the page is HTML, and as
it is when it is other text. It refuses a page that is none, is no text, or is larger than `LARGEST` bytes. A search
asks the free search, the server of Parallel for agents, which needs no key and answers the protocol of MCP through the
official SDK, and answers each result under its title and its address, with its excerpts cut at `EXCERPT` characters.
Each refuses a work that takes more than `TIMEOUT` seconds."""

import asyncio
import json
from collections.abc import Awaitable, Callable

import httpx2
from bs4 import BeautifulSoup
from markdownify import MarkdownConverter
from mcp.client import Client
from mcp.client.streamable_http import streamable_http_client

from furb.skill import Fact, Skill

AGENT = "Mozilla/5.0 (compatible; furb; +https://github.com/uael/furb)"
# The kinds of a page that a fetch prefers.
ACCEPT = "text/html, text/markdown;q=0.9, text/plain;q=0.8, */*;q=0.5"
# The largest page that a fetch reads, in bytes.
LARGEST = 10 << 20
# The tags of a page that hold no text to read, which its Markdown leaves out.
SKIPPED = ("head", "script", "style", "noscript", "template", "svg", "iframe", "canvas")
# The kinds of a page that are text, beside HTML.
TEXTS = ("text/", "json", "xml", "javascript", "yaml", "toml")
# How long a fetch or a search may take, in seconds.
TIMEOUT = 300
FREE = "https://search.parallel.ai/mcp"
# The most of the excerpts of one result of the free search that an answer holds, in characters.
EXCERPT = 1500

type Json = dict[str, object]


class Unread(Exception):
  """Why a fetch or a search gives no text, as a person reads it."""


def text(path: str, content: str) -> Json:
  """A Text of furb, as the wire says it."""
  return {"is": "Text", "path": path, "content": content}


async def timed(work: Awaitable[Json]) -> Json:
  try:
    async with asyncio.timeout(TIMEOUT):
      return await work
  except TimeoutError:
    why = f"it took more than {TIMEOUT} seconds"
    raise Unread(why) from None


async def fetched(url: str) -> Json:
  """The page at an address, as a text at the path of that address."""
  headers = {"user-agent": AGENT, "accept": ACCEPT}
  async with httpx2.AsyncClient(headers=headers, timeout=60, follow_redirects=True) as client:
    try:
      async with client.stream("GET", url) as response:
        if not response.is_success:
          why = f"{url} answered {response.status_code} {response.reason_phrase}"
          raise Unread(why)
        kind = response.headers.get("content-type", "").lower()
        large = f"{url} is larger than {LARGEST} bytes"
        if int(response.headers.get("content-length") or 0) > LARGEST:
          raise Unread(large)
        # A page that tells no length, or tells one that is wrong, is read on only while it stays under the bound.
        body = bytearray()
        async for chunk in response.aiter_bytes():
          if len(body) + len(chunk) > LARGEST:
            raise Unread(large)
          body += chunk
    except httpx2.HTTPError as no:
      why = f"{url}: {no}"
      raise Unread(why) from no
  return text(url, page(url, kind, body.decode("utf-8", "replace")))


def page(url: str, kind: str, body: str) -> str:
  """The text of a page of a kind: its Markdown when it is HTML, the page itself when it is other text, and a page of no
  kind is HTML when it opens with a tag."""
  if "html" in kind or (not kind and body.lstrip().startswith("<")):
    return markdown(body)
  if not kind or any(one in kind for one in TEXTS):
    return body
  why = f"{url} is no text: {kind}"
  raise Unread(why)


def markdown(html: str) -> str:
  """The Markdown of a page of HTML, less the tags that hold no text to read, with no space at the end of a line and no
  run of more than one empty line."""
  soup = BeautifulSoup(html, "html.parser")
  for tag in soup(SKIPPED):
    tag.decompose()
  lines: list[str] = []
  for line in MarkdownConverter(heading_style="ATX").convert_soup(soup).splitlines():
    if (line := line.rstrip()) or (lines and lines[-1]):
      lines.append(line)
  while lines and not lines[-1]:
    lines.pop()
  return "\n".join(lines)


async def searched(query: str) -> Json:
  """The answer of the free search to a query, as a text at the path of the query."""
  http = httpx2.AsyncClient(timeout=httpx2.Timeout(60, read=TIMEOUT))
  async with Client(streamable_http_client(FREE, http_client=http), cache=None) as client:
    result = await client.call_tool("web_search", {"objective": query, "search_queries": [query]})
  said = "".join(getattr(one, "text", "") for one in result.content)
  if result.is_error:
    raise Unread(said)
  return text(query, found(said))


def found(said: str) -> str:
  """Each result that the free search listed under its title and its address, with its excerpts cut, or what it said
  when it listed none."""
  try:
    listed = json.loads(said)
  except json.JSONDecodeError:
    return said
  answers = []
  for one in listed.get("results", []) if isinstance(listed, dict) else []:
    url, title = one.get("url", ""), one.get("title", "")
    excerpts = "\n\n".join(one.get("excerpts", []))
    more = "…" if len(excerpts) > EXCERPT else ""
    answers.append(f"## [{title or url}]({url})\n\n{excerpts[:EXCERPT].strip()}{more}")
  return "\n\n".join(answers) or "No result."


def answering(welcome: object) -> dict[str, Callable[[Fact], Awaitable[Json]]]:
  del welcome
  return {"fetch": lambda fact: timed(fetched(str(fact[4]))), "search": lambda fact: timed(searched(str(fact[4])))}


async def main() -> None:
  skill = await Skill.attach(kinds=[], answering=answering)
  await skill.closed


if __name__ == "__main__":
  asyncio.run(main())
