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
    self.pausing = False
    self.now = 1000.0
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
      "vetoes": {},
    }
    self.say(r"rev-parse origin/main", "sha1\n")
    self.say(r"gh issue view", WORDS + "\n")
    self.say(r"python3 -I -", listing())
    self.say(r"merge-base", "base1\n")
    ns: dict = {"__name__": "converge", "Text": Text, "Exit": Exit, "Refused": Refused, "ROOT": "chain1"}
    ns.update(HIDDEN=None, module=lambda on="": self.round, cwd=lambda: str(tmp), read=self.read, write=self.write)
    ns.update(bash=self.bash, prompt=self.prompt, chain=self.chain, rung=self.rung, grant=self.grant, wait=self.wait)
    ns.update(over=lambda name: name in self.done, peek=lambda name, waiting=None: self.done.get(name, waiting))
    ns.update(acting=lambda: "rung1", scope=lambda name: "chain1")
    ns.update(get=self.facts.get, transcript=lambda on="", of="": [])
    ns.update(paused=lambda name: self.pausing, clock=lambda: self.now, cancel=self.cancel)
    exec(compile(SOURCE, str(HERE / "converge.py"), "exec"), ns)  # noqa: S102
    self.ns = ns
    self.target = {**ns["TUI"], "capture": lambda target, pairs, out: f"echo capture {pairs}"}

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

  def cancel(self, name: str) -> None:
    self.pending = [(one, value) for one, value in self.pending if one != name]
    self.done[name] = Refused("cancelled")

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


def listing(changed: tuple = ("demo/01-welcome",), kept: tuple = (), kind: str = "new", **more: object) -> str:
  shots = {name: "d1.png" for name in (*changed, *kept)}
  images = [["now", "d1.png"]] if kind == "new" else []
  told = [{"name": name, "kind": kind, "text": f"+  1 the row of {name}", "images": images} for name in changed]
  seen = {"shots": shots, "failed": [], "notes": [], "changed": told, "kept": list(kept), "records": {}}
  return json.dumps({**seen, **more}) + "\n"


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
  assert len(world.judged()) == 2
  assert "![now demo/01-welcome](furb-image://d1.png)" in world.judged()[0]
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
  world.say(r"python3 -I -", listing(failed=[["demo", "the step demo of the capture failed: screen 07 never came", ""]]))
  book = world.run(1)
  assert book["passes"][0]["gaps"] == [["demo", "the step demo of the capture failed: screen 07 never came", ""]]


def test_the_words_of_the_owner_are_his_issues_whole(world: World) -> None:
  assert asyncio.run(world.ruling(world.target)) == WORDS
  views = world.commands[0].split(" && echo && ")
  assert len(views) == 5
  assert views[0].startswith("gh issue view 14 --repo 'uael/furb-ops' --json number,title,body,comments --jq ")


def test_the_capture_of_the_tui_fills_each_place_of_its_command(world: World) -> None:
  command = world.tui_capture(world.ns["TUI"], [("before", "sha0"), ("head", "sha1")], "/out")
  assert not re.search(r"\{(repo|tree|out|pairs|name|records|heaviest|still|capturer)\}", command)
  assert f"tree='{world.tmp}-converge-tui'" in command
  assert "for pair in 'before:sha0' 'head:sha1'; do" in command
  assert command.count("FURB_STILL_CLOCK=1767225600000 ") == 2 and 'FURB_GALLERY_CELLS="$at/demo"' in command
  assert "bun run screenshots" in command and "converge-capture.ts" in command


def test_each_part_of_a_capture_opens_the_same_records_anew(world: World) -> None:
  command = world.tui_capture(world.ns["TUI"], [("head", "sha1")], "/out")
  assert 'rm -rf "$out/open" && cp -R "$out/records" "$out/open"' in command
  assert 'FURB_CONVERGE_RECORDS="$(ls "$out"/open/*.jsonl 2>/dev/null)"' in command


def grid(*rows: str, colors: tuple[str, str] = ("#ffffff", "#000000"), attributes: int = 0) -> str:
  return json.dumps({"cols": 40, "rows": len(rows), "lines": [[[row, *colors, attributes]] for row in rows]})


def shoot(tmp: Path, was: dict | None, now: dict, fails: str = "", whole: bool = False) -> dict:
  out = tmp / "out"
  for part, shots in (("before", was or {}), ("head", now)):
    for name, (png, cells) in shots.items():
      folder, base = out / part / name.split("/")[0], name.split("/")[1]
      folder.mkdir(parents=True, exist_ok=True)
      (folder / f"{base}.png").write_bytes(png)
      if cells is not None:
        (folder / f"{base}.json").write_text(cells, encoding="utf-8")
  if fails:
    (out / "fails.txt").write_text(fails, encoding="utf-8")
  words = [str(out), str(tmp / "images"), str(tmp), str(out / "head"), str(out / "before") if was is not None else ""]
  got = subprocess.run(
    [sys.executable, "-I", "-", *words, "1" if whole else "0", "main", "head"],
    input=World(tmp).ns["SHOTS"],
    capture_output=True,
    # On Windows, text=True encodes the input in the code page, which holds no braille spinner: the write fails in a
    # thread of its own, stdin never closes, and the child waits until the timeout kills the worker.
    encoding="utf-8",
    check=True,
  )
  return json.loads(got.stdout)


def test_the_list_of_the_captures_names_each_image_by_the_digest_of_its_bytes(tmp_path: Path) -> None:
  seen = shoot(tmp_path, None, {"demo/01-welcome": (b"\x89PNG one", None)})
  digest = hashlib.sha256(b"\x89PNG one").hexdigest() + ".png"
  assert seen["shots"] == {"demo/01-welcome": digest}
  assert (tmp_path / "images" / digest).read_bytes() == b"\x89PNG one"


def test_a_capture_whose_cells_did_not_change_goes_to_no_judge(tmp_path: Path) -> None:
  was = {"demo/02-feed": (b"one", grid("⠋ Working  # 212ms $0.0024   x", "runs  9.0s", "saved 1793047160463.html"))}
  now = {"demo/02-feed": (b"two", grid("⠴ Working  # 1215ms $0.0024  x", "runs  15s", "saved 1793047721926.html"))}
  seen = shoot(tmp_path, was, now)
  assert (seen["kept"], seen["changed"]) == (["demo/02-feed"], [])


def test_a_capture_whose_text_changed_comes_as_its_frame_with_each_changed_row_marked(tmp_path: Path) -> None:
  was = {"demo/02-feed": (b"one", grid("Main", "old row", "end"))}
  now = {"demo/02-feed": (b"two", grid("Main", "new row", "end"))}
  [change] = shoot(tmp_path, was, now)["changed"]
  assert (change["name"], change["kind"], change["images"]) == ("demo/02-feed", "text", [])
  assert change["text"].splitlines() == [
    "     styles: a #ffffff on #000000",
    "   1 Main",
    "+  2 new row",
    "     main 1-3: old",
    "     colors 1-3: a1-3",
    "   3 end",
  ]


def test_a_row_that_is_gone_or_new_comes_whole_and_rows_far_from_a_change_are_named(tmp_path: Path) -> None:
  rows = [f"row {n}" for n in range(1, 9)]
  was = {"demo/02-feed": (b"one", grid(*rows))}
  now = {"demo/02-feed": (b"two", grid(*rows[:6], "one more", *rows[6:], "the end"))}
  [change] = shoot(tmp_path, was, now)["changed"]
  assert change["text"].splitlines() == [
    "     styles: a #ffffff on #000000",
    "     rows 1-4 as main",
    "   5 row 5",
    "   6 row 6",
    "+  7 one more",
    "     colors: a1-8",
    "   8 row 7",
    "   9 row 8",
    "+ 10 the end",
    "     colors: a1-7",
  ]


def test_a_capture_whose_change_is_the_change_of_another_names_it(tmp_path: Path) -> None:
  was = {f"demo/0{n}-shot": (b"one", grid("old")) for n in (1, 2)}
  now = {f"demo/0{n}-shot": (bytes([n]), grid("new")) for n in (1, 2)}
  first, second = shoot(tmp_path, was, now)["changed"]
  assert (first["kind"], second["kind"], second["text"]) == ("text", "same", "the same change as demo/01-shot")


def test_a_capture_whose_colors_alone_changed_comes_with_both_images(tmp_path: Path) -> None:
  was = {"demo/02-feed": (b"one", grid("Main", colors=("#ffffff", "#000000")))}
  now = {"demo/02-feed": (b"two", grid("Main", colors=("#888888", "#000000"), attributes=2))}
  [change] = shoot(tmp_path, was, now)["changed"]
  assert change["kind"] == "colors"
  assert change["text"].splitlines() == [
    "     styles: a #ffffff on #000000, b #888888 on #000000 dim",
    "~  1 Main",
    "     colors main 1-4: a1-4",
    "     colors head 1-4: b1-4",
  ]
  digests = [hashlib.sha256(b).hexdigest() + ".png" for b in (b"one", b"two")]
  assert change["images"] == [["main", digests[0]], ["head", digests[1]]]


def test_a_new_capture_comes_whole_with_its_image(tmp_path: Path) -> None:
  [change] = shoot(tmp_path, {}, {"life/r-chain1": (b"one", grid("Main", "row"))})["changed"]
  assert (change["kind"], change["text"]) == ("new", "   1 Main\n   2 row")
  assert change["images"] == [["head", hashlib.sha256(b"one").hexdigest() + ".png"]]


def test_a_capture_of_a_pass_of_main_that_changed_comes_with_its_image(tmp_path: Path) -> None:
  was = {"demo/02-feed": (b"one", grid("old"))}
  [change] = shoot(tmp_path, was, {"demo/02-feed": (b"two", grid("new"))}, whole=True)["changed"]
  assert change["images"] == [["head", hashlib.sha256(b"two").hexdigest() + ".png"]]


def test_a_failed_step_of_the_head_is_a_gap_and_one_of_main_is_a_note(tmp_path: Path) -> None:
  out = tmp_path / "out"
  out.mkdir()
  (out / "build.log").write_text("cargo\nfailed here\n", encoding="utf-8")
  fails = f"head\tbuild\t{out / 'build.log'}\nbefore\tdemo\t{out / 'none.log'}\n"
  seen = shoot(tmp_path, {}, {}, fails)
  assert seen["failed"] == [["build", "the step build of the capture failed: cargo failed here", ""]]
  assert seen["notes"] == [["demo", "the step demo of the capture failed: ", ""]]


def test_a_message_to_a_model_has_no_blank_line_that_a_header_follows(world: World) -> None:
  world.say(r"gh issue view", "## #17 I4. T\n\n> " + QUOTE + "\n\n## #18 U\n\n#393 is wrong")
  world.judge = lambda message, to: gap()
  world.round["working"]["prompt80"] = "tui-design"
  world.run(1)
  messages = [*world.judged(), *[m for shape, m, to, on in world.prompts if shape is None and on == "at-prompt80"]]
  assert len(messages) == 2
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

def veto(world: World) -> bool:
  return asyncio.run(world.converge_veto(pr(verdict="pass")))


def test_the_conductor_is_given_the_veto_of_converge(world: World) -> None:
  assert world.round["vetoes"]["converge"] is world.ns["converge_veto"]


def test_a_green_pr_that_changes_a_capture_waits_for_the_judges_and_lands_on_a_clean_head(world: World) -> None:
  converged(world)
  world.say(r"gh pr view 5 --json files", "tui/src/app.ts\n")
  world.say(r"python3 -I -", listing(kind="text"))
  assert veto(world) is False
  assert world.book()["heads"]["5 abc"] == {"state": "clean", "gaps": []}
  assert not any(c.startswith("gh pr edit") for c in world.commands)
  assert any("for pair in 'before:base1' 'head:abc'; do" in c for c in world.commands)
  [asked] = world.judged()
  assert '<capture name="demo/01-welcome" change="text">\n+  1 the row of demo/01-welcome\n</capture>' in asked


def test_a_pr_head_asks_one_judge_one_prompt_with_the_words_once_and_its_changed_captures_alone(world: World) -> None:
  converged(world)
  shots = [f"demo/{n:02d}-shot" for n in range(40)]
  world.say(r"gh pr view 5 --json files", "tui/src/app.ts\n")
  world.say(r"python3 -I -", listing(changed=tuple(shots[:30]), kept=tuple(shots[30:]), kind="text"))
  veto(world)
  [(shape, message, to, on)] = [p for p in world.prompts if p[0] is not None]
  assert to == "claude-cli:sonnet/high"
  assert message.count(WORDS) == 1
  assert message.count("<capture name=") == 30 and "demo/35-shot" not in message


def test_each_chain_of_a_judge_holds_a_grant_of_its_ceiling(world: World) -> None:
  converged(world)
  world.say(r"gh pr view 5 --json files", "tui/src/app.ts\n")
  world.say(r"python3 -I -", listing(kind="text"))
  veto(world)
  [asked] = [p for p in world.prompts if p[0] is not None]
  grants = [fact for fact in world.facts.values() if fact[0] == "grant"]
  assert [(fact[3], fact[4]) for fact in grants] == [(asked[3], 3.0)]


def test_a_pr_with_gaps_keeps_the_veto_and_its_worker_and_a_comment_get_them(world: World) -> None:
  converged(world)
  world.judge = lambda message, to: gap()
  world.say(r"gh pr view 5 --json files", "tui/src/app.ts\n")
  world.say(r"python3 -I -", listing(kind="text"))
  world.round["owner_of"] = lambda one: "prompt70"
  world.round["prs"][5] = pr(verdict="pass")
  assert veto(world) is True
  assert veto(world) is True
  assert world.book()["heads"]["5 abc"]["state"] == "gaps"
  assert len(world.judged()) == 1
  assert any(c.startswith("gh pr comment 5") for c in world.commands)
  assert not any("label" in c for c in world.commands)
  assert [on for shape, message, to, on in world.prompts if shape is None] == ["at-prompt70"]


def test_a_pr_that_changes_no_capture_passes_with_no_judge(world: World) -> None:
  converged(world)
  world.say(r"gh pr view 5 --json files", "tui/src/app.ts\n")
  world.say(r"python3 -I -", listing(changed=(), kept=("demo/01-welcome",)))
  assert veto(world) is False
  assert world.book()["heads"]["5 abc"]["state"] == "clean"
  assert world.judged() == []


def test_a_pr_outside_the_paths_is_not_judged(world: World) -> None:
  converged(world)
  world.say(r"gh pr view 5 --json files", "src/engine.rs\n")
  assert veto(world) is False
  assert world.ns["LIVE"]["tui"]["heads"]["5 abc"] == {"state": "outside"}
  assert world.judged() == []


def test_judges_that_give_no_list_twice_keep_the_veto_and_tell_the_operator_once(world: World) -> None:
  converged(world)
  world.judge = lambda message, to: Refused("paused")
  world.say(r"gh pr view 5 --json files", "tui/src/app.ts\n")
  world.say(r"python3 -I -", listing(kind="text"))
  assert [veto(world) for _ in range(3)] == [True, True, True]
  assert world.book()["heads"]["5 abc"]["state"] == "stuck"
  assert len(world.told()) == 1


def test_a_judge_whose_chain_pauses_gives_no_answer_and_its_head_goes_stuck(world: World) -> None:
  converged(world)
  world.slow, world.pausing = True, True
  world.say(r"gh pr view 5 --json files", "tui/src/app.ts\n")
  world.say(r"python3 -I -", listing(kind="text"))
  assert [veto(world) for _ in range(3)] == [True, True, True]
  assert world.book()["heads"]["5 abc"] == {"state": "stuck", "tries": 2}
  assert world.pending == []
  assert len(world.told()) == 1 and "gave no answer twice" in world.told()[0]


def test_a_judge_that_passes_its_bound_gives_no_answer(world: World) -> None:
  converged(world)
  world.slow = True
  world.say(r"gh pr view 5 --json files", "tui/src/app.ts\n")
  world.say(r"python3 -I -", listing(kind="text"))
  assert veto(world) is True
  assert world.book()["heads"]["5 abc"] == {"state": "held"}
  world.now += world.ns["BOUND"]
  assert veto(world) is True
  assert world.book()["heads"]["5 abc"] == {"state": "held", "tries": 1}
  assert world.pending == []


def test_a_capture_of_main_that_did_not_change_keeps_the_gaps_of_the_last_pass(world: World) -> None:
  world.judge = lambda message, to: gap()
  world.run(1)
  world.round["queue"].clear()
  world.done.update({n: None for n in list(world.done)})
  world.ns["LIVE"]["tui"]["step"] = None
  world.say(r"python3 -I -", listing(changed=(), kept=("demo/01-welcome",)))
  book = world.run(1)
  assert len(world.judged()) == 1
  assert [p["gaps"] for p in book["passes"]] == [gap(), gap()]
  assert book["passes"][1]["judged"] == 0
  assert book["frames"].endswith("/head")


def test_a_pass_of_main_keeps_its_records_for_the_next_capture(world: World) -> None:
  world.say(r"python3 -I -", listing(records={"r.jsonl": 10}))
  book = world.run(2)
  assert book["records"] == {"r.jsonl": 10}
  assert any("""printf '%s' '{"r.jsonl": 10}'""" in c for c in world.commands)
