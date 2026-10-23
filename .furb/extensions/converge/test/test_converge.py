import asyncio
import hashlib
import json
import re
import subprocess
import sys
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path

import pytest

HERE = Path(__file__).resolve().parent.parent
SOURCE = (HERE / "converge.py").read_text(encoding="utf-8")
WORDS = "## #17 I4. The TUI converges\n\n> I want the TUI to be perfect so it need to be the result of a convergence"
QUOTE = "I want the TUI to be perfect"


@dataclass
class Text:
  path: str
  content: str = ""

  @property
  def lines(self) -> list[str]:
    return self.content.splitlines()


@dataclass
class Exit:
  code: int | None
  stdout: Text
  stderr: Text


class Refused(Exception):
  pass


class Done(str):
  value: object = None

  def __await__(self):
    return self.value
    yield


class World:
  def __init__(self, tmp: Path) -> None:
    self.tmp = tmp
    self.says: list[tuple[str, str, int]] = []
    self.commands: list[str] = []
    self.files: dict[str, str] = {}
    self.done: dict[str, object] = {}
    self.facts: dict[str, tuple] = {}
    self.prompts: list[tuple[object, str, str, str]] = []
    self.judge: Callable[[str, str], object] = lambda message, to: []
    self.waited: Callable[[], None] = lambda: None
    self.made = 0
    self.waits: list[list[str]] = []
    self.pending: list[tuple[str, object]] = []
    self.slow = False
    self.round = {
      "quietly": lambda verb, *args, **words: verb(*args, **words),
      "shq": lambda s: "'" + s.replace("'", "'\\''") + "'",
      "ops": lambda: ("uael/furb-ops", "13"),
      "queue": [],
      "working": {},
      "results": {},
      "merged": set(),
      "prs": {},
      "timers": {"sweep": 1.0},
      "owner_of": lambda pr: "",
      "working_at": lambda name: f"at-{name}",
    }
    self.say(r"rev-parse origin/main", "sha1\n")
    self.say(r"gh issue view", WORDS + "\n")
    self.say(r"python3 -I -", "S\tdemo/01-welcome\td1.png\n")
    ns: dict = {"__name__": "converge", "Text": Text, "Exit": Exit, "Refused": Refused, "ROOT": "chain1"}
    ns.update(HIDDEN=None, module=lambda on="": self.round, cwd=lambda: str(tmp), read=self.read, write=self.write)
    ns.update(bash=self.bash, prompt=self.prompt, chain=self.chain, rung=self.rung, grant=self.grant, wait=self.wait)
    ns.update(over=lambda name: name in self.done, peek=lambda name, waiting=None: self.done.get(name, waiting))
    ns.update(get=self.facts.get, transcript=lambda on="", of="": [])
    exec(compile(SOURCE, str(HERE / "converge.py"), "exec"), ns)  # noqa: S102
    self.ns = ns
    self.target = {**ns["TUI"], "capture": lambda target, sha, out: f"echo capture {sha}"}

  def __getattr__(self, name: str) -> object:
    return self.ns[name]

  def say(self, pattern: str, out: str, code: int = 0) -> None:
    self.says.insert(0, (pattern, out, code))

  def act(self, kind: str, value: object = None, *fact: object) -> Done:
    self.made += 1
    name = Done(f"{kind}{self.made}")
    name.value = value
    self.done[name] = value
    self.facts[name] = (kind, name, "rung1", *fact)
    return name

  def bash(self, command: str, stdin: str | None = "", timeout: float = 600, show: object = None) -> Done:
    self.commands.append(command)
    out, code = next(((o, c) for p, o, c in self.says if re.search(p, command)), ("", 0))
    return self.act("bash", Exit(code, Text("stdout", out), Text("stderr", "")))

  def read(self, path: str, show: object = None, on: str = "") -> Text:
    if path not in self.files:
      raise Refused(path)
    return Text(path, self.files[path])

  def write(self, text: Text, on: str = "") -> Text:
    self.files[text.path] = text.content
    return text

  def chain(self, label: str = "", source: str = "", on: str = "") -> str:
    return self.act("chain", None, "chain1", label)

  def rung(self, word: str = "", on: str = "") -> Done:
    return self.act("rung", None, on, word)

  def grant(self, usd: float | None = None, share: float | None = None, on: str = "") -> Done:
    return self.act("grant", None, on, usd)

  def prompt(self, shape: object, message: str = "", to: str = "", on: str = "") -> Done:
    self.prompts.append((shape, message, to, on))
    if shape is None:
      return self.act("prompt", None, on, "None", message, to)
    name = self.act("prompt", self.judge(message, to), on, str(shape), message, to)
    if self.slow:
      self.pending.append((name, self.done.pop(name)))
    return name

  def wait(self, seconds: float = 0.0, *acts: str) -> Done:
    self.waits.append([a for a in acts if a in self.done])
    self.waited()
    return self.act("wait", None)

  def told(self) -> list[str]:
    return [message for shape, message, to, on in self.prompts if shape is None and on == "chain1"]

  def judged(self) -> list[str]:
    return [message for shape, message, to, on in self.prompts if shape is not None]

  def book(self) -> dict:
    return json.loads(self.files[".furb/converge/tui.json"])

  def run(self, ticks: int) -> dict:
    return asyncio.run(self.converge(self.target, ticks))


@pytest.fixture
def world(tmp_path: Path) -> World:
  return World(tmp_path)


def gap(shot: str = "demo/01-welcome", quote: str = QUOTE) -> list[list[str]]:
  return [[shot, "The feed shows bright white text.", quote]]


def test_a_gap_counts_only_when_it_names_a_capture_and_quotes_the_words_of_the_owner(world: World) -> None:
  answer = [
    ("demo/01-welcome", "The text is cut.", QUOTE),
    ("demo/01-welcome", "  the text is CUT.  ", "> I want the **TUI** to be perfect"),
    ("demo/99-none", "No such capture.", QUOTE),
    ("demo/01-welcome", "A taste of the judge.", "it should be blue"),
    ("demo/01-welcome", "Too short a quote.", "TUI"),
    "not a tuple",
  ]
  asked = world.act("prompt", answer)
  assert world.gaps_of([asked], WORDS, {"demo/01-welcome"}) == ([["demo/01-welcome", "The text is cut.", QUOTE]], [])


def test_a_judge_that_gives_no_list_makes_the_pass_inconclusive(world: World) -> None:
  world.judge = lambda message, to: Refused("paused") if "sonnet" in to else []
  book = world.run(1)
  assert book["clean"] == 0
  assert book["passes"][0]["sha"] == "sha1"
  assert "inconclusive" in book["passes"][0]


def test_a_target_converges_after_clean_passes_in_a_row_and_tells_the_operator(world: World) -> None:
  book = world.run(3)
  assert [p["gaps"] for p in book["passes"]] == [[], []]
  assert book["state"] == "converged"
  assert world.told() == [
    "converge: tui converged: 2 passes in a row at sha1 found no gap against the words of the owner."
    " Its ledger is .furb/converge/tui.json."
  ]
  assert len(world.judged()) == 4
  assert "![demo/01-welcome](furb-image://d1.png)" in world.judged()[0]
  assert WORDS in world.judged()[0]


def test_a_pass_with_gaps_queues_one_item_and_the_next_pass_waits_for_its_pr_to_land(world: World) -> None:
  world.judge = lambda message, to: gap()
  queue, working = world.round["queue"], world.round["working"]
  steps = iter(
    [
      lambda: None,
      lambda: (queue.clear(), working.update({"prompt90": "converge-tui-1"})),
      lambda: (working.clear(), world.done.update({"prompt90": (7, "report")}), world.round["prs"].update({7: pr(7, head="other")})),
      lambda: world.round["merged"].add(7),
      lambda: None,
    ]
  )
  world.waited = lambda: next(steps)()
  book = world.run(5)
  assert len(book["passes"]) == 2
  key, text, thread = queue[0]
  assert (key, thread) == ("converge-tui-2", "tui")
  assert "The feed shows bright white text." in text and "His words: " + QUOTE in text
  assert "![demo/01-welcome](furb-image://d1.png)" in text and WORDS in text


def test_the_gaps_go_to_each_worker_of_an_owner_that_works(world: World) -> None:
  world.judge = lambda message, to: gap()
  world.round["working"]["prompt80"] = "tui-design"
  book = world.run(1)
  assert book["step"] == {"routed": ["prompt80"], "sha": "sha1"}
  assert world.round["queue"] == []
  assert [on for shape, message, to, on in world.prompts if shape is None] == ["at-prompt80"]


def test_a_target_stalls_after_passes_with_no_fewer_gaps_than_its_best(world: World) -> None:
  world.judge = lambda message, to: gap()
  world.waited = lambda: world.round["queue"].clear()
  book = world.run(6)
  assert len(book["passes"]) == 4
  assert book["state"] == "stalled"
  assert world.told()[0].startswith("converge: tui stalled: its last 3 passes found [1, 1, 1] gaps")


def pr(n: int = 5, sha: str = "abc", verdict: str = "pending", labels: tuple = (), head: str = "") -> dict:
  return {"n": n, "sha": sha, "head": head or f"w/k{n}", "base": "main", "labels": list(labels), "verdict": verdict}


def converged(world: World) -> None:
  world.files[".furb/converge/tui.json"] = json.dumps(
    {"state": "converged", "since": "sha1", "words": WORDS, "shots": {"demo/01-welcome": "d1.png"}, "clean": 2}
  )


def test_an_open_pr_that_changes_the_paths_is_held_and_judged_once_green(world: World) -> None:
  converged(world)
  world.say(r"gh pr view 5 --json files", "tui/src/app.ts\n")
  world.round["prs"][5] = pr()
  world.waited = lambda: world.round["prs"].update({5: pr(verdict="pass", labels=("hold",))})
  world.say(r"python3 -I -", "S\tdemo/01-welcome\td2.png\n")
  book = world.run(2)
  assert [c for c in world.commands if c.startswith("gh pr edit")] == [
    "gh pr edit 5 --add-label hold >/dev/null 2>&1",
    "gh pr edit 5 --remove-label hold >/dev/null 2>&1",
  ]
  assert book["heads"]["5 abc"] == {"state": "clean", "gaps": []}
  assert book["held"] == []
  pairs = world.judged()[0]
  assert "![main demo/01-welcome](furb-image://d1.png)" in pairs and "![head demo/01-welcome](furb-image://d2.png)" in pairs


def test_a_pr_with_gaps_keeps_its_hold_and_its_worker_and_a_comment_get_them(world: World) -> None:
  converged(world)
  world.judge = lambda message, to: gap()
  world.say(r"gh pr view 5 --json files", "tui/src/app.ts\n")
  world.say(r"python3 -I -", "S\tdemo/01-welcome\td2.png\n")
  world.round["prs"][5] = pr(verdict="pass")
  world.round["owner_of"] = lambda one: "prompt70"
  book = world.run(1)
  assert book["heads"]["5 abc"]["state"] == "gaps"
  assert book["held"] == [5]
  assert any(c.startswith("gh pr comment 5") for c in world.commands)
  assert [on for shape, message, to, on in world.prompts if shape is None] == ["at-prompt70"]


def test_a_pr_that_changes_no_capture_passes_with_no_judge(world: World) -> None:
  converged(world)
  world.say(r"gh pr view 5 --json files", "tui/src/app.ts\n")
  world.round["prs"][5] = pr(verdict="pass")
  book = world.run(1)
  assert book["heads"]["5 abc"]["state"] == "clean"
  assert world.judged() == []


def test_a_pr_outside_the_paths_is_neither_held_nor_judged(world: World) -> None:
  converged(world)
  world.say(r"gh pr view 5 --json files", "src/engine.rs\n")
  world.round["prs"][5] = pr(verdict="pass")
  book = world.run(1)
  assert book["heads"]["5 abc"] == {"state": "outside"}
  assert not any(c.startswith("gh pr edit") for c in world.commands)


def test_a_change_of_the_words_of_the_owner_starts_the_passes_again(world: World) -> None:
  converged(world)
  world.say(r"gh issue view", WORDS + " A new word.\n")
  book = world.run(1)
  assert book["state"] == ""
  assert len(book["passes"]) == 1


def test_the_ledger_lets_a_later_life_go_on_from_its_last_pass(world: World, tmp_path: Path) -> None:
  world.run(1)
  later = World(tmp_path)
  later.files = world.files
  book = later.run(1)
  assert book["state"] == "converged"
  assert len(book["passes"]) == 2


def test_the_budget_ends_the_loop(world: World) -> None:
  world.ns["JOBS"] = 1
  world.judge = lambda message, to: gap()
  world.waited = lambda: world.round["queue"].clear()
  book = world.run(10)
  assert book["state"] == "spent"
  assert world.told() == ["converge: tui spent its budget: 1 captures and 0.00 dollars."]


def test_a_failed_capture_is_a_gap_of_its_own(world: World) -> None:
  world.say(r"python3 -I -", "S\tdemo/01-welcome\td1.png\nF\tdemo\tscreen 07 never came\n")
  book = world.run(1)
  assert book["passes"][0]["gaps"] == [["demo", "the step demo of the capture failed: screen 07 never came", ""]]


def test_the_words_of_the_owner_are_his_issues_whole(world: World) -> None:
  assert asyncio.run(world.ruling(world.target)) == WORDS
  views = world.commands[0].split(" && echo && ")
  assert len(views) == 5
  assert views[0].startswith("gh issue view 14 --repo 'uael/furb-ops' --json number,title,body,comments --jq ")


def test_the_capture_of_the_tui_fills_each_place_of_its_command(world: World) -> None:
  command = world.tui_capture(world.ns["TUI"], "sha1", "/out")
  assert not re.search(r"\{(repo|tree|out|sha|name|records|heaviest|capturer)\}", command)
  assert f"tree='{world.tmp}-converge-tui'" in command
  assert "bun run screenshots" in command and "converge-capture.ts" in command


def test_the_list_of_the_captures_names_each_image_by_the_digest_of_its_bytes(tmp_path: Path) -> None:
  out, images = tmp_path / "out", tmp_path / "images"
  (out / "demo").mkdir(parents=True)
  (out / "demo" / "01-welcome.png").write_bytes(b"\x89PNG one")
  (out / "build.log").write_text("cargo\nfailed here\n", encoding="utf-8")
  (out / "fails.txt").write_text(f"build\t{out / 'build.log'}\n", encoding="utf-8")
  world = World(tmp_path)
  got = subprocess.run(
    [sys.executable, "-I", "-", str(out), str(images), str(tmp_path)],
    input=world.ns["SHOTS"],
    capture_output=True,
    text=True,
    check=True,
  )
  digest = hashlib.sha256(b"\x89PNG one").hexdigest() + ".png"
  assert got.stdout.splitlines() == [f"S\tdemo/01-welcome\t{digest}", "F\tbuild\tcargo failed here"]
  assert (images / digest).read_bytes() == b"\x89PNG one"

def test_a_message_to_a_model_has_no_blank_line_that_a_header_follows(world: World) -> None:
  world.say(r"gh issue view", "## #17 I4. T\n\n> " + QUOTE + "\n\n## #18 U\n\n#393 is wrong")
  world.judge = lambda message, to: gap()
  world.round["working"]["prompt80"] = "tui-design"
  world.run(1)
  messages = [*world.judged(), *[m for shape, m, to, on in world.prompts if shape is None and on == "at-prompt80"]]
  assert len(messages) == 3
  assert not any(re.search(r"\n\n#\S", message) for message in messages)

def test_a_wait_names_only_the_acts_that_still_run(world: World) -> None:
  world.slow = True

  def finish() -> None:
    if world.pending:
      name, value = world.pending.pop(0)
      world.done[name] = value

  world.waited = finish
  book = world.run(8)
  assert world.waits and all(done == [] for done in world.waits)
  assert book["passes"] and world.pending == []

def test_a_list_of_gaps_for_a_worker_holds_its_first_few_and_counts_the_rest(world: World) -> None:
  gaps = [["demo/01-welcome", f"Gap {i}.", QUOTE] for i in range(15)]
  text = world.listed(gaps, {"demo/01-welcome": "d1.png"})
  assert text.count("His words: ") == 12
  assert text.endswith("- and 3 more gaps, which a later pass shows again")
