import asyncio
import json
import os
import re
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

import pytest

HERE = Path(__file__).resolve().parent.parent
SOURCE = (HERE / "conductor.py").read_text(encoding="utf-8")
PULL = HERE.parent / "pr" / "pr.py"
IMPORT = re.compile(r"^from furb\.engine import \(.*?\)\n", re.S | re.M)
POSIX = pytest.mark.skipif(sys.platform == "win32", reason="the round runs its commands in sh")
JQ = pytest.mark.skipif(not shutil.which("jq"), reason="the stub of gh applies --jq with jq")
GH = """import json, os, re, subprocess, sys
args = sys.argv[1:]
at = args.index("--jq") if "--jq" in args else len(args)
jq = args[at + 1] if at < len(args) else None
said = " ".join(args[:at] + args[at + 2:])
with open(os.environ["GH_LOG"], "a", encoding="utf-8") as f:
  f.write(said + chr(10))
with open(os.environ["GH_SAYS"], encoding="utf-8") as f:
  says = json.load(f)
for pattern, out, code in says:
  if re.search(pattern, said):
    break
else:
  sys.exit(0)
if jq and out and not code:
  out = subprocess.run(["jq", "-r", jq], input=out, capture_output=True, text=True, check=True).stdout
sys.stdout.write(out)
sys.exit(code)
"""
SONNET, OPUS = "claude-cli:sonnet/high", "claude-cli:opus/medium"
RULED = [{"type": "required_status_checks", "parameters": {"required_status_checks": [{"context": "the gates"}]}}]


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


def raw(n: int, key: str = "", checks: tuple = (("the gates", "SUCCESS"),), labels: tuple = (), **more: object) -> dict:
  marks = [*([f"w:{key}"] if key else []), *labels]
  rollup = [
    {"__typename": "CheckRun", "name": name, "status": "COMPLETED" if end else "IN_PROGRESS", "conclusion": end, "startedAt": "2026-11-03T10:00:00Z"}
    for name, end in checks
  ]
  return {
    "number": n,
    "isDraft": False,
    "labels": [{"name": m} for m in marks],
    "mergeable": "MERGEABLE",
    "baseRefName": "main",
    "headRefName": f"w/{key or n}",
    "headRefOid": f"sha{n}",
    "statusCheckRollup": rollup,
    "body": "",
    "files": [],
    "autoMergeRequest": None,
    **more,
  }


class Round:
  def __init__(self, tmp: Path, before: dict | None = None) -> None:
    self.tmp, self.now, self.chain = tmp, 1000.0, "chain1"
    self.rungs: list[tuple[str, str]] = []
    self.targets: list[str] = []
    self.says: list[tuple[str, str, int]] = []
    self.prompts: list[tuple[str, str]] = []
    self.hired: list[tuple[str, str, str]] = []
    self.files: dict[str, str] = {}
    self.facts: dict[str, tuple] = {}
    self.done: dict[str, object] = {}
    self.facts_of: dict[str, list[tuple]] = {}
    folder = tmp / "bin"
    folder.mkdir()
    (folder / "gh").write_text(f"#!{sys.executable}\n{GH}", encoding="utf-8")
    (folder / "gh").chmod(0o755)
    self.env = {
      **{n: v for n, v in os.environ.items() if not n.startswith(("GIT_", "COVERAGE_PROCESS_"))},
      "PATH": f"{folder}{os.pathsep}{os.environ['PATH']}",
      "GH_LOG": str(tmp / "gh.log"),
      "GH_SAYS": str(tmp / "says.json"),
    }
    self.say("^$", "")
    self.says.clear()
    ns: dict = {"__name__": "conductor", "Text": Text, "Exit": Exit, "Refused": Refused, "HIDDEN": None, "ROOT": "chain1"}
    ns.update(operated=lambda site: site.split("/")[0] == "operator", bash=self.bash, clock=lambda: self.now)
    ns.update(cwd=lambda: str(tmp), get=self.facts.get, over=lambda name: name in self.done, prompt=self.prompt)
    ns.update(peek=lambda name, waiting=None: self.done.get(name, waiting), read=self.read, write=self.write)
    ns.update(transcript=lambda on="", of="": [x for x in self.facts_of.get(on or "chain1", []) if not of or x[0] == of])
    ns.update(scope=lambda name: self.chain, acting=lambda: "rung1", module=lambda on="": {})
    ns.update(chain=lambda label="", source="": f"chain-{label}", rung=self.rung, cd=lambda path, on="": path)
    ns.update(act=lambda kind, on, ear, *words: kind, wait=self.wait)
    ns["limits"] = {"room": {"sonnet": 3, "opus": 2}, "fixer": SONNET, "models": {"editor": SONNET, "audit": OPUS}}
    ns.update(before or {})
    # The root plays pr before the conductor, which uses its names, as .furb/config.json orders them.
    exec(compile(IMPORT.sub("", PULL.read_text(encoding="utf-8")), str(PULL), "exec"), ns)  # noqa: S102
    exec(compile(IMPORT.sub("", SOURCE), str(HERE / "conductor.py"), "exec"), ns)  # noqa: S102
    self.real_hire = ns["hire"]
    ns.update(quietly=self.quietly, hire=self.hire, PROBE="echo 64")
    self.ns = ns

  def __getattr__(self, name: str) -> object:
    return self.ns[name]

  def quietly(self, verb: object, *args: object, **words: object) -> object:
    got = verb(*args, **words)
    if asyncio.iscoroutine(got):
      got.close()
    return got

  async def wait(self, seconds: float = 0, *acts: str, on: str = "") -> None:
    return None

  def say(self, pattern: str, out: object, code: int = 0) -> None:
    self.says.insert(0, (pattern, out if isinstance(out, str) else json.dumps(out), code))
    (self.tmp / "says.json").write_text(json.dumps(self.says), encoding="utf-8")

  def log(self) -> str:
    path = self.tmp / "gh.log"
    return path.read_text(encoding="utf-8") if path.exists() else ""

  def bash(self, command: str, stdin: str | None = "", timeout: float = 600, **_: object):
    async def done() -> Exit:
      got = subprocess.run(["sh", "-c", command], input=stdin or "", capture_output=True, text=True, env=self.env, cwd=self.tmp, check=False)
      return Exit(got.returncode, Text("stdout", got.stdout), Text("stderr", got.stderr))

    return done()

  def prompt(self, shape: object, message: str = "", to: str = "", on: str = "") -> str:
    self.prompts.append((message, on))
    self.targets.append(to)
    name = f"prompt{100 + len(self.prompts)}"
    self.facts.setdefault(name, ("prompt", name, "rung1", on or "chain1", "None", message, to))
    return name

  def rung(self, word: str = "", retells: str = "", to: str = "", on: str = "") -> str:
    self.rungs.append((word, on))
    return f"rung{len(self.rungs)}"

  def read(self, path: str, show: object = None, on: str = "") -> Text:
    if path not in self.files:
      raise Refused(path)
    return Text(path, self.files[path])

  def write(self, text: Text, on: str = "") -> Text:
    self.files[text.path] = text.content
    return text

  def hire(self, key: str, what: str, thread: str = "", brief: str = "", model: str = "") -> str:
    self.hired.append((key, brief or what, thread))
    return self.busy(f"worker{len(self.hired)}", key, model or self.ns["model_of"](key))

  def busy(self, name: str, key: str, model: str = SONNET) -> str:
    self.ns["working"][name] = key
    self.ns["models"][name] = model
    self.facts[name] = ("prompt", name, "rung1", f"chain-{key}", "tuple[int, str]", "", model)
    return name

  def end(self, name: str, value: object) -> None:
    self.done[name] = value

  def notices(self) -> list[str]:
    return [m for m, on in self.prompts if on == ""]

  def github(self, *open_prs: dict, merged: tuple = (), main: str = "success", ruled: list | None = None) -> None:
    self.say(r"pr list --state open", list(open_prs))
    self.say(r"pr list --state merged", [{"number": n} for n in merged])
    self.say(r"run list --branch main", [{"headSha": "m1", "conclusion": main}])
    self.say(r"rules/branches/main", RULED if ruled is None else ruled)


@pytest.fixture
def conductor(tmp_path: Path) -> Round:
  return Round(tmp_path)


def test_the_file_loads_over_a_module_that_holds_the_state_of_the_old_one(tmp_path: Path) -> None:
  old = {
    "queue": [("a", "brief a", "t")],
    "backlog": [("b", "brief b", "t")],
    "working": {"prompt9": "c"},
    "results": {"prompt8": (7, "done")},
    "ends": {"d": [("prompt8", 7)]},
    "after": {"b": ["d"]},
    "areas": {"a": ["src/furb"]},
    "efforts": {"a": OPUS},
    "kinds": {"align7": "audit"},
    "homes": {"c": "chain-c"},
    "models": {"prompt9": SONNET},
    "slots": {"prompt9": "w2"},
    "prs": {7: {"n": 7}},
    "merged": {1, 2},
    "touched": {7: {"sha": "x", "paths": ["a.py"]}},
    "assigned": {7: {"sha": "x", "who": "prompt9", "at": 1.0, "fixer": False, "maker": "prompt9", "tries": 0}},
    "limits": {"room": {"opus": 1, "sonnet": 3, "opus/high": 2}, "fixer": SONNET, "models": {"actor": OPUS, "editor": SONNET, "audit": OPUS}},
    "lead": {"home": "chain1"},
    "noticed": {"k"},
    "heard": ["line"],
    "landed": [3],
    "owner_asks": {"A1": {"prompt": "prompt483", "quote": "q", "items": ["a"], "state": "open"}},
    "align_state": {"base": 40, "running": "rung5"},
    "comments": {"c1"},
  }
  kept = {name: id(value) for name, value in old.items()}
  got = Round(tmp_path, dict(old))
  assert {name: id(got.ns[name]) for name in old} == kept
  assert got.cap() == 6
  assert got.model_of("a") == OPUS


def test_an_old_module_that_bound_owner_asks_to_a_function_reads_the_asks_from_their_file(tmp_path: Path) -> None:
  def owner_asks() -> list:
    return []

  got = Round(tmp_path, {"owner_asks": owner_asks})
  assert got.owner_asks == {}


def test_cap_is_ten_at_most_and_room_counts_live_workers_of_a_family(conductor: Round) -> None:
  conductor.limits["room"] = {"sonnet": 4, "opus": 8}
  assert conductor.cap() == 10
  for n in range(4):
    conductor.busy(f"w{n}", f"k{n}")
  assert not conductor.room(SONNET)
  assert conductor.room(OPUS)
  for n in range(4, 10):
    conductor.busy(f"w{n}", f"k{n}", OPUS)
  assert not conductor.room(OPUS)
  conductor.end("w0", (1, "r"))
  assert conductor.room(OPUS)


def test_a_load_with_no_limits_binds_the_room_of_the_owner_and_caps_at_ten(tmp_path: Path) -> None:
  got = Round(tmp_path)
  del got.ns["limits"]
  exec(compile(IMPORT.sub("", SOURCE), str(HERE / "conductor.py"), "exec"), got.ns)  # noqa: S102
  assert got.limits["room"] == {"opus/high": 2, "opus/medium": 2, "sonnet/high": 3, "sonnet/medium": 3}
  assert got.cap() == 10
  assert got.GATE == "the gates"
  assert got.family("claude-cli:opus/high") == "opus/high"


def test_staff_hires_queue_then_backlog_in_order_while_the_cap_has_room(conductor: Round) -> None:
  conductor.queue.extend([("q1", "b", "t"), ("q2", "b", "t")])
  conductor.backlog.extend([("b1", "b", "t"), ("b2", "b", "t"), ("b3", "b", "t"), ("b4", "b", "t")])
  conductor.limits["room"] = {"sonnet": 5}
  conductor.staff()
  assert [h[0] for h in conductor.hired] == ["q1", "q2", "b1", "b2", "b3"]
  assert conductor.backlog == [("b4", "b", "t")]


def test_an_item_waits_for_its_after_until_each_pr_of_it_merged_and_says_why_once(conductor: Round) -> None:
  conductor.queue.append(("late", "b", "t"))
  conductor.after["late"] = ["first"]
  conductor.staff()
  conductor.staff()
  assert conductor.hired == []
  assert conductor.heard == ["late waits: first has not ended, and no item holds it."]
  conductor.ends["first"] = [("prompt1", 12)]
  conductor.staff()
  assert conductor.heard[-1] == "late waits: #12 of first has not merged."
  conductor.merged.add(12)
  conductor.staff()
  assert [h[0] for h in conductor.hired] == ["late"]


def test_an_item_waits_while_a_live_worker_holds_its_files_and_a_test_file_holds_none(conductor: Round) -> None:
  conductor.areas.update(one=["src/furb/engine.py", "test/conftest.py"], two=["src/furb", "test/conftest.py"], three=["test/conftest.py"])
  conductor.busy("prompt1", "one")
  conductor.queue.extend([("two", "b", "t"), ("three", "b", "t")])
  conductor.staff()
  assert [h[0] for h in conductor.hired] == ["three"]
  assert conductor.heard == ["two waits: the worker of one holds src/furb."]


def test_an_item_waits_while_an_open_pr_of_the_round_changes_its_files_but_not_for_its_own_pr(conductor: Round) -> None:
  conductor.areas.update(mine=["tui/src"], other=["tui/src/app.ts"])
  conductor.prs[9] = {"n": 9, "labels": ["w:mine"]}
  conductor.touched[9] = {"sha": "s", "paths": ["tui/src/app.ts", "tui/test/app.test.ts"]}
  conductor.queue.extend([("other", "b", "t"), ("mine", "b", "t")])
  conductor.staff()
  assert [h[0] for h in conductor.hired] == ["mine"]
  assert conductor.heard == ["other waits: #9 changes tui/src/app.ts."]


def test_an_item_with_no_areas_holds_no_file_whatever_its_brief_names(conductor: Round) -> None:
  conductor.prs[9] = {"n": 9, "labels": ["w:x"]}
  conductor.touched[9] = {"sha": "s", "paths": ["src/furb/engine.py"]}
  conductor.queue.append(("free", "Change src/furb/engine.py, as https://github.com/a/b/blob/main/src/furb/engine.py shows.", "t"))
  conductor.staff()
  assert [h[0] for h in conductor.hired] == ["free"]


def test_an_item_whose_model_no_worker_runs_waits_with_its_cause(conductor: Round) -> None:
  conductor.efforts["big"] = "claude-cli:opus/max"
  conductor.queue.append(("big", "b", "t"))
  conductor.staff()
  assert conductor.hired == []
  assert conductor.heard == [f"big waits: it asks claude-cli:opus/max, and a worker runs only {SONNET}, {OPUS}."]


def test_hire_starts_a_worker_on_its_chain_with_the_asks_on_top_and_its_workspace(conductor: Round) -> None:
  conductor.owner_asks["A3"] = {"prompt": "prompt30", "quote": "limit to 5 max", "items": ["cap"], "state": "open"}
  name = conductor.real_hire("cap", "Hold the cap.", "round")
  message, on = conductor.prompts[-1]
  path = f"{conductor.tmp}-w1"
  assert on == "chain-round/cap"
  assert message.startswith('This item carries the asks A3 of the owner, in his words:\n- A3: "limit to 5 max" (prompt30)\n')
  assert f"Work in the workspace {path}," in message
  assert 'labels=" --label w:cap --label thread:round"' in message
  assert message.endswith("The item, cap:\nHold the cap.")
  assert "{" + "workspace}" not in message and "echo chain-round/cap >" in message
  assert conductor.rungs == [(f"actor = {SONNET!r}", "chain-round/cap"), (f"try:\n  cd({path!r})\nexcept Refused:\n  pass", "chain-round/cap")]
  assert (conductor.working[name], conductor.models[name], conductor.homes["cap"]) == ("cap", SONNET, "chain-round/cap")
  assert conductor.slots[name] == conductor.slots["chain-round/cap"] == "w1"


def test_a_chain_goes_on_in_its_last_workspace_unless_a_live_worker_holds_it(conductor: Round) -> None:
  conductor.slots["chain-w/again"] = "w2"
  conductor.real_hire("again", "x")
  assert conductor.slots["chain-w/again"] == "w2"
  conductor.busy("prompt7", "other")
  conductor.slots["prompt7"] = "w3"
  conductor.slots["chain-w/third"] = "w3"
  conductor.real_hire("third", "x")
  assert conductor.slots["chain-w/third"] == "w1"


def test_an_audit_runs_in_no_workspace_and_a_fix_takes_its_brief_whole(conductor: Round) -> None:
  conductor.kinds["look"] = "audit"
  conductor.real_hire("look", "Count the lines.")
  assert "scratch://audit/look.md" in conductor.prompts[-1][0]
  assert len(conductor.rungs) == 1
  conductor.real_hire("fix4", "Fix #4 in {workspace}", "t")
  assert conductor.prompts[-1][0].startswith(f"Fix #4 in Work in the workspace {conductor.tmp}-w1,")


def test_each_brief_forbids_a_window_on_this_machine(conductor: Round) -> None:
  for brief in (conductor.WORKER, conductor.AUDIT, conductor.FIX, conductor.ANSWER):
    assert "Never open a window on this machine" in brief


def test_each_brief_sends_each_build_through_the_slot_of_the_machine(conductor: Round) -> None:
  for brief in (conductor.WORKER, conductor.AUDIT, conductor.FIX, conductor.ANSWER):
    assert "sh script/build-slot.sh <command>" in brief
    assert "no CARGO_BUILD_JOBS above 4" in brief
  assert "sh {repo}/script/build-slot.sh {repo}/script/worktree.sh {path}" in conductor.WORKSPACE


async def test_the_round_reads_the_free_memory_and_a_hire_counts_its_share_of_it(conductor: Round) -> None:
  conductor.ns["PROBE"] = "echo 7.5"
  await conductor.probe_memory()
  assert conductor.machine == {"free": 7.5}
  conductor.real_hire("a", "x")
  assert conductor.machine["free"] == 6.5
  conductor.ns["PROBE"] = "echo nonsense"
  with pytest.raises(Refused, match="the probe of the memory said nonsense"):
    await conductor.probe_memory()


def test_an_item_waits_while_the_machine_has_under_the_memory_that_a_hire_needs(conductor: Round) -> None:
  conductor.limits["room"] = {"sonnet": 5}
  conductor.queue.append(("q", "b", "t"))
  conductor.machine["free"] = 5.9
  conductor.staff()
  assert conductor.hired == []
  assert conductor.heard == ["q waits: the machine has under 6 GB of memory free."]
  conductor.machine["free"] = 6.0
  conductor.staff()
  assert [h[0] for h in conductor.hired] == ["q"]


async def test_a_red_pr_waits_for_a_fixer_while_the_machine_has_too_little_memory(conductor: Round) -> None:
  conductor.github(raw(5, "k5", (("the gates", "FAILURE"),)))
  conductor.ns["touched"].clear()
  conductor.machine["free"] = 1.0
  conductor.ns["held_keys"] = lambda: {"k5"}
  await conductor.sweep()
  assert conductor.hired == []
  assert "waits for a fixer" in conductor.heard[-1] and "GB of memory free" in conductor.heard[-1]


def test_a_worker_that_ends_with_a_pr_is_a_line_with_its_whole_report(conductor: Round) -> None:
  report = "I pushed one commit to main by mistake, then restored it."
  conductor.busy("prompt1", "k")
  conductor.end("prompt1", (40, report))
  conductor.tend()
  assert conductor.heard == [f"The worker k (prompt1) ended with #40: {report}"]
  assert conductor.ends == {"k": [("prompt1", 40)]}
  assert conductor.results["prompt1"] == (40, report)
  assert conductor.working == {}


def test_a_worker_that_ends_with_no_pr_or_is_cancelled_is_a_decision_and_starts_no_rerun(conductor: Round) -> None:
  conductor.busy("prompt1", "k")
  conductor.busy("prompt2", "j")
  conductor.end("prompt1", (0, "Nothing to change."))
  conductor.end("prompt2", asyncio.CancelledError())
  conductor.tend()
  assert conductor.notices() == [
    "The worker k (prompt1) ended with no PR, which needs your decision:\n\nNothing to change.",
    "The worker j (prompt2) ended with no PR, which needs your decision:\n\nCancelledError()",
  ]
  assert conductor.hired == []


def test_an_auditor_queues_each_new_row_of_its_register_once(conductor: Round) -> None:
  conductor.kinds["align8"] = "audit"
  conductor.ends["align8-old"] = [("prompt1", 3)]
  conductor.files["scratch://audit/align8.md"] = "\n".join([
    "| Dn | key | model | area | brief |",
    "| D1 | gap one | sonnet/high | src/furb/engine.py | Fix the gap. |",
    "| D2 | old | opus/max | tui | Old work. |",
    "| D3 | gap one | sonnet/high | tui | Twice. |",
  ])
  conductor.busy("prompt2", "align8", OPUS)
  conductor.end("prompt2", (0, "Two rows."))
  conductor.tend()
  key = "align8-gap-one"
  assert [q[0] for q in conductor.queue] == [key]
  assert conductor.queue[0][2] == key
  assert conductor.queue[0][1].endswith("Fix the gap.\n\nThis item is row D1 of the register scratch://audit/align8.md.")
  assert (conductor.efforts[key], conductor.areas[key], conductor.kinds[key]) == (SONNET, ["src/furb/engine.py"], "edit")
  assert conductor.heard == [
    "The auditor align8 (prompt2) ended: Two rows.",
    "The row D2 of the register of align8 is align8-old, which the round holds already, so it queued nothing for it.",
    f"The register of align8 queued {key}.",
  ]


def test_an_ask_that_a_worker_leaves_queues_one_follow_up_unless_another_item_carries_it(conductor: Round) -> None:
  conductor.owner_asks.update(
    A1={"prompt": "prompt1", "quote": "q1", "items": ["k"], "state": "open"},
    A2={"prompt": "prompt2", "quote": "q2", "items": ["k", "later"], "state": "open"},
  )
  conductor.queue.append(("later", "b", "t"))
  conductor.busy("prompt9", "k")
  conductor.end("prompt9", (5, "Served: A3\nLeft: A1 (no room), A2 (the item later)"))
  conductor.tend()
  assert [q[0] for q in conductor.queue] == ["later", "ask-a1-1"]
  assert conductor.owner_asks["A1"]["items"] == ["k", "ask-a1-1"]
  assert conductor.owner_asks["A1"]["state"] == "partly"
  assert conductor.owner_asks["A2"]["state"] == "open"
  assert json.loads(conductor.files["scratch://owner/asks.json"])[0]["id"] == "A1"
  conductor.queue.remove(conductor.queue[1])
  conductor.busy("prompt10", "ask-a1-1")
  conductor.end("prompt10", (0, "Left: A1 (still no room)"))
  conductor.tend()
  assert conductor.owner_asks["A1"]["state"] == "decision"
  assert conductor.notices()[-1].startswith("The follow-up ask-a1-1 of the ask A1 of the owner left it again")
  assert [q[0] for q in conductor.queue] == ["later"]


def test_a_message_of_the_owner_that_no_ask_names_is_one_notice(conductor: Round) -> None:
  conductor.owner_asks["A1"] = {"prompt": "prompt1, prompt3", "items": [], "state": "done"}
  conductor.facts_of["chain1"] = [
    ("prompt", "prompt1", "operator", "chain1", "str", "a"),
    ("prompt", "prompt2", "operator/uael", "chain1", "str", "b"),
    ("prompt", "prompt3", "operator", "chain1", "str", "c"),
    ("prompt", "prompt4", "rung5", "chain1", "str", "d"),
  ]
  conductor.asked()
  conductor.asked()
  assert conductor.notices() == [
    "The message prompt2 of the owner holds no ask in owner_asks. Split it into its asks, each with his quote and the items that carry it, before anything else."
  ]


def test_an_ask_of_the_owner_that_no_live_item_carries_is_one_notice(conductor: Round) -> None:
  conductor.owner_asks.update(
    A1={"prompt": "prompt1", "items": [], "state": "missing"},
    A2={"prompt": "prompt2", "items": ["gone"], "state": "partly"},
    A3={"prompt": "prompt3", "items": ["q"], "state": "open"},
    A4={"prompt": "prompt4", "items": [], "state": "done"},
    A5={"prompt": "prompt5", "items": ["k"], "state": "decision"},
  )
  conductor.queue.append(("q", "b", "t"))
  conductor.asked()
  conductor.asked()
  assert conductor.notices() == [
    "The ask A1 of the owner (prompt1) is missing, and no item names it. Queue an item for it, or mark it done with the evidence of main, before anything else.",
    "The ask A2 of the owner (prompt2) is partly, and no item carries it now: its items gone ended. Queue an item for it, or mark it done with the evidence of main, before anything else.",
  ]
  conductor.owner_asks["A2"]["items"].append("again")
  conductor.asked()
  assert len(conductor.notices()) == 3

def test_the_words_of_the_owner_hold_his_messages_and_his_answers_in_order(conductor: Round) -> None:
  conductor.facts_of["chain1"] = [
    ("prompt", "prompt1", "operator", "chain1", "str", "Do it.", ""),
    ("prompt", "prompt2", "rung3", "chain1", "str", "Which one?", "operator"),
    ("prompt", "prompt3", "rung3", "chain1", "str", "Which?", "claude-cli:opus"),
  ]
  conductor.done.update(prompt2="a", prompt3="b")
  conductor.owner_words()
  assert conductor.files["scratch://owner/words.md"] == (
    "# The words of the owner in this session, verbatim, in order\n\n## prompt1, his message\n\nDo it.\n\n"
    "## prompt2, his answer to my question\n\na\n\n"
  )


def test_an_audit_of_alignment_starts_at_eight_merges_with_the_next_free_key(conductor: Round) -> None:
  conductor.align()
  assert conductor.align_state == {"base": "0"}
  conductor.merged.update(range(7))
  conductor.align()
  assert conductor.queue == []
  conductor.merged.add(7)
  conductor.ends.update({f"align{n}": [("p", 0)] for n in range(1, 8)})
  conductor.lead["main"] = "abc"
  conductor.align()
  key, brief, thread = conductor.queue[0]
  assert (key, thread, conductor.kinds[key]) == ("align8", "align", "audit")
  assert "scratch://owner/words.md, scratch://owner/picks.md" in brief and "scratch://owner/asks.json" in brief
  assert "git log --oneline -40 origin/main" in brief and "align7" in brief
  assert conductor.align_state == {"base": "8", "n": "8", "sha": "abc"}
  conductor.merged.update(range(8, 20))
  conductor.align()
  assert len(conductor.queue) == 1


@POSIX
@JQ
async def test_the_round_turns_on_auto_merge_for_its_prs_behind_the_ruleset_and_off_for_a_held_one(conductor: Round) -> None:
  conductor.ends.update(a=[], b=[], c=[], d=[])
  boxed = raw(4, "d", body="- [ ] Its prose obeys AGENTS.md")
  conductor.github(raw(1, "a"), raw(2, "b", labels=("hold",), autoMergeRequest={}), raw(3, "c", isDraft=True), boxed, raw(5, "other"))
  await conductor.sweep()
  calls = [x for x in conductor.log().splitlines() if x.startswith("pr merge")]
  assert calls == ["pr merge 1 --auto --squash", "pr merge 2 --disable-auto"]
  assert conductor.touched == {n: {"sha": f"sha{n}", "paths": []} for n in (1, 2, 3, 4)}


@POSIX
@JQ
async def test_with_no_ruleset_on_main_the_round_turns_on_no_auto_merge_and_tells_once(conductor: Round) -> None:
  conductor.ends["a"] = []
  conductor.github(raw(1, "a"), ruled=[])
  await conductor.sweep()
  await conductor.sweep()
  assert "pr merge" not in conductor.log()
  assert conductor.notices() == ["main has no ruleset that requires the check the gates, so the round turns on no auto-merge and merges no PR."]


@POSIX
@JQ
async def test_a_pr_of_the_round_that_merged_is_named_in_the_next_notice(conductor: Round) -> None:
  conductor.ends.update(a=[], b=[])
  conductor.github(raw(1, "a"), raw(2, "b"))
  await conductor.sweep()
  conductor.github(raw(2, "b"), merged=(1, 99))
  await conductor.sweep()
  assert conductor.landed == [1]
  assert conductor.merged == {1, 99}
  assert 1 not in conductor.touched


@POSIX
@JQ
async def test_a_red_pr_gets_one_order_to_its_live_maker_then_a_fixer_after_owned(conductor: Round) -> None:
  conductor.busy("prompt1", "a")
  conductor.github(raw(1, "a", checks=(("the gates", "FAILURE"),), labels=("thread:t",)))
  await conductor.sweep()
  await conductor.sweep()
  assert [on for _, on in conductor.prompts] == ["chain-a"]
  assert conductor.prompts[0][0].startswith("Your open PR #1 has a failed check at sha1.")
  conductor.now += conductor.OWNED
  await conductor.sweep()
  assert [h[0] for h in conductor.hired] == ["fix1"]
  assert conductor.hired[0][2] == "t"
  assert conductor.assigned[1]["fixer"] and conductor.assigned[1]["tries"] == 1


@POSIX
@JQ
async def test_a_red_pr_with_no_live_maker_gets_at_most_two_fixers_for_a_head_then_a_notice(conductor: Round) -> None:
  conductor.ends["a"] = [("prompt1", 1)]
  conductor.github(raw(1, "a", mergeable="CONFLICTING", checks=()))
  for _ in range(4):
    await conductor.sweep()
    for name in conductor.living():
      conductor.end(name, (1, "no move"))
  assert [h[0] for h in conductor.hired] == ["fix1", "fix1"]
  assert "it conflicts with its base at sha1" in conductor.hired[0][1]
  assert conductor.notices() == ["#1 conflicts with its base at sha1, and 2 fixers ended at this head and did not move it, so the round sends no third one."]


@POSIX
@JQ
async def test_a_red_main_sends_no_fixer_to_a_red_pr_and_tells_once(conductor: Round) -> None:
  conductor.ends["a"] = []
  conductor.github(raw(1, "a", checks=(("the gates", "FAILURE"),)), main="failure")
  await conductor.sweep()
  await conductor.sweep()
  assert conductor.hired == []
  assert len(conductor.notices()) == 1
  assert conductor.notices()[0].startswith("main is red at m1: find its cause first.")
  assert conductor.heard == ["#1 is red at sha1 while main is red, so it waits for main."]


@POSIX
@JQ
async def test_a_pr_whose_checks_still_run_is_no_red_pr(conductor: Round) -> None:
  conductor.ends["a"] = []
  conductor.github(raw(1, "a", checks=(("the gates", ""), ("ruff", "FAILURE"))))
  await conductor.sweep()
  assert conductor.prs[1]["state"] == "running"
  assert conductor.hired == []


@POSIX
@JQ
async def test_a_sweep_that_gh_fails_raises_refused(conductor: Round) -> None:
  conductor.say(r"pr list --state open", "", 1)
  with pytest.raises(Refused, match="gh listed no PRs"):
    await conductor.sweep()


@POSIX
@JQ
async def test_a_comment_of_the_owner_goes_to_the_worker_on_the_pr_and_holds_it(conductor: Round) -> None:
  conductor.busy("prompt1", "a")
  conductor.ends["b"] = []
  conductor.github(raw(1, "a"), raw(2, "b"), raw(3, "foreign"))
  await conductor.sweep()
  conductor.comments.add("seen")
  conductor.say(r"pulls/1/comments", [{"pull_request_url": "x/pulls/1", "id": 10, "user": {"login": "uael"}, "path": "a.py", "line": 3, "body": "Name it well."}])
  conductor.say(r"issues/2/comments", [{"issue_url": "x/issues/2", "id": 11, "user": {"login": "uael"}, "body": "And this."}, {"issue_url": "x/issues/2", "id": 12, "user": {"login": "uael"}, "body": "🤖 mine"}])
  await conductor.review()
  assert conductor.prompts[0] == ("A new comment of the owner on your PR #1. Do what it asks, answer it on GitHub with a reply that starts with 🤖, push the fix, and take the label hold off.\n#1 c10 a.py:3: Name it well.", "chain-a")
  key, brief, _ = conductor.queue[0]
  assert key == "fix2" and "#2 i11 -: And this." in brief and "w/b" in brief
  assert "pulls/3/" not in conductor.log()
  assert "pr edit 1 --add-label hold" in conductor.log() and "pr edit 2 --add-label hold" in conductor.log()
  assert conductor.comments == {"seen", "c10", "i11", "i12"}


@POSIX
@JQ
async def test_the_first_read_of_the_comments_sends_nothing(conductor: Round) -> None:
  conductor.busy("prompt1", "a")
  conductor.github(raw(1, "a"))
  await conductor.sweep()
  conductor.say(r"pulls/1/comments", [{"pull_request_url": "x/pulls/1", "id": 10, "user": {"login": "uael"}, "path": "a", "line": 1, "body": "old"}])
  await conductor.review()
  assert conductor.prompts == []
  assert conductor.comments == {"c10"}


@POSIX
@JQ
async def test_a_step_that_fails_is_one_notice_and_the_round_goes_on_until_it_holds_nothing(conductor: Round) -> None:
  conductor.say(r"pr list --state open", "", 1)
  conductor.queue.append(("one", "b", "t"))
  steps = iter([False, True])

  async def wait(seconds: float = 0, *acts: str, on: str = "") -> None:
    for name in list(conductor.living()):
      conductor.end(name, (3, "done"))
    if next(steps):
      conductor.github()

  conductor.ns["wait"] = wait
  got = await conductor.conduct()
  assert got == {"worker1": (3, "done")}
  told = conductor.notices()
  assert told[0].startswith("The round on chain1 met a fault in its step sweep: Refused: gh listed no PRs")
  assert len(told) == 2
  assert told[1].startswith("The round on chain1 ended: it holds no item and no open PR that it tends.")
  assert "The worker one (worker1) ended with #3: done" in told[1]
