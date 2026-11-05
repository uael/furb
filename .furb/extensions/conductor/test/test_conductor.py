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
SONNET, OPUS, ASTRA = "claude-cli:sonnet/high", "claude-cli:opus/medium", "codex:gpt-6-astra/high"
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
    self.stopped: set[str] = set()
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
    ns.update(peek=lambda name, waiting=None: self.done.get(name, waiting), read=self.read, write=self.write, paused=lambda name: name in self.stopped)
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
    ns.update(quietly=self.quietly, hire=self.hire, PROBE="echo 64", KEEP=0)
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

  async def hire(self, key: str, what: str, thread: str = "", brief: str = "", model: str = "") -> str:
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


def test_a_paused_worker_gives_its_slot_to_the_room_but_stays_a_living_worker(conductor: Round) -> None:
  conductor.limits["room"] = {"sonnet": 2}
  conductor.busy("w1", "k1")
  conductor.busy("w2", "k2")
  assert not conductor.room(SONNET)
  conductor.stopped.add("w1")
  assert conductor.room(SONNET)
  assert conductor.living() == ["w1", "w2"]
  assert conductor.running() == ["w2"]
  conductor.stopped.clear()
  assert not conductor.room(SONNET)


def test_a_load_with_no_limits_binds_the_room_of_the_owner_and_caps_at_ten(tmp_path: Path) -> None:
  got = Round(tmp_path)
  del got.ns["limits"]
  exec(compile(IMPORT.sub("", SOURCE), str(HERE / "conductor.py"), "exec"), got.ns)  # noqa: S102
  assert got.limits["room"] == {"opus/high": 3, "opus/medium": 3, "sonnet/high": 3, "sonnet/medium": 3, "gpt-6-astra/high": 1}
  assert got.limits["models"]["astra"] == ASTRA
  assert got.ALIGN_EVERY == 4
  assert got.cap() == 10
  assert got.family(ASTRA) == "gpt-6-astra/high"
  assert got.GATE == "the gates"
  assert got.family("claude-cli:opus/high") == "opus/high"


async def test_staff_hires_queue_then_backlog_in_order_and_keeps_one_slot_for_a_fixer(conductor: Round) -> None:
  conductor.queue.extend([("q1", "b", "t"), ("q2", "b", "t")])
  conductor.backlog.extend([("b1", "b", "t"), ("b2", "b", "t"), ("b3", "b", "t"), ("b4", "b", "t")])
  conductor.limits["room"] = {"sonnet": 5}
  await conductor.staff()
  assert [h[0] for h in conductor.hired] == ["q1", "q2", "b1", "b2"]
  assert conductor.backlog == [("b3", "b", "t"), ("b4", "b", "t")]


async def test_an_item_waits_for_its_after_until_each_pr_of_it_merged_and_says_why_once(conductor: Round) -> None:
  conductor.queue.append(("late", "b", "t"))
  conductor.after["late"] = ["first"]
  await conductor.staff()
  await conductor.staff()
  assert conductor.hired == []
  assert conductor.heard == ["late waits: first has not ended, and no item holds it."]
  conductor.ends["first"] = [("prompt1", 12)]
  await conductor.staff()
  assert conductor.heard[-1] == "late waits: #12 of first has not merged."
  conductor.merged.add(12)
  await conductor.staff()
  assert [h[0] for h in conductor.hired] == ["late"]


async def test_an_item_waits_while_a_live_worker_holds_a_file_that_changes_alone_and_a_test_file_holds_none(conductor: Round) -> None:
  conductor.areas.update(one=["src/furb/engine.py", "test/conftest.py"], two=["src/furb", "test/conftest.py"], three=["test/conftest.py"])
  conductor.busy("prompt1", "one")
  conductor.queue.extend([("two", "b", "t"), ("three", "b", "t")])
  await conductor.staff()
  assert [h[0] for h in conductor.hired] == ["three"]
  assert conductor.heard == ["two waits: the worker of one holds src/furb/engine.py."]


async def test_an_item_waits_while_an_open_pr_of_the_round_changes_a_file_that_changes_alone_but_not_for_its_own_pr(conductor: Round) -> None:
  conductor.areas.update(mine=["DOCTRINE.md"], other=["DOCTRINE.md"], shared=["tui/src"])
  conductor.prs[9] = {"n": 9, "labels": ["w:mine"]}
  conductor.touched[9] = {"sha": "s", "paths": ["DOCTRINE.md", "tui/src/app.ts"]}
  conductor.queue.extend([("other", "b", "t"), ("mine", "b", "t"), ("shared", "b", "t")])
  await conductor.staff()
  assert [h[0] for h in conductor.hired] == ["mine", "shared"]
  assert conductor.heard == ["other waits: #9 changes DOCTRINE.md."]


async def test_an_item_with_no_areas_holds_no_file_whatever_its_brief_names(conductor: Round) -> None:
  conductor.prs[9] = {"n": 9, "labels": ["w:x"]}
  conductor.touched[9] = {"sha": "s", "paths": ["src/furb/engine.py"]}
  conductor.queue.append(("free", "Change src/furb/engine.py, as https://github.com/a/b/blob/main/src/furb/engine.py shows.", "t"))
  await conductor.staff()
  assert [h[0] for h in conductor.hired] == ["free"]


async def test_an_item_whose_model_no_worker_runs_waits_with_its_cause(conductor: Round) -> None:
  conductor.efforts["big"] = "claude-cli:opus/max"
  conductor.queue.append(("big", "b", "t"))
  await conductor.staff()
  assert conductor.hired == []
  assert conductor.heard == [f"big waits: it asks claude-cli:opus/max, and a worker runs only {SONNET}, {OPUS}."]


async def test_hire_starts_a_worker_on_its_chain_with_the_asks_on_top_and_its_workspace(conductor: Round) -> None:
  conductor.owner_asks["A3"] = {"prompt": "prompt30", "quote": "limit to 5 max", "items": ["cap"], "state": "open"}
  name = await conductor.real_hire("cap", "Hold the cap.", "round")
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


async def test_a_chain_goes_on_in_its_last_workspace_unless_a_live_worker_holds_it(conductor: Round) -> None:
  conductor.slots["chain-w/again"] = "w2"
  await conductor.real_hire("again", "x")
  assert conductor.slots["chain-w/again"] == "w2"
  conductor.busy("prompt7", "other")
  conductor.slots["prompt7"] = "w3"
  conductor.slots["chain-w/third"] = "w3"
  await conductor.real_hire("third", "x")
  assert conductor.slots["chain-w/third"] == "w1"


async def test_an_audit_runs_in_no_workspace_and_a_fix_takes_its_brief_whole(conductor: Round) -> None:
  conductor.kinds["look"] = "audit"
  await conductor.real_hire("look", "Count the lines.")
  assert "scratch://audit/look.md" in conductor.prompts[-1][0]
  assert len(conductor.rungs) == 1
  await conductor.real_hire("fix4", "Fix #4 in {workspace}", "t")
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
  await conductor.real_hire("a", "x")
  assert conductor.machine["free"] == 6.5
  conductor.ns["PROBE"] = "echo nonsense"
  with pytest.raises(Refused, match="the probe of the memory said nonsense"):
    await conductor.probe_memory()


async def test_an_item_waits_while_the_machine_has_under_the_memory_that_a_hire_needs(conductor: Round) -> None:
  conductor.limits["room"] = {"sonnet": 5}
  conductor.queue.append(("q", "b", "t"))
  conductor.machine["free"] = 5.9
  await conductor.staff()
  assert conductor.hired == []
  assert conductor.heard == ["q waits: the machine has under 6 GB of memory free."]
  conductor.machine["free"] = 6.0
  await conductor.staff()
  assert [h[0] for h in conductor.hired] == ["q"]


async def test_a_red_pr_waits_for_a_fixer_while_the_machine_has_too_little_memory(conductor: Round) -> None:
  conductor.github(raw(5, "k5", (("the gates", "FAILURE"),)))
  conductor.ns["touched"].clear()
  conductor.machine["free"] = 1.0
  conductor.ns["held_keys"] = lambda: {"k5"}
  await conductor.sweep()
  assert conductor.hired == []
  assert "waits for a fixer" in conductor.heard[-1] and "GB of memory free" in conductor.heard[-1]


def test_conducting_and_audit_hold_the_rules_of_rank_and_of_the_frozen_fixes(conductor: Round) -> None:
  rule = " ".join(conductor.CONDUCTING.split())
  assert "does not become its own item or PR" in rule
  assert "waits in scratch://round/frozen.md" in rule
  assert "land together in one change" in rule
  assert "in the order of the register after the items of queue" in rule
  assert "unless the coordinator asks" in rule
  audit = " ".join(conductor.AUDIT.split())
  assert "the hard work first by prompt266" in audit
  assert "reshapes a module, a contract or a protocol ranks before a row of polish or a simple row" in audit


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


def test_a_message_of_the_owner_that_no_ask_names_is_one_line_of_a_notice(conductor: Round) -> None:
  conductor.owner_asks["A1"] = {"prompt": "prompt1, prompt3", "items": [], "state": "done"}
  conductor.facts_of["chain1"] = [
    ("prompt", "prompt1", "operator", "chain1", "str", "a"),
    ("prompt", "prompt2", "operator/uael", "chain1", "str", "b"),
    ("prompt", "prompt3", "operator", "chain1", "str", "c"),
    ("prompt", "prompt4", "rung5", "chain1", "str", "d"),
  ]
  conductor.asked()
  conductor.asked()
  [notice] = conductor.notices()
  assert "\n- The message prompt2 of the owner holds no ask in owner_asks.\n" in notice
  assert notice.count("\n- ") == 1


def test_an_ask_of_the_owner_that_no_live_item_carries_is_one_line_of_a_notice(conductor: Round) -> None:
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
  [notice] = conductor.notices()
  assert (
    "\n- The ask A1 of the owner (prompt1) is missing, and no item names it."
    "\n- The ask A2 of the owner (prompt2) is partly, and no item carries it now: its items gone ended.\n"
  ) in notice
  assert notice.count("\n- ") == 2
  conductor.owner_asks["A2"]["items"].append("again")
  conductor.asked()
  assert len(conductor.notices()) == 1
  conductor.end("prompt101", None)
  conductor.asked()
  assert len(conductor.notices()) == 2
  assert "its items gone, again ended" in conductor.notices()[1]
  assert conductor.notices()[1].count("\n- ") == 1


def test_a_round_with_five_uncarried_asks_gives_one_notice_and_the_next_none_while_it_stands_open(conductor: Round) -> None:
  for i in range(1, 6):
    conductor.owner_asks[f"A{i}"] = {"prompt": f"prompt{i}", "items": [], "state": "open"}
  conductor.asked()
  assert len(conductor.notices()) == 1
  assert all(f"The ask A{i} of the owner" in conductor.notices()[0] for i in range(1, 6))
  conductor.owner_asks["A6"] = {"prompt": "prompt6", "items": [], "state": "open"}
  conductor.asked()
  assert len(conductor.notices()) == 1
  conductor.end("prompt101", None)
  conductor.asked()
  assert len(conductor.notices()) == 2
  assert "The ask A6 of the owner" in conductor.notices()[1]
  assert "The ask A1 of the owner" not in conductor.notices()[1]


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


def test_an_audit_of_alignment_starts_at_four_merges_on_astra_and_the_model_of_audit_in_turn(conductor: Round) -> None:
  conductor.limits["models"]["astra"] = ASTRA
  conductor.align()
  assert conductor.align_state == {"base": "0"}
  conductor.merged.update(range(3))
  conductor.align()
  assert conductor.queue == []
  conductor.merged.add(3)
  conductor.ends.update({f"align{n}": [("p", 0)] for n in range(1, 8)})
  conductor.lead["main"] = "abc"
  conductor.align()
  key, brief, thread = conductor.queue[0]
  assert (key, thread, conductor.kinds[key], conductor.model_of(key)) == ("align8", "align", "audit", OPUS)
  assert "scratch://owner/words.md, scratch://owner/picks.md" in brief and "scratch://owner/asks.json" in brief
  assert "git log --oneline -40 origin/main" in brief and "align7" in brief
  assert conductor.align_state == {"base": "4", "n": "8", "sha": "abc"}
  conductor.merged.update(range(4, 20))
  conductor.align()
  assert len(conductor.queue) == 1
  conductor.queue.clear()
  conductor.ends["align8"] = [("p", 0)]
  conductor.align()
  assert [q[0] for q in conductor.queue] == ["align9"]
  assert conductor.model_of("align9") == ASTRA


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
async def test_a_pr_of_a_live_maker_that_turns_conflicting_tells_the_maker_once_for_each_head_and_sends_no_fixer(conductor: Round) -> None:
  conductor.busy("prompt1", "a")
  conductor.facts_of["chain-a"] = [("bash", "bash7", "prompt1", "chain-a")]
  conductor.github(raw(1, "a", labels=("thread:t",)))
  await conductor.sweep()
  assert conductor.prompts == []
  conductor.github(raw(1, "a", mergeable="CONFLICTING", checks=(), labels=("thread:t",)))
  await conductor.sweep()
  await conductor.sweep()
  assert [on for _, on in conductor.prompts] == ["chain-a"]
  assert conductor.prompts[0][0].startswith("Your open PR #1 conflicts with its base at sha1. Fix it before any other step:")
  assert conductor.hired == []
  assert conductor.guard_faults() == []
  conductor.now += conductor.UNMOVED - 1
  await conductor.sweep()
  assert conductor.notices() == []
  conductor.now += 1
  await conductor.sweep()
  await conductor.sweep()
  assert conductor.notices() == ["#1 conflicts with its base at sha1, and prompt1, whom the round told of it, has not moved its head in 30 minutes. Its last act: bash bash7."]
  assert conductor.hired == []
  conductor.github(raw(1, "a", mergeable="CONFLICTING", checks=(), headRefOid="sha9", labels=("thread:t",)))
  await conductor.sweep()
  await conductor.sweep()
  assert [m[:36] for m, on in conductor.prompts if on == "chain-a"] == ["Your open PR #1 conflicts with its b"] * 2
  assert "at sha9." in conductor.prompts[-1][0]
  assert conductor.hired == []


@POSIX
@JQ
async def test_a_red_pr_whose_told_maker_ends_is_flagged_by_the_guard_and_goes_to_a_fixer_at_the_next_sweep(conductor: Round) -> None:
  conductor.busy("prompt1", "a")
  conductor.github(raw(1, "a", checks=(("the gates", "FAILURE"),), labels=("thread:t",)))
  await conductor.sweep()
  assert conductor.prompts[0][0].startswith("Your open PR #1 has a failed check at sha1.")
  conductor.end("prompt1", (1, "pushed"))
  assert conductor.guard_faults() == ["#1 has a failed check at sha1, and no worker that knows of it acts"]
  await conductor.sweep()
  assert [h[0] for h in conductor.hired] == ["fix1"]
  assert conductor.hired[0][2] == "t"
  assert conductor.assigned[1]["fixer"] and conductor.assigned[1]["tries"] == 1
  assert conductor.guard_faults() == []


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


def pushed(conductor: Round, *branches: str) -> dict[str, str]:
  def git(*args: str, where: Path | None = None) -> str:
    cmd = ["git", "-c", "user.name=t", "-c", "user.email=t@t", *args]
    return subprocess.run(cmd, cwd=where or conductor.tmp, env=conductor.env, capture_output=True, text=True, check=True).stdout.strip()  # noqa: S603

  git("init", "-q", "--bare", "remote.git")
  git("init", "-q")
  git("remote", "add", "origin", "remote.git")
  heads = {}
  for n, branch in enumerate(branches):
    git("commit", "--allow-empty", "-qm", f"c{n}")
    git("push", "-q", "origin", f"HEAD:refs/heads/{branch}")
    heads[branch] = git("rev-parse", "HEAD")
  return heads


@POSIX
@JQ
async def test_a_pr_whose_head_lags_its_branch_for_two_sweeps_is_closed_and_reopened_once_for_that_head(conductor: Round) -> None:
  conductor.ends["a"] = []
  conductor.ends["b"] = []
  heads = pushed(conductor, "w/a", "w/b")
  conductor.github(raw(1, "a"), raw(2, "b", headRefOid=heads["w/b"]))
  calls = []
  for _ in range(4):
    await conductor.sweep()
    calls.append([x for x in conductor.log().splitlines() if x.startswith(("pr close", "pr reopen"))])
  assert calls[0] == []
  assert calls[1] == calls[2] == calls[3] == ["pr close 1", "pr reopen 1"]
  assert conductor.heard == [f"#1 kept the head sha1 for two sweeps while its branch w/a holds {heads['w/a'][:7]} on origin, so the round closed and reopened it, once for this head."]
  conductor.github(raw(1, "a", headRefOid=heads["w/a"]), raw(2, "b", headRefOid=heads["w/b"]))
  await conductor.sweep()
  assert conductor.lagged == {}


@POSIX
@JQ
async def test_a_red_pr_whose_failed_checks_main_fails_too_gets_no_fixer_and_one_line(conductor: Round) -> None:
  conductor.ends["a"] = []
  conductor.ends["b"] = []
  conductor.github(raw(1, "a", checks=(("the gates", "FAILURE"), ("bench", "FAILURE"))), raw(2, "b", checks=(("the gates", "FAILURE"), ("ruff", "FAILURE"))))
  conductor.say(r"commits/main/check-runs", {"check_runs": [{"name": "the gates", "conclusion": "failure"}, {"name": "bench", "conclusion": "failure"}, {"name": "ruff", "conclusion": "success"}]})
  await conductor.sweep()
  await conductor.sweep()
  assert [h[0] for h in conductor.hired] == ["fix2"]
  assert conductor.heard == ["#1 is red at sha1 only on bench, the gates, which main fails too, so the round hires no fixer for it."]


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


@POSIX
@JQ
async def test_a_pr_that_refuses_auto_merge_merges_plainly_or_as_a_layer_of_its_stack(conductor: Round) -> None:
  conductor.ends["a"] = []
  conductor.github(raw(1, "a"))
  conductor.say(r"pr merge 1 --auto", "", 1)
  conductor.say(r"pr merge 1 --squash$", "", 1)
  await conductor.sweep()
  calls = [x for x in conductor.log().splitlines() if x.startswith(("pr merge", "stack merge"))]
  assert calls == ["pr merge 1 --auto --squash", "pr merge 1 --squash", "stack merge 1 --yes --squash"]


@POSIX
@JQ
async def test_a_green_pr_that_has_not_merged_after_stale_seconds_is_one_line(conductor: Round) -> None:
  conductor.ends["a"] = []
  conductor.github(raw(1, "a"))
  await conductor.sweep()
  conductor.now += conductor.STALE + 1
  await conductor.sweep()
  await conductor.sweep()
  assert conductor.heard == ["#1 is green at sha1 for 45 minutes and has not merged."]


@POSIX
@JQ
async def test_a_fixer_of_a_full_family_runs_on_the_next_model_of_upgrade(conductor: Round) -> None:
  conductor.ends["a"] = [("prompt1", 1)]
  for n in range(3):
    conductor.busy(f"prompt{n + 10}", f"k{n}")
  conductor.github(raw(1, "a", mergeable="CONFLICTING", checks=()))
  await conductor.sweep()
  assert [h[0] for h in conductor.hired] == ["fix1"]
  assert conductor.models["worker1"] == OPUS


async def test_an_item_of_a_full_family_moves_to_the_next_model_of_upgrade(conductor: Round) -> None:
  conductor.busy("prompt1", "one")
  conductor.limits["room"] = {"sonnet": 1, "opus": 2}
  conductor.queue.append(("two", "b", "t"))
  await conductor.staff()
  assert [h[0] for h in conductor.hired] == ["two"]
  assert conductor.efforts["two"] == OPUS


async def test_a_wait_on_an_item_that_has_not_ended_is_no_line(conductor: Round) -> None:
  conductor.efforts["dep"] = "claude-cli:opus/max"
  conductor.queue.extend([("dep", "b", "t"), ("late", "b", "t")])
  conductor.after["late"] = ["dep"]
  await conductor.staff()
  assert conductor.hired == []
  assert conductor.heard == [f"dep waits: it asks claude-cli:opus/max, and a worker runs only {SONNET}, {OPUS}."]


def test_a_register_with_a_column_after_sets_the_waits_of_each_row(conductor: Round) -> None:
  conductor.ends["triage-old"] = []
  conductor.files["scratch://audit/look.md"] = "\n".join([
    "| Dn | key | model | area | after | brief |",
    "| D1 | old | sonnet/high | src | | Done. |",
    "| D2 | base | sonnet/high | src | | Build it. |",
    "| D3 | top | sonnet/high | tui | D1, D2 | Use it. |",
  ])
  conductor.register("look")
  assert [q[0] for q in conductor.queue] == ["triage-base", "triage-top"]
  assert conductor.queue[1][1].endswith("Use it.\n\nThis item is row D3 of the register scratch://audit/look.md.")
  assert conductor.after == {"triage-top": ["triage-old", "triage-base"]}


def git(where: Path, *words: str) -> str:
  return subprocess.run(["git", "-C", str(where), *words], capture_output=True, text=True, check=True).stdout.strip()


@POSIX
def test_settle_keeps_the_work_of_another_chain_on_a_new_branch_and_never_on_the_branch_of_a_pr(conductor: Round, tmp_path: Path) -> None:
  origin, space = tmp_path / "origin.git", tmp_path / "w2"
  subprocess.run(["git", "init", "-q", "--bare", str(origin)], check=True)
  subprocess.run(["git", "clone", "-q", str(origin), str(space)], check=True, capture_output=True)
  git(space, "config", "user.email", "t@t")
  git(space, "config", "user.name", "t")
  git(space, "switch", "-q", "-c", "w/pr")
  (space / "a.txt").write_text("a\n", encoding="utf-8")
  git(space, "add", "a.txt")
  git(space, "commit", "-q", "-m", "a")
  git(space, "push", "-q", "origin", "w/pr")
  pushed = git(space, "rev-parse", "HEAD")
  (space / ".git" / "furb-holder").write_text("chain-old\n", encoding="utf-8")
  (space / "b.txt").write_text("foreign\n", encoding="utf-8")
  subprocess.run(["sh", "-c", conductor.SETTLE, "sh", str(space), "chain-new"], check=True, capture_output=True)
  assert git(origin, "rev-parse", "w/pr") == pushed
  wip = [x.split()[-1] for x in git(origin, "branch", "--list", "w/wip-*").splitlines()]
  assert len(wip) == 1 and wip[0].startswith(f"w/wip-{pushed[:7]}-")
  assert git(origin, "show", f"{wip[0]}:b.txt") == "foreign"
  assert (space / ".git" / "furb-holder").read_text(encoding="utf-8") == "chain-new\n"


@POSIX
async def test_hire_writes_the_chain_of_the_worker_in_its_workspace_before_its_prompt(conductor: Round) -> None:
  space = Path(f"{conductor.tmp}-w1")
  subprocess.run(["git", "init", "-q", str(space)], check=True)
  (space / ".git" / "furb-holder").write_text("chain-old\n", encoding="utf-8")
  seen = []
  ask = conductor.ns["prompt"]

  def prompt(shape: object, message: str = "", to: str = "", on: str = "") -> str:
    seen.append((space / ".git" / "furb-holder").read_text(encoding="utf-8"))
    return ask(shape, message, to, on)

  conductor.ns["prompt"] = prompt
  await conductor.real_hire("cap", "x")
  assert seen == ["chain-w/cap\n"]


def test_a_worker_whose_chain_says_no_fact_for_quiet_seconds_is_no_live_worker_and_one_notice(conductor: Round) -> None:
  conductor.busy("prompt1", "a")
  conductor.facts_of["chain-a"] = [("bash", "bash7", "prompt1", "chain-a")]
  conductor.tend()
  assert conductor.running() == ["prompt1"]
  conductor.now += conductor.QUIET
  conductor.tend()
  conductor.tend()
  assert conductor.running() == [] and conductor.living() == ["prompt1"]
  assert conductor.notices() == ["The worker a (prompt1) said no fact on its chain for 30 minutes, so the round counts it as no live worker. Its open acts: bash7."]
  conductor.facts_of["chain-a"].append(("wait", "wait2", "prompt1", "chain-a"))
  conductor.tend()
  assert conductor.running() == ["prompt1"]
  conductor.end("prompt1", (1, "r"))
  conductor.tend()
  assert conductor.progress == {}


@POSIX
async def test_a_hire_takes_no_workspace_whose_holder_chain_has_a_live_worker(conductor: Round) -> None:
  conductor.busy("prompt7", "tui")
  space = Path(f"{conductor.tmp}-w1")
  subprocess.run(["git", "init", "-q", str(space)], check=True)
  (space / ".git" / "furb-holder").write_text("chain-tui\n", encoding="utf-8")
  await conductor.real_hire("names", "x")
  assert conductor.slots["chain-w/names"] == "w2"
  assert (space / ".git" / "furb-holder").read_text(encoding="utf-8") == "chain-tui\n"


async def test_an_item_of_a_full_opus_medium_waits_for_its_family_and_never_moves_up_to_opus_high(conductor: Round) -> None:
  high = "claude-cli:opus/high"
  conductor.limits["room"] = {"opus/medium": 1, "opus/high": 3}
  conductor.limits["models"].update(audit=high, lead=OPUS)
  conductor.efforts.update(two=OPUS, core=high)
  conductor.busy("prompt1", "one", OPUS)
  conductor.queue.extend([("two", "b", "t"), ("core", "b", "t")])
  await conductor.staff()
  assert [h[0] for h in conductor.hired] == ["core"]
  assert conductor.efforts["two"] == OPUS
  assert conductor.heard == ["two waits: opus/medium has no room."]


@POSIX
async def test_the_disk_frees_the_target_of_the_workspace_that_ended_first_and_tells_once_when_it_stays_short(conductor: Round) -> None:
  for slot in ("w1", "w2", "w3"):
    (Path(f"{conductor.tmp}-{slot}") / "target").mkdir(parents=True)
  for n in (1, 2, 3):
    conductor.busy(f"prompt{n}", f"k{n}")
    conductor.slots[f"prompt{n}"] = f"w{n}"
  conductor.end("prompt3", (1, "r"))
  conductor.tend()
  conductor.now += 100
  conductor.end("prompt2", (2, "r"))
  conductor.tend()
  await conductor.keep_disk()
  assert all((Path(f"{conductor.tmp}-w{n}") / "target").exists() for n in (1, 2, 3))
  conductor.ns["KEEP"] = 10**9
  await conductor.keep_disk()
  assert (Path(f"{conductor.tmp}-w3") / "target").exists()
  conductor.now += conductor.DISK_EVERY
  await conductor.keep_disk()
  conductor.now += conductor.DISK_EVERY
  await conductor.keep_disk()
  assert [(Path(f"{conductor.tmp}-w{n}") / "target").exists() for n in (1, 2, 3)] == [True, False, False]
  [told] = conductor.notices()
  assert told.startswith("The disk holds ") and f"after the round deleted the target of each ended workspace: {conductor.tmp}-w3, {conductor.tmp}-w2.\n" in told
