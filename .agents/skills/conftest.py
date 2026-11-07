"""The conftest of the suites of the extensions: the hooks and the fixtures of the harness of the engine, which reach
the suite of each extension only through a conftest above it, and the helpers that only those suites need."""

import ast
import asyncio
import json
import os
import re
import subprocess
import sys
import tempfile
from collections.abc import Callable, Generator, Mapping, Sequence
from dataclasses import dataclass, field, replace
from functools import partial
from pathlib import Path

import pytest

import conftest
from conftest import (  # noqa: F401
  STANDS,
  Sand,
  World,
  engine_of,
  life,
  of,
  paragraphs,
  pytest_generate_tests,
  settle,
  sown,
  tilded,
  world_says,
)
from furb import engine
from furb.engine import Exit, Refused, Text
from furb_monty import _monty

USE = (
  "skill(name) loads a skill: a skill with code plays its word and tells its interface, and any other tells its"
  ' SKILL.md. skill() plays each default skill. read("skills://<name>") gives the word of a skill with code, and'
  ' read("skills://<name>/<file>") a file of its folder. read("settings://<name>") gives the keys of a skill in the'
  " configs, as JSON."
)
"""USE is the first line of the list of the skills, which says how a chain loads and reads a skill."""


def official_lines(on: Sequence[str]) -> list[str]:
  """The line of the list of the skills of each official extension, as its manifest gives it, with on for each one in
  on."""
  made, official = [], {one["name"] for one in _monty.official()}
  for manifest in sorted(Path(__file__).parent.glob("*/SKILL.md")):
    if manifest.parent.name not in official:
      continue
    text = manifest.read_text(encoding="utf-8")
    name, said = (re.search(rf"^{key}: (.+)$", text, re.M) for key in ("name", "description"))
    assert name, f"the manifest {manifest} gives a name"
    assert said, f"the manifest {manifest} gives a description"
    marks = ["code", *(["on"] if name[1] in on else []), *(["default"] if '  furb-default: "true"' in text else [])]
    made.append(f"{name[1]} ({', '.join(marks)}): {said[1]}")
  return made


def index(*lines: str, on: Sequence[str] = ("skills",)) -> list[str]:
  """The lines of the list of the skills of a chain that finds the skills of these lines in its folders: the first
  line, then these lines and the line of each official extension, with on for each one in on, in the order of the names,
  and a line that comes earlier wins for a name."""
  kept: dict[str, str] = {}
  for line in [*lines, *official_lines(on)]:
    kept.setdefault(re.split(r"[ :]", line, maxsplit=1)[0], line)
  return [USE, *(kept[name] for name in sorted(kept))]


SKILLED = (".furb/skills", ".agents/skills", ".claude/skills")
"""SKILLED is the folders of skills that the World finds in the working directory of a chain and in each folder above
it."""


def stray(folder: Path) -> bool:
  """Whether a folder above this one holds a folder of skills, whose skills the World finds for a chain that stands
  under it."""
  return any((one / place).is_dir() for one in folder.parents for place in SKILLED)


TOP = Path(tempfile.gettempdir()).anchor if sys.platform == "win32" else "/tmp"  # noqa: S108
"""TOP is the folder of the system that a test takes its temporary folder in when the one of pytest lies under skills:
/tmp, and on Windows, which has no /tmp, the root of the drive of the temporary folder."""


@pytest.fixture
def tmp_path(tmp_path: Path) -> Generator[Path]:
  """The temporary folder of a test, under no folder that holds skills: the one of pytest, or else a new one in TOP,
  as when the temporary folder of the system lies under a home that holds skills. So a test sees only the skills that
  it gives, and none of the machine."""
  if not stray(tmp_path):
    yield tmp_path
    return
  with tempfile.TemporaryDirectory(dir=TOP) as made:
    one = Path(made).resolve()
    assert not stray(one), f"a folder above {one} holds skills"
    yield one


@pytest.fixture(autouse=True)
def home(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Path:
  """The home of the user for a test: the folder above its temporary folder, on every system, so the World writes each
  path of the test with ~ in place of the home, as it writes a path of a project under the home of the user."""
  monkeypatch.setenv("HOME", str(tmp_path.parent))
  monkeypatch.setenv("USERPROFILE", str(tmp_path.parent))
  return tmp_path.parent


def extended(  # noqa: PLR0913
  name: str,
  at: Path,
  ear: Callable[[str], World] | None = None,
  *,
  lives: bool = False,
  record: Sequence[tuple] = (),
  sand: Sand | None = None,
  doctrine: str = "",
  also: Sequence[str] = (),
  roster: Sequence[Sequence[object]] = (),
) -> tuple[Sand, str]:
  """A life on a record, whose chains stand in the folder work of a directory, which enables at its tip the official
  extension of that name, with its life word when it lives: it hears the ear of the extension when it has one, whose
  config directory of the user is the folder config of the directory, and the files of the machine after the World
  of the suite, or after the World it is given. A standing that names a doctrine, which the folder holds as
  DOCTRINE.md with that text, and each extension that the life enables, as the World of a life gives them, sets up
  the root as a new record of this build does. Before it, the life enables each
  official extension that also names, with its life word. The life stands on the roster of the suite, or on the roster
  that it is given. It gives the World and the root."""
  one = next(x for x in _monty.official() if x["name"] == name)
  (at / "work").mkdir(exist_ok=True)
  before = [x for x in _monty.official() if x["name"] in also]
  enabled = [*before, one if lives else {**one, "life": ""}]
  given = _monty.played(enabled)
  taught = []
  if doctrine:
    (path := at / "DOCTRINE.md").write_text(doctrine, encoding="utf-8", newline="")
    plays = [x for x in enabled if x["word"].strip() or x["life"]]
    taught = [
      str(path),
      [[x["name"], str(dict(x)["file"]) if x["word"].strip() else "", x["life"], x["root"]] for x in plays],
      str(Path.home()),
    ]
  sand = replace(sand or sown(), stands=[list(roster) or STANDS[0], str(at / "work"), STANDS[2], *taught])
  files = _monty.files(str(at / "scratch"), str(at / "config"))
  return sand, life(
    sand, record, **{"extensions": given}, **({name: ear(str(at / "config"))} if ear else {}), files=files
  )[1]


def naming(
  at: Path, roster: Sequence[Sequence[object]] = (), record: Sequence[tuple] = (), *, lives: bool = True
) -> tuple[Sand, str]:
  """A life of a new record that runs the extension naming with its life word, or with none when lives is false, and no compaction, on a
  roster whose model h has the first place in the list of automatic work, or on the roster that it is given, and on
  the record that it is given: the World and the root."""
  given = roster or [*STANDS[0], ["h", ["low"], 200000, "cheap", 0]]
  return extended("naming", at, lives=lives, doctrine="Read first.\n", roster=given, record=record)


CATCHUP = next(x for x in _monty.official() if x["name"] == "catchup")
"""CATCHUP is the official extension catchup, with its life word."""


def number(skill: Mapping[str, object], name: str) -> int:
  """The number that the word of an extension binds to a name in capitals, as each chain binds it."""
  word = skill["word"]
  assert isinstance(word, str)
  got = next(
    ast.literal_eval(one.value)
    for one in ast.parse(word).body
    if isinstance(one, ast.Assign) and [ast.unparse(x) for x in one.targets] == [name]
  )
  assert isinstance(got, int)
  return got


async def catching(sand: Sand, record: Sequence[tuple] = ()) -> tuple[list[tuple], str]:
  """A life on a World of the suite and a record, which runs the extension catchup with its life word, once the loop
  gave it room to play the extension on the root: what was said in the life, and the root."""
  log, root = life(sand, record, **{"extensions": _monty.played([CATCHUP])})
  await settle(300)
  return log, root


async def caught_up(**world: object) -> tuple[Sand, list[tuple], str]:
  """A life on the World of the suite, which a test changes by keyword, that runs the extension catchup: the World,
  what was said in the life, and the root."""
  sand = sown(**world)
  log, root = await catching(sand)
  return sand, log, root


def caught(got: Sequence[tuple], chain: str) -> list[str]:
  """Each paragraph headed caught up that the turns of a chain hold, in order."""
  return [one for one in paragraphs(got) if one.startswith(f"#{chain} caught up")]


def noted(folder: Path, text: str, name: str = "AGENTS.md") -> Path:
  """A memory file in a folder, which it makes with the folders above it, and the path of that file."""
  folder.mkdir(parents=True, exist_ok=True)
  (path := folder / name).write_text(text, encoding="utf-8", newline="")
  return path


MEMORIES = ("AGENTS.md", "CLAUDE.md", "AGENTS.local.md", "CLAUDE.local.md")
"""MEMORIES is the names of the memory files of a folder."""


def recalled(chain: str, at: Path) -> list[str]:
  """Every paragraph that a read told a chain of a memory file under a directory or in the config directory, in order,
  since a folder above that directory belongs to the machine, each with read in place of the name of its read."""
  told = []
  for one in of(engine.turns(on=chain), "read"):
    head, _, value = one.split("\n")[1].partition(" = ")
    path = ast.literal_eval(value) if head.endswith("_path") else ""
    name = str(path).replace("\\", "/").rsplit("/", 1)[-1]
    if name in MEMORIES and str(path).startswith((tilded(at), "config://")):
      told.append(re.sub(r"\bread\d+", "read", one))
  return told


def memorized(path: Path | str, text: str) -> str:
  """The paragraph that a read tells of one memory file, with read in place of the name of the read: its header, the
  binding of the path of the file, and the binding of its text, as a quote when it holds more than one line."""
  lines = text.splitlines()
  told = f"<s:read_text>\n{'\n'.join(lines)}</s:read_text>" if len(lines) > 1 else f"read_text = {lines[0]!r}"
  return f"#read\nread_path = {path if isinstance(path, str) else tilded(path)!r}\n{told}"


def skilled(skills: Path, folder: str, head: str) -> Path:
  """A skill in a folder of skills, whose SKILL.md file opens with a frontmatter of these lines, and the path of that
  file."""
  (skills / folder).mkdir(parents=True, exist_ok=True)
  (path := skills / folder / "SKILL.md").write_text(f"---\n{head}\n---\nSteps.\n", encoding="utf-8", newline="")
  return path


FULL = (300000, 0, 0, 0, 0.0)
"""An answer that reads three quarters of the window of the default actor of the suite, which is 400000 tokens."""


def compacting(at: Path) -> tuple[Sand, str, str]:
  """A life that runs the extension compaction, whose folder work holds a.txt: the World, the root and the folder."""
  sand, root = extended("compaction", at)
  work = str(at / "work")
  sand.files[f"{work}/a.txt"] = "one\ntwo\n"
  return sand, root, work


def surfed(answers: dict[str, Text], heard: list[tuple]) -> World:
  """The web of the suite, in memory: an ear that takes each search and each fetch, keeps it in heard, says it
  started, and says it done from its own loop with the answer of its query or its address, or with a refusal when it
  has none."""
  while True:
    a = yield
    if a[0] in ("search", "fetch") and engine.question(a):
      heard.append(a)
      yield "started", a[1]
      got = answers.get(a[4], Refused(f"No page answers {a[4]}."))
      asyncio.get_running_loop().call_soon(world_says, "done", a[1], got)


def webbed(at: Path, answers: dict[str, Text]) -> tuple[str, list[tuple]]:
  """A life that runs the extension web, on the web of the suite with these answers: the root, and what the web
  heard."""
  heard: list[tuple] = []
  return extended("web", at, lambda _: surfed(answers, heard))[1], heard


MCP = ("mcp_servers", "mcp_call", "mcp_read", "mcp_prompt", "mcp_control")
"""MCP is the kinds of act that the MCP of the suite answers."""


def served(answers: dict[str, object], heard: list[tuple]) -> World:
  """The MCP of the suite, in memory: an ear that takes each act of a kind in MCP, keeps it in heard, says it started,
  and says it done from its own loop with the answer of its kind, which is a value or a function of the act, or with a
  refusal when the kind has no answer."""
  while True:
    a = yield
    if a[0] in MCP and engine.question(a):
      heard.append(a)
      yield "started", a[1]
      got = answers.get(a[0], Refused(f"No server answers {a[0]}."))
      asyncio.get_running_loop().call_soon(world_says, "done", a[1], got(a) if callable(got) else got)


def mcped(
  at: Path, answers: dict[str, object], *, lives: bool = False, record: Sequence[tuple] = ()
) -> tuple[Sand, str, list[tuple]]:
  """A life that runs the extension mcp, with its life word when it lives, on the MCP of the suite with these answers:
  the World, the root and what the MCP heard."""
  heard: list[tuple] = []
  sand, root = extended("mcp", at, lambda _: served(answers, heard), lives=lives, record=record)
  return sand, root, heard


async def given(root: str, **names: object) -> None:
  """Binds names in the module of a chain before a word that uses them, so that the word does not hold their text
  and the open of an act tells it: a tell leaves out a text that the word of its act already shows."""
  await engine.rung("\n".join(f"{name} = {value!r}" for name, value in names.items()), on=root)


def server(name: str, **words: object) -> dict:
  """A server as the World answers mcp_servers: connected, with no tools, resources, templates, prompts or binding,
  which the words change."""
  return {
    "name": name,
    "scope": "project",
    "transport": "stdio",
    "target": f"{name}-mcp",
    "state": "connected",
    "error": "",
    "version": "1.0",
    "instructions": "",
    "tools": [],
    "resources": [],
    "templates": [],
    "prompts": [],
    "binding": "",
    **words,
  }


def tooled(name: str, description: str = "", schema: dict | None = None, **words: object) -> dict:
  """A tool as a server lists it, whose input is an object with no property unless the schema gives some, which the
  words change."""
  return {
    "name": name,
    "title": "",
    "description": description,
    "input": schema or {"type": "object", "properties": {}},
    "output": None,
    "annotations": {},
    **words,
  }


def bindings(name: str) -> str:
  """The binding of a server of that name that has one tool, ping, as the extension binds a class."""
  return f'class {name}:\n  async def ping() -> McpResult:\n    return await mcp_call("{name}", "ping", {{}})\n'


def quoted(name: str, text: str) -> str:
  """The quote that a paragraph binds a text of several lines in: its open mark, the text, and its close mark."""
  return "<" + f"s:{name}>\n{text}</" + f"s:{name}>"


IMPORT = re.compile(r"^from furb\.engine import \(.*?\)\n", re.S | re.M)


GH = """import json, os, re, subprocess, sys
args = sys.argv[1:]
at = args.index("--jq") if "--jq" in args else len(args)
jq = args[at + 1] if at < len(args) else None
said = " ".join(args[:at] + args[at + 2:])
log = os.environ["GH_LOG"]
before = open(log, encoding="utf-8").read().splitlines() if os.path.exists(log) else []
with open(log, "a", encoding="utf-8") as f:
  f.write(said + chr(10))
with open(os.environ["GH_SAYS"], encoding="utf-8") as f:
  says = json.load(f)
for pattern, out, code, *times in says:
  if re.search(pattern, said) and not (times and sum(bool(re.search(pattern, x)) for x in before) >= times[0]):
    break
else:
  sys.exit(0)
if jq and out and not code:
  out = subprocess.run(["jq", "-r", jq], input=out, capture_output=True, text=True, check=True).stdout
sys.stdout.write(out)
sys.exit(code)
"""


@dataclass
class Shell(Sand):
  """A World that runs each command in sh in the working directory of its chain, with the gh of the test first on
  its PATH, and says its done later, from its loop, but holds the done of each command that hold matches until the test
  releases it. It keeps each write of scratch:// in its files, and reads the clock at now, which a test moves."""

  env: dict[str, str] = field(default_factory=dict)
  now: float = 1000.0
  hold: str = ""
  held: list = field(default_factory=list)

  def hear(self, a: tuple):
    match a:
      case ("bash", about, _, on, command, stdin, *_):
        yield "started", about
        here = conftest.engine.cwd(on=on) or None
        args, merged = ["sh", "-c", command], conftest.engine.ask("merged", on, about)
        err = subprocess.STDOUT if merged else subprocess.PIPE
        got = subprocess.run(  # noqa: S603
          args,
          input=stdin or "",
          stdout=subprocess.PIPE,
          stderr=err,
          encoding="utf-8",
          env=self.env,
          cwd=here,
          check=False,
        )
        done = partial(
          world_says,
          "done",
          about,
          Exit(got.returncode, Text(f"{about}/stdout", got.stdout), Text(f"{about}/stderr", got.stderr or "")),
        )
        if self.hold and re.search(self.hold, command):
          self.held.append(done)
        else:
          self.loop.call_soon(done)
      case ("clock", qid, *_):
        yield "done", qid, self.now
      case ("write", qid, _, _, Text(path=path, content=content)) if path.startswith("scratch://"):
        self.files[path] = content
        yield "done", qid, Text(path, content)
      case _:
        yield from super().hear(a)

  def release(self) -> None:
    self.hold = ""
    for done in self.held:
      self.loop.call_soon(done)
    self.held.clear()


class Bench:
  """A life whose root stands in the folder of a test and plays the words of files of extensions, on a World whose gh
  answers each call from a table of patterns: the first that matches the call gives its output and its code."""

  def __init__(self, tmp: Path, files: list[Path], roster: list, actor: str) -> None:
    folder = tmp / "bin"
    folder.mkdir()
    (folder / "gh").write_text(f"#!{sys.executable}\n{GH}", encoding="utf-8")
    (folder / "gh").chmod(0o755)
    kept = {n: v for n, v in os.environ.items() if not n.startswith(("GIT_", "COVERAGE_PROCESS_"))}
    env = {
      **kept,
      "PATH": f"{folder}{os.pathsep}{os.environ['PATH']}",
      "GH_LOG": str(tmp / "gh.log"),
      "GH_SAYS": str(tmp / "says.json"),
    }
    self.tmp, self.says, self.engine = tmp, [], conftest.engine
    self.sand = Shell(files={}, env=env, stands=[roster, str(tmp), actor])
    self.say("^$", "")
    self.says.clear()
    self.log, self.root = life(self.sand)
    self.text = "\n\n".join(IMPORT.sub("", f.read_text(encoding="utf-8")) for f in files)

  async def load(self, *words: str) -> None:
    await self.word("\n\n".join([f"cd({str(self.tmp)!r})", self.text, *words]))

  @property
  def ns(self) -> dict:
    return self.engine.module(self.root)

  async def word(self, text: str) -> None:
    await self.engine.rung(text, on=self.root)
    await settle()

  async def value(self, expression: str) -> object:
    await self.word(f"got = {expression}")
    return self.ns["got"]

  def say(self, pattern: str, out: object, code: int = 0, times: int | None = None) -> None:
    """Answer each call that matches pattern, or only its first `times` calls: the later ones go to the next pattern."""
    entry = (pattern, out if isinstance(out, str) else json.dumps(out), code)
    self.says.insert(0, entry if times is None else (*entry, times))
    (self.tmp / "says.json").write_text(json.dumps(self.says), encoding="utf-8")

  def gh(self) -> list[str]:
    path = self.tmp / "gh.log"
    return path.read_text(encoding="utf-8").splitlines() if path.exists() else []
