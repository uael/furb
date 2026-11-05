"""The round of the conductor, on the real engine, on a World that runs each command in sh with a gh of the test."""

import asyncio
import json
import os
import re
import shutil
import subprocess
import sys
from dataclasses import dataclass, field
from functools import partial
from pathlib import Path

import pytest

import conftest
from conftest import Sand, life, paragraphs, said, settle, world_says
from furb.engine import OPERATOR, Exit, Refused, Text, site

HERE = Path(__file__).resolve().parent.parent
IMPORT = re.compile(r"^from furb\.engine import \(.*?\)\n", re.S | re.M)
WORD = "\n\n".join(IMPORT.sub("", (HERE / f).read_text(encoding="utf-8")) for f in ("../pr/pr.py", "conductor.py"))
POSIX = pytest.mark.skipif(sys.platform == "win32", reason="the round runs its commands in sh")
JQ = pytest.mark.skipif(not shutil.which("jq"), reason="the gh of the test applies --jq with jq")
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
ROSTER = [
  [OPERATOR, [], 200000],
  ["claude-cli:sonnet", ["medium", "high"], 1000000],
  ["claude-cli:opus", ["medium", "high", "max"], 1000000],
  ["codex:gpt-6-astra", ["high"], 258400],
]
RULED = [{"type": "required_status_checks", "parameters": {"required_status_checks": [{"context": "the gates"}]}}]
TEST = f'''limits = {{"room": {{"sonnet": 3, "opus": 2}}, "fixer": {SONNET!r}, "models": {{"editor": {SONNET!r}, "audit": {OPUS!r}}}}}
PROBE = "echo 64"
KEEP = 0


def busy(key, model={SONNET!r}):
  where = chain(f"w/{{key}}")
  name = prompt(tuple[int, str], "", to=model, on=where)
  working[name], models[name], homes[key] = key, model, where
  return name


def stepped(step, *args):
  def ear(id):
    yield "started", id
    try:
      got = yield from step(*args)
    except Exception as fault:
      got = fault
    yield "done", id, got

  return act("stepped", "", controlled(ear))
'''


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
        got = subprocess.run(["sh", "-c", command], input=stdin or "", capture_output=True, text=True, env=self.env, cwd=here, check=False)
        done = partial(world_says, "done", about, Exit(got.returncode, Text(f"{about}/stdout", got.stdout), Text(f"{about}/stderr", got.stderr)))
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


class Round:
  """A life whose root stands in the folder of a test and plays pr and the conductor, on a World whose gh answers
  each call from a table of patterns: the first that matches the call gives its output and its code."""

  def __init__(self, tmp: Path) -> None:
    folder = tmp / "bin"
    folder.mkdir()
    (folder / "gh").write_text(f"#!{sys.executable}\n{GH}", encoding="utf-8")
    (folder / "gh").chmod(0o755)
    kept = {n: v for n, v in os.environ.items() if not n.startswith(("GIT_", "COVERAGE_PROCESS_"))}
    env = {**kept, "PATH": f"{folder}{os.pathsep}{os.environ['PATH']}", "GH_LOG": str(tmp / "gh.log"), "GH_SAYS": str(tmp / "says.json")}
    self.tmp, self.says, self.engine = tmp, [], conftest.engine
    self.sand = Shell(files={}, env=env, stands=[ROSTER, str(tmp), OPUS])
    self.say("^$", "")
    self.says.clear()
    self.log, self.root = life(self.sand)

  async def load(self, *words: str) -> None:
    await self.word("\n\n".join([f"cd({str(self.tmp)!r})", WORD, *words]))

  @property
  def ns(self) -> dict:
    return self.engine.module(self.root)

  async def word(self, text: str) -> None:
    await self.engine.rung(text, on=self.root)
    await settle()

  async def value(self, expression: str) -> object:
    await self.word(f"got = {expression}")
    return self.ns["got"]

  async def step(self, name: str, *args: object) -> object:
    return await self.value(f"await stepped({', '.join([name, *map(repr, args)])})")

  async def busy(self, key: str, model: str = SONNET) -> str:
    return str(await self.value(f"busy({key!r}, {model!r})"))

  async def end(self, name: str, value: object) -> None:
    self.engine.close(value, name)
    await settle()

  def say(self, pattern: str, out: object, code: int = 0) -> None:
    self.says.insert(0, (pattern, out if isinstance(out, str) else json.dumps(out), code))
    (self.tmp / "says.json").write_text(json.dumps(self.says), encoding="utf-8")

  def github(self, *open_prs: dict, merged: tuple = (), main: str = "success", ruled: list | None = None) -> None:
    self.say(r"pr list --state open", list(open_prs))
    self.say(r"pr list --state merged", [{"number": n} for n in merged])
    self.say(r"run list --branch main", [{"headSha": "m1", "conclusion": main}])
    self.say(r"rules/branches/main", RULED if ruled is None else ruled)

  def gh(self) -> list[str]:
    path = self.tmp / "gh.log"
    return path.read_text(encoding="utf-8").splitlines() if path.exists() else []

  def notices(self) -> list[str]:
    return [str(x[5]) for x in said(self.log, "prompt") if x[3] == self.root and x[4] == "None"]

  def told(self) -> list[tuple[str, str]]:
    return [(str(x[5]), str(x[3])) for x in said(self.log, "prompt") if x[3] != self.root and x[4] == "None"]

  def hired(self) -> list[str]:
    made = [x for x in said(self.log, "prompt") if x[4] == "tuple[int, str]" and not self.engine.question(("rung", x[2]))]
    return [str(self.engine.get(x[3])[4]).split("/", 1)[1] for x in made]

  def words(self, chain: str) -> list[str]:
    return [str(x[4]) for x in said(self.log, "rung") if x[3] == chain and x[4]]

  def briefs(self) -> list[str]:
    return [str(x[5]) for x in said(self.log, "prompt") if x[4] == "tuple[int, str]" and not self.engine.question(("rung", x[2]))]


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


@pytest.fixture
async def conductor(tmp_path: Path) -> Round:
  made = Round(tmp_path)
  await made.load(TEST)
  return made


async def test_staff_hires_queue_then_backlog_in_order_and_keeps_one_slot_for_a_fixer(conductor: Round) -> None:
  await conductor.word('queue.extend([("q1", "b", "t"), ("q2", "b", "t")])\nbacklog.extend([("b1", "b", "t"), ("b2", "b", "t"), ("b3", "b", "t"), ("b4", "b", "t")])\nlimits["room"] = {"sonnet": 5}')
  await conductor.step("staff")
  assert conductor.hired() == ["q1", "q2", "b1", "b2"]
  assert conductor.ns["backlog"] == [("b3", "b", "t"), ("b4", "b", "t")]


async def until(ready, tries: int = 200) -> None:
  for _ in range(tries):
    if ready():
      return
    await asyncio.sleep(0.02)
  raise AssertionError("the round did not get there")


async def test_the_file_loads_over_a_module_that_holds_the_state_of_the_old_one(tmp_path: Path) -> None:
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
    "lead": {"round": "conduct9"},
    "noticed": {"k"},
    "heard": ["line"],
    "landed": [3],
    "owner_asks": {"A1": {"prompt": "prompt483", "quote": "q", "items": ["a"], "state": "open"}},
    "align_state": {"base": 40, "running": "rung5"},
    "comments": {"c1"},
  }
  made = Round(tmp_path)
  await made.word("\n".join(f"{name} = {value!r}" for name, value in old.items()))
  kept = {name: id(made.ns[name]) for name in old}
  await made.load()
  assert {name: id(made.ns[name]) for name in old} == kept
  assert await made.value("(cap(), model_of('a'))") == (6, OPUS)


async def test_an_old_module_that_bound_owner_asks_to_a_function_reads_the_asks_from_their_file(tmp_path: Path) -> None:
  made = Round(tmp_path)
  made.ns["owner_asks"] = list
  await made.load()
  assert made.ns["owner_asks"] == {}


async def test_cap_is_ten_at_most_and_room_counts_live_workers_of_a_family(conductor: Round) -> None:
  await conductor.word('limits["room"] = {"sonnet": 4, "opus": 8}')
  assert await conductor.value("cap()") == 10
  first = [await conductor.busy(f"k{n}") for n in range(4)]
  assert await conductor.value(f"(room({SONNET!r}), room({OPUS!r}))") == (False, True)
  for n in range(4, 10):
    await conductor.busy(f"k{n}", OPUS)
  assert await conductor.value(f"room({OPUS!r})") is False
  await conductor.end(first[0], (1, "r"))
  assert await conductor.value(f"room({OPUS!r})") is True


async def test_a_paused_worker_gives_its_slot_to_the_room_but_stays_a_living_worker(conductor: Round) -> None:
  await conductor.word('limits["room"] = {"sonnet": 2}')
  one, two = await conductor.busy("k1"), await conductor.busy("k2")
  assert await conductor.value(f"room({SONNET!r})") is False
  conductor.engine.pause(one)
  await settle()
  assert await conductor.value(f"(room({SONNET!r}), living(), running())") == (True, [one, two], [two])
  conductor.engine.wake(one)
  await settle()
  assert await conductor.value(f"room({SONNET!r})") is False


async def test_a_load_with_no_limits_binds_the_room_of_the_owner_and_caps_at_ten(conductor: Round) -> None:
  await conductor.word("del limits")
  await conductor.load()
  assert conductor.ns["limits"]["room"] == {"opus/high": 3, "opus/medium": 3, "sonnet/high": 3, "sonnet/medium": 3, "gpt-6-astra/high": 1}
  assert conductor.ns["limits"]["models"]["astra"] == ASTRA
  got = await conductor.value(f"(ALIGN_EVERY, cap(), family({ASTRA!r}), GATE, family('claude-cli:opus/high'))")
  assert got == (4, 10, "gpt-6-astra/high", "the gates", "opus/high")


async def test_an_item_waits_for_its_after_until_each_pr_of_it_merged_and_says_why_once(conductor: Round) -> None:
  await conductor.word('queue.append(("late", "b", "t"))\nafter["late"] = ["first"]')
  await conductor.step("staff")
  await conductor.step("staff")
  assert conductor.hired() == []
  assert conductor.ns["heard"] == ["late waits: first has not ended, and no item holds it."]
  await conductor.word('ends["first"] = [("prompt1", 12)]')
  await conductor.step("staff")
  assert conductor.ns["heard"][-1] == "late waits: #12 of first has not merged."
  await conductor.word("merged.add(12)")
  await conductor.step("staff")
  assert conductor.hired() == ["late"]


async def test_an_item_waits_while_a_live_worker_holds_a_file_that_changes_alone_and_a_test_file_holds_none(conductor: Round) -> None:
  await conductor.word('areas.update(one=["src/furb/engine.py", "test/conftest.py"], two=["src/furb", "test/conftest.py"], three=["test/conftest.py"])')
  await conductor.busy("one")
  await conductor.word('queue.extend([("two", "b", "t"), ("three", "b", "t")])')
  await conductor.step("staff")
  assert conductor.hired() == ["three"]
  assert conductor.ns["heard"] == ["two waits: the worker of one holds src/furb/engine.py."]


async def test_an_item_waits_while_an_open_pr_of_the_round_changes_a_file_that_changes_alone_but_not_for_its_own_pr(conductor: Round) -> None:
  await conductor.word(
    'areas.update(mine=["DOCTRINE.md"], other=["DOCTRINE.md"], shared=["tui/src"])\nprs[9] = {"n": 9, "labels": ["w:mine"]}\n'
    'touched[9] = {"sha": "s", "paths": ["DOCTRINE.md", "tui/src/app.ts"]}\nqueue.extend([("other", "b", "t"), ("mine", "b", "t"), ("shared", "b", "t")])'
  )
  await conductor.step("staff")
  assert conductor.hired() == ["mine", "shared"]
  assert conductor.ns["heard"] == ["other waits: #9 changes DOCTRINE.md."]


async def test_an_item_with_no_areas_holds_no_file_whatever_its_brief_names(conductor: Round) -> None:
  await conductor.word(
    'prs[9] = {"n": 9, "labels": ["w:x"]}\ntouched[9] = {"sha": "s", "paths": ["src/furb/engine.py"]}\n'
    'queue.append(("free", "Change src/furb/engine.py, as https://github.com/a/b/blob/main/src/furb/engine.py shows.", "t"))'
  )
  await conductor.step("staff")
  assert conductor.hired() == ["free"]


async def test_an_item_whose_model_no_worker_runs_waits_with_its_cause(conductor: Round) -> None:
  await conductor.word('efforts["big"] = "claude-cli:opus/max"\nqueue.append(("big", "b", "t"))')
  await conductor.step("staff")
  assert conductor.hired() == []
  assert conductor.ns["heard"] == [f"big waits: it asks claude-cli:opus/max, and a worker runs only {SONNET}, {OPUS}."]


async def test_hire_starts_a_worker_on_its_chain_with_the_asks_on_top_and_its_workspace(conductor: Round) -> None:
  await conductor.word('owner_asks["A3"] = {"prompt": "prompt30", "quote": "limit to 5 max", "items": ["cap"], "state": "open"}')
  name = await conductor.step("hire", "cap", "Hold the cap.", "round")
  [message] = conductor.briefs()
  where, path = conductor.engine.get(name)[3], f"{conductor.tmp}-w1"
  assert conductor.engine.get(where)[4] == "round/cap"
  assert message.startswith('This item carries the asks A3 of the owner, in his words:\n- A3: "limit to 5 max" (prompt30)\n')
  assert f"Work in the workspace {path}," in message
  assert 'labels=" --label w:cap --label thread:round"' in message
  assert message.endswith("The item, cap:\nHold the cap.")
  assert "{" + "workspace}" not in message and f"echo {where} >" in message
  assert f'on="{conductor.root}"' in message
  assert conductor.words(where) == [f"actor = {SONNET!r}", f"try:\n  cd({path!r})\nexcept Refused:\n  pass"]
  assert (conductor.ns["working"][name], conductor.ns["models"][name], conductor.ns["homes"]["cap"]) == ("cap", SONNET, where)
  assert conductor.ns["slots"][name] == "w1"
  assert not [x for x in conductor.gh() if x.startswith("label create")]


async def test_an_audit_runs_in_no_workspace_and_a_fix_takes_its_brief_whole(conductor: Round) -> None:
  await conductor.word('kinds["look"] = "audit"')
  look = await conductor.step("hire", "look", "Count the lines.")
  assert "scratch://audit/look.md" in conductor.briefs()[-1]
  assert conductor.words(conductor.engine.get(look)[3]) == [f"actor = {OPUS!r}"]
  await conductor.step("hire", "fix4", "Fix #4 in {workspace}", "t")
  assert conductor.briefs()[-1].startswith(f"Fix #4 in Work in the workspace {conductor.tmp}-w1,")


async def test_each_brief_forbids_a_window_on_this_machine(conductor: Round) -> None:
  for brief in ("WORKER", "AUDIT", "FIX", "ANSWER"):
    assert "Never open a window on this machine" in conductor.ns[brief]


async def test_each_brief_sends_each_build_through_the_slot_of_the_machine(conductor: Round) -> None:
  for brief in ("WORKER", "AUDIT", "FIX", "ANSWER"):
    assert "sh script/build-slot.sh <command>" in conductor.ns[brief]
    assert "no CARGO_BUILD_JOBS above 4" in conductor.ns[brief]
  assert "sh {repo}/script/build-slot.sh {repo}/script/worktree.sh {path}" in conductor.ns["WORKSPACE"]


async def test_the_round_reads_the_free_memory_and_a_hire_counts_its_share_of_it(conductor: Round) -> None:
  await conductor.word('PROBE = "echo 7.5"')
  await conductor.step("probe_memory")
  assert conductor.ns["machine"] == {"free": 7.5}
  await conductor.step("hire", "a", "x")
  assert conductor.ns["machine"]["free"] == 6.5
  await conductor.word('PROBE = "echo nonsense"')
  with pytest.raises(Refused, match="the probe of the memory said nonsense"):
    await conductor.step("probe_memory")


async def test_an_item_waits_while_the_machine_has_under_the_memory_that_a_hire_needs(conductor: Round) -> None:
  await conductor.word('limits["room"] = {"sonnet": 5}\nqueue.append(("q", "b", "t"))\nmachine["free"] = 5.9')
  await conductor.step("staff")
  assert conductor.hired() == []
  assert conductor.ns["heard"] == ["q waits: the machine has under 6 GB of memory free."]
  await conductor.word('machine["free"] = 6.0')
  await conductor.step("staff")
  assert conductor.hired() == ["q"]


@POSIX
@JQ
async def test_a_red_pr_waits_for_a_fixer_while_the_machine_has_too_little_memory(conductor: Round) -> None:
  conductor.github(raw(5, "k5", (("the gates", "FAILURE"),)))
  await conductor.word('machine["free"] = 1.0\nends["k5"] = []')
  await conductor.step("sweep")
  assert conductor.hired() == []
  assert "waits for a fixer" in conductor.ns["heard"][-1] and "GB of memory free" in conductor.ns["heard"][-1]


async def test_conducting_and_audit_hold_the_rules_of_rank_and_of_the_frozen_fixes(conductor: Round) -> None:
  rule = " ".join(conductor.ns["CONDUCTING"].split())
  assert "does not become its own item or PR" in rule
  assert "waits in scratch://round/frozen.md" in rule
  assert "land together in one change" in rule
  assert "in the order of the register after the items of queue" in rule
  assert "unless the coordinator asks" in rule
  audit = " ".join(conductor.ns["AUDIT"].split())
  assert "the hard work first by prompt266" in audit
  assert "reshapes a module, a contract or a protocol ranks before a row of polish or a simple row" in audit


async def test_a_worker_that_ends_with_a_pr_is_a_line_with_its_whole_report(conductor: Round) -> None:
  report = "I pushed one commit to main by mistake, then restored it."
  name = await conductor.busy("k")
  await conductor.end(name, (40, report))
  await conductor.word("tend()")
  assert conductor.ns["heard"] == [f"The worker k ({name}) ended with #40: {report}"]
  assert conductor.ns["ends"] == {"k": [(name, 40)]}
  assert conductor.ns["results"][name] == (40, report)
  assert conductor.ns["working"] == {}


async def test_a_worker_that_ends_with_no_pr_or_is_cancelled_is_a_decision_and_starts_no_rerun(conductor: Round) -> None:
  one, two = await conductor.busy("k"), await conductor.busy("j")
  await conductor.end(one, (0, "Nothing to change."))
  conductor.engine.cancel(two)
  await settle()
  await conductor.word("tend()")
  assert conductor.notices() == [
    f"The worker k ({one}) ended with no PR, which needs your decision:\n\nNothing to change.",
    f"The worker j ({two}) ended with no PR, which needs your decision:\n\nCancelledError()",
  ]
  assert conductor.hired() == []


async def test_an_auditor_queues_each_new_row_of_its_register_once(conductor: Round) -> None:
  conductor.sand.files["scratch://audit/align8.md"] = "\n".join([
    "| Dn | key | model | area | brief |",
    "| D1 | gap one | sonnet/high | src/furb/engine.py | Fix the gap. |",
    "| D2 | old | opus/max | tui | Old work. |",
    "| D3 | gap one | sonnet/high | tui | Twice. |",
  ])
  await conductor.word('kinds["align8"] = "audit"\nends["align8-old"] = [("prompt1", 3)]')
  name = await conductor.busy("align8", OPUS)
  await conductor.end(name, (0, "Two rows."))
  await conductor.word("tend()")
  key = "align8-gap-one"
  [(made, brief, thread)] = conductor.ns["queue"]
  assert (made, thread) == (key, key)
  assert brief.endswith("Fix the gap.\n\nThis item is row D1 of the register scratch://audit/align8.md.")
  assert (conductor.ns["efforts"][key], conductor.ns["areas"][key], conductor.ns["kinds"][key]) == (SONNET, ["src/furb/engine.py"], "edit")
  assert conductor.ns["heard"] == [
    f"The auditor align8 ({name}) ended: Two rows.",
    "The row D2 of the register of align8 is align8-old, which the round holds already, so it queued nothing for it.",
    f"The register of align8 queued {key}.",
  ]


async def test_an_ask_that_a_worker_leaves_queues_one_follow_up_unless_another_item_carries_it(conductor: Round) -> None:
  await conductor.word(
    'owner_asks.update(A1={"prompt": "prompt1", "quote": "q1", "items": ["k"], "state": "open"}, '
    'A2={"prompt": "prompt2", "quote": "q2", "items": ["k", "later"], "state": "open"})\nqueue.append(("later", "b", "t"))'
  )
  first = await conductor.busy("k")
  await conductor.end(first, (5, "Served: A3\nLeft: A1 (no room), A2 (the item later)"))
  await conductor.word("tend()")
  asks = conductor.ns["owner_asks"]
  assert [q[0] for q in conductor.ns["queue"]] == ["later", "ask-a1-1"]
  assert (asks["A1"]["items"], asks["A1"]["state"], asks["A2"]["state"]) == (["k", "ask-a1-1"], "partly", "open")
  assert json.loads(conductor.sand.files["scratch://owner/asks.json"])[0]["id"] == "A1"
  await conductor.word("queue.remove(queue[1])")
  second = await conductor.busy("ask-a1-1")
  await conductor.end(second, (0, "Left: A1 (still no room)"))
  await conductor.word("tend()")
  assert conductor.ns["owner_asks"]["A1"]["state"] == "decision"
  assert conductor.notices()[-1].startswith("The follow-up ask-a1-1 of the ask A1 of the owner left it again")
  assert [q[0] for q in conductor.ns["queue"]] == ["later"]


async def test_a_message_of_the_owner_that_no_ask_names_is_one_line_of_a_notice(conductor: Round) -> None:
  first = conductor.engine.prompt(str, "a", on=conductor.root)
  with site.set("operator/uael"):
    second = conductor.engine.prompt(str, "b", on=conductor.root)
  third = conductor.engine.prompt(str, "c", on=conductor.root)
  await conductor.word(f'owner_asks["A1"] = {{"prompt": "{first}, {third}", "items": [], "state": "done"}}\nprompt(str, "d")')
  await conductor.word("asked()")
  await conductor.word("asked()")
  [notice] = conductor.notices()
  assert f"\n- The message {second} of the owner holds no ask in owner_asks.\n" in notice
  assert notice.count("\n- ") == 1


async def test_an_ask_of_the_owner_that_no_live_item_carries_is_one_line_of_a_notice(conductor: Round) -> None:
  await conductor.word(
    'owner_asks.update(A1={"prompt": "prompt1", "items": [], "state": "missing"}, A2={"prompt": "prompt2", "items": ["gone"], "state": "partly"}, '
    'A3={"prompt": "prompt3", "items": ["q"], "state": "open"}, A4={"prompt": "prompt4", "items": [], "state": "done"}, '
    'A5={"prompt": "prompt5", "items": ["k"], "state": "decision"})\nqueue.append(("q", "b", "t"))'
  )
  await conductor.word("asked()")
  await conductor.word("asked()")
  [notice] = conductor.notices()
  assert (
    "\n- The ask A1 of the owner (prompt1) is missing, and no item names it."
    "\n- The ask A2 of the owner (prompt2) is partly, and no item carries it now: its items gone ended.\n"
  ) in notice
  assert notice.count("\n- ") == 2
  await conductor.word('owner_asks["A2"]["items"].append("again")\nasked()')
  assert len(conductor.notices()) == 1
  await conductor.end(conductor.ns["lead"]["asked"], None)
  await conductor.word("asked()")
  assert len(conductor.notices()) == 2
  assert "its items gone, again ended" in conductor.notices()[1]
  assert conductor.notices()[1].count("\n- ") == 1


async def test_a_round_with_five_uncarried_asks_gives_one_notice_and_the_next_none_while_it_stands_open(conductor: Round) -> None:
  await conductor.word('owner_asks.update({f"A{i}": {"prompt": f"prompt{i}", "items": [], "state": "open"} for i in range(1, 6)})\nasked()')
  assert len(conductor.notices()) == 1
  assert all(f"The ask A{i} of the owner" in conductor.notices()[0] for i in range(1, 6))
  await conductor.word('owner_asks["A6"] = {"prompt": "prompt6", "items": [], "state": "open"}\nasked()')
  assert len(conductor.notices()) == 1
  await conductor.end(conductor.ns["lead"]["asked"], None)
  await conductor.word("asked()")
  assert len(conductor.notices()) == 2
  assert "The ask A6 of the owner" in conductor.notices()[1]
  assert "The ask A1 of the owner" not in conductor.notices()[1]


async def test_the_words_of_the_owner_hold_his_messages_and_his_answers_in_order(conductor: Round) -> None:
  told = conductor.engine.prompt(str, "Do it.", on=conductor.root)
  await conductor.word(f'one = prompt(str, "Which one?", to="operator")\ntwo = prompt(str, "Which?", to={OPUS!r})')
  await conductor.end(conductor.ns["one"], "a")
  await conductor.end(conductor.ns["two"], "b")
  await conductor.word("owner_words()")
  assert conductor.sand.files["scratch://owner/words.md"] == (
    f"# The words of the owner in this session, verbatim, in order\n\n## {told}, his message\n\nDo it.\n\n"
    f"## {conductor.ns['one']}, his answer to my question\n\na\n\n"
  )


async def test_an_audit_of_alignment_starts_at_four_merges_on_astra_and_the_model_of_audit_in_turn(conductor: Round) -> None:
  await conductor.word(f'limits["models"]["astra"] = {ASTRA!r}\nalign()')
  assert conductor.ns["align_state"] == {"base": "0"}
  await conductor.word("merged.update(range(3))\nalign()")
  assert conductor.ns["queue"] == []
  await conductor.word('merged.add(3)\nends.update({f"align{n}": [("p", 0)] for n in range(1, 8)})\nlead["main"] = "abc"\nalign()')
  key, brief, thread = conductor.ns["queue"][0]
  assert (key, thread, conductor.ns["kinds"][key], await conductor.value(f"model_of({key!r})")) == ("align8", "align", "audit", OPUS)
  assert "scratch://owner/words.md, scratch://owner/picks.md" in brief and "scratch://owner/asks.json" in brief
  assert "git log --oneline -40 origin/main" in brief and "align7" in brief
  assert conductor.ns["align_state"] == {"base": "4", "n": "8", "sha": "abc"}
  await conductor.word("merged.update(range(4, 20))\nalign()")
  assert len(conductor.ns["queue"]) == 1
  await conductor.word('queue.clear()\nends["align8"] = [("p", 0)]\nalign()')
  assert [q[0] for q in conductor.ns["queue"]] == ["align9"]
  assert await conductor.value("model_of('align9')") == ASTRA


@POSIX
@JQ
async def test_the_round_turns_on_auto_merge_for_its_prs_behind_the_ruleset_and_off_for_a_held_one(conductor: Round) -> None:
  await conductor.word("ends.update(a=[], b=[], c=[], d=[])")
  boxed = raw(4, "d", body="- [ ] Its prose obeys AGENTS.md")
  conductor.github(raw(1, "a"), raw(2, "b", labels=("hold",), autoMergeRequest={}), raw(3, "c", isDraft=True), boxed, raw(5, "other"))
  await conductor.step("sweep")
  assert [x for x in conductor.gh() if x.startswith("pr merge")] == ["pr merge 1 --auto --squash", "pr merge 2 --disable-auto"]
  assert conductor.ns["touched"] == {n: {"sha": f"sha{n}", "paths": []} for n in (1, 2, 3, 4)}


@POSIX
@JQ
async def test_with_no_ruleset_on_main_the_round_turns_on_no_auto_merge_and_tells_once(conductor: Round) -> None:
  await conductor.word('ends["a"] = []')
  conductor.github(raw(1, "a"), ruled=[])
  await conductor.step("sweep")
  await conductor.step("sweep")
  assert not [x for x in conductor.gh() if x.startswith("pr merge")]
  assert conductor.notices() == ["main has no ruleset that requires the check the gates, so the round turns on no auto-merge and merges no PR."]


@POSIX
@JQ
async def test_a_pr_of_the_round_that_merged_is_named_in_the_next_notice(conductor: Round) -> None:
  await conductor.word("ends.update(a=[], b=[])")
  conductor.github(raw(1, "a"), raw(2, "b"))
  await conductor.step("sweep")
  conductor.github(raw(2, "b"), merged=(1, 99))
  await conductor.step("sweep")
  assert (conductor.ns["landed"], conductor.ns["merged"]) == ([1], {1, 99})
  assert 1 not in conductor.ns["touched"]


@POSIX
@JQ
async def test_a_pr_of_a_live_maker_that_turns_conflicting_tells_the_maker_once_for_each_head_and_sends_no_fixer(conductor: Round) -> None:
  maker = await conductor.busy("a")
  where = conductor.engine.get(maker)[3]
  conductor.github(raw(1, "a", labels=("thread:t",)))
  await conductor.step("sweep")
  assert conductor.told() == []
  conductor.github(raw(1, "a", mergeable="CONFLICTING", checks=(), labels=("thread:t",)))
  await conductor.step("sweep")
  await conductor.step("sweep")
  assert [on for _, on in conductor.told()] == [where]
  assert conductor.told()[0][0].startswith("Your open PR #1 conflicts with its base at sha1. Fix it before any other step:")
  assert conductor.hired() == []
  conductor.sand.now += conductor.ns["UNMOVED"] - 1
  await conductor.step("sweep")
  assert conductor.notices() == []
  conductor.sand.now += 1
  await conductor.step("sweep")
  await conductor.step("sweep")
  [stall] = conductor.notices()
  assert stall.startswith(f"#1 conflicts with its base at sha1, and {maker}, whom the round told of it, has not moved its head in 30 minutes. Its last act: ")
  assert conductor.hired() == []
  conductor.github(raw(1, "a", mergeable="CONFLICTING", checks=(), headRefOid="sha9", labels=("thread:t",)))
  await conductor.step("sweep")
  await conductor.step("sweep")
  assert [m[:36] for m, on in conductor.told() if on == where] == ["Your open PR #1 conflicts with its b"] * 2
  assert "at sha9." in conductor.told()[-1][0]
  assert conductor.hired() == []


@POSIX
@JQ
async def test_a_red_pr_whose_told_maker_ends_goes_to_a_fixer_at_the_next_sweep(conductor: Round) -> None:
  maker = await conductor.busy("a")
  conductor.github(raw(1, "a", checks=(("the gates", "FAILURE"),), labels=("thread:t",)))
  await conductor.step("sweep")
  assert conductor.told()[0][0].startswith("Your open PR #1 has a failed check at sha1.")
  await conductor.end(maker, (1, "pushed"))
  await conductor.step("sweep")
  assert conductor.hired() == ["fix1"]
  assert conductor.engine.get(conductor.engine.get(conductor.ns["assigned"][1]["who"])[3])[4] == "t/fix1"
  assert conductor.ns["assigned"][1]["fixer"] and conductor.ns["assigned"][1]["tries"] == 1


@POSIX
@JQ
async def test_a_red_pr_with_no_live_maker_gets_at_most_two_fixers_for_a_head_then_a_notice(conductor: Round) -> None:
  await conductor.word('ends["a"] = [("prompt1", 1)]')
  conductor.github(raw(1, "a", mergeable="CONFLICTING", checks=()))
  for _ in range(4):
    await conductor.step("sweep")
    for name in await conductor.value("living()"):
      await conductor.end(name, (1, "no move"))
  assert conductor.hired() == ["fix1", "fix1"]
  assert "it conflicts with its base at sha1" in conductor.briefs()[0]
  assert conductor.notices() == ["#1 conflicts with its base at sha1, and 2 fixers ended at this head and did not move it, so the round sends no third one."]


@POSIX
@JQ
async def test_a_red_main_sends_no_fixer_to_a_red_pr_and_tells_once(conductor: Round) -> None:
  await conductor.word('ends["a"] = []')
  conductor.github(raw(1, "a", checks=(("the gates", "FAILURE"),)), main="failure")
  await conductor.step("sweep")
  await conductor.step("sweep")
  assert conductor.hired() == []
  [told] = conductor.notices()
  assert told.startswith("main is red at m1: find its cause first.")
  assert conductor.ns["heard"] == ["#1 is red at sha1 while main is red, so it waits for main."]


def pushed(conductor: Round, *branches: str) -> dict[str, str]:
  def git(*args: str) -> str:
    cmd = ["git", "-c", "user.name=t", "-c", "user.email=t@t", *args]
    return subprocess.run(cmd, cwd=conductor.tmp, env=conductor.sand.env, capture_output=True, text=True, check=True).stdout.strip()  # noqa: S603

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
  await conductor.word("ends.update(a=[], b=[])")
  heads = pushed(conductor, "w/a", "w/b")
  conductor.github(raw(1, "a"), raw(2, "b", headRefOid=heads["w/b"]))
  calls = []
  for _ in range(4):
    await conductor.step("sweep")
    calls.append([x for x in conductor.gh() if x.startswith(("pr close", "pr reopen"))])
  assert calls[0] == []
  assert calls[1] == calls[2] == calls[3] == ["pr close 1", "pr reopen 1"]
  assert conductor.ns["heard"] == [f"#1 kept the head sha1 for two sweeps while its branch w/a holds {heads['w/a'][:7]} on origin, so the round closed and reopened it, once for this head."]
  conductor.github(raw(1, "a", headRefOid=heads["w/a"]), raw(2, "b", headRefOid=heads["w/b"]))
  await conductor.step("sweep")
  assert conductor.ns["lagged"] == {}


@POSIX
@JQ
async def test_a_red_pr_whose_failed_checks_main_fails_too_gets_no_fixer_and_one_line(conductor: Round) -> None:
  await conductor.word("ends.update(a=[], b=[])")
  conductor.github(raw(1, "a", checks=(("the gates", "FAILURE"), ("bench", "FAILURE"))), raw(2, "b", checks=(("the gates", "FAILURE"), ("ruff", "FAILURE"))))
  conductor.say(r"commits/main/check-runs", {"check_runs": [{"name": "the gates", "conclusion": "failure"}, {"name": "bench", "conclusion": "failure"}, {"name": "ruff", "conclusion": "success"}]})
  await conductor.step("sweep")
  await conductor.step("sweep")
  assert conductor.hired() == ["fix2"]
  assert conductor.ns["heard"] == ["#1 is red at sha1 only on bench, the gates, which main fails too, so the round hires no fixer for it."]


@POSIX
@JQ
async def test_a_pr_whose_checks_still_run_is_no_red_pr(conductor: Round) -> None:
  await conductor.word('ends["a"] = []')
  conductor.github(raw(1, "a", checks=(("the gates", ""), ("ruff", "FAILURE"))))
  await conductor.step("sweep")
  assert conductor.ns["prs"][1]["state"] == "running"
  assert conductor.hired() == []


@POSIX
@JQ
async def test_a_sweep_that_gh_fails_raises_refused(conductor: Round) -> None:
  conductor.say(r"pr list --state open", "", 1)
  with pytest.raises(Refused, match="gh listed no PRs"):
    await conductor.step("sweep")


@POSIX
@JQ
async def test_a_comment_of_the_owner_goes_to_the_worker_on_the_pr_and_holds_it(conductor: Round) -> None:
  maker = await conductor.busy("a")
  await conductor.word('ends["b"] = []')
  conductor.github(raw(1, "a"), raw(2, "b"), raw(3, "foreign"))
  await conductor.step("sweep")
  await conductor.word('comments.add("seen")')
  conductor.say(r"pulls/1/comments", [{"pull_request_url": "x/pulls/1", "id": 10, "user": {"login": "uael"}, "path": "a.py", "line": 3, "body": "Name it well."}])
  conductor.say(r"issues/2/comments", [{"issue_url": "x/issues/2", "id": 11, "user": {"login": "uael"}, "body": "And this."}, {"issue_url": "x/issues/2", "id": 12, "user": {"login": "uael"}, "body": "🤖 mine"}])
  await conductor.step("review")
  assert conductor.told()[0] == (
    "A new comment of the owner on your PR #1. Do what it asks, answer it on GitHub with a reply that starts with 🤖, push the fix, and take the label hold off.\n#1 c10 a.py:3: Name it well.",
    conductor.engine.get(maker)[3],
  )
  key, brief, _ = conductor.ns["queue"][0]
  assert key == "fix2" and "#2 i11 -: And this." in brief and "w/b" in brief
  assert not [x for x in conductor.gh() if "pulls/3/" in x]
  assert {"pr edit 1 --add-label hold", "pr edit 2 --add-label hold"} <= set(conductor.gh())
  assert conductor.ns["comments"] == {"seen", "c10", "i11", "i12"}


@POSIX
@JQ
async def test_the_first_read_of_the_comments_sends_nothing(conductor: Round) -> None:
  await conductor.busy("a")
  conductor.github(raw(1, "a"))
  await conductor.step("sweep")
  conductor.say(r"pulls/1/comments", [{"pull_request_url": "x/pulls/1", "id": 10, "user": {"login": "uael"}, "path": "a", "line": 1, "body": "old"}])
  await conductor.step("review")
  assert conductor.told() == []
  assert conductor.ns["comments"] == {"c10"}


@POSIX
@JQ
async def test_a_pr_that_refuses_auto_merge_merges_plainly_or_as_a_layer_of_its_stack(conductor: Round) -> None:
  await conductor.word('ends["a"] = []')
  conductor.github(raw(1, "a"))
  conductor.say(r"pr merge 1 --auto", "", 1)
  conductor.say(r"pr merge 1 --squash$", "", 1)
  await conductor.step("sweep")
  assert [x for x in conductor.gh() if x.startswith(("pr merge", "stack merge"))] == ["pr merge 1 --auto --squash", "pr merge 1 --squash", "stack merge 1 --yes --squash"]


@POSIX
@JQ
async def test_a_green_pr_that_has_not_merged_after_stale_seconds_is_one_line(conductor: Round) -> None:
  await conductor.word('ends["a"] = []')
  conductor.github(raw(1, "a"))
  await conductor.step("sweep")
  conductor.sand.now += conductor.ns["STALE"] + 1
  await conductor.step("sweep")
  await conductor.step("sweep")
  assert conductor.ns["heard"] == ["#1 is green at sha1 for 45 minutes and has not merged."]


@POSIX
@JQ
async def test_a_fixer_of_a_full_family_runs_on_the_next_model_of_upgrade(conductor: Round) -> None:
  await conductor.word('ends["a"] = [("prompt1", 1)]')
  for n in range(3):
    await conductor.busy(f"k{n}")
  conductor.github(raw(1, "a", mergeable="CONFLICTING", checks=()))
  await conductor.step("sweep")
  assert conductor.hired() == ["fix1"]
  assert conductor.ns["models"][conductor.ns["assigned"][1]["who"]] == OPUS


async def test_an_item_of_a_full_family_moves_to_the_next_model_of_upgrade(conductor: Round) -> None:
  await conductor.busy("one")
  await conductor.word('limits["room"] = {"sonnet": 1, "opus": 2}\nqueue.append(("two", "b", "t"))')
  await conductor.step("staff")
  assert conductor.hired() == ["two"]
  assert conductor.ns["efforts"]["two"] == OPUS


async def test_a_wait_on_an_item_that_has_not_ended_is_no_line(conductor: Round) -> None:
  await conductor.word('efforts["dep"] = "claude-cli:opus/max"\nqueue.extend([("dep", "b", "t"), ("late", "b", "t")])\nafter["late"] = ["dep"]')
  await conductor.step("staff")
  assert conductor.hired() == []
  assert conductor.ns["heard"] == [f"dep waits: it asks claude-cli:opus/max, and a worker runs only {SONNET}, {OPUS}."]


async def test_a_register_with_a_column_after_sets_the_waits_of_each_row(conductor: Round) -> None:
  conductor.sand.files["scratch://audit/look.md"] = "\n".join([
    "| Dn | key | model | area | after | brief |",
    "| D1 | old | sonnet/high | src | | Done. |",
    "| D2 | base | sonnet/high | src | | Build it. |",
    "| D3 | top | sonnet/high | tui | D1, D2 | Use it. |",
  ])
  await conductor.word('ends["triage-old"] = []\nregister("look")')
  assert [q[0] for q in conductor.ns["queue"]] == ["triage-base", "triage-top"]
  assert conductor.ns["queue"][1][1].endswith("Use it.\n\nThis item is row D3 of the register scratch://audit/look.md.")
  assert conductor.ns["after"] == {"triage-top": ["triage-old", "triage-base"]}


def git(where: Path, *words: str) -> str:
  return subprocess.run(["git", "-C", str(where), *words], capture_output=True, text=True, check=True).stdout.strip()


@POSIX
async def test_settle_keeps_the_work_of_another_chain_on_a_new_branch_and_never_on_the_branch_of_a_pr(conductor: Round, tmp_path: Path) -> None:
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
  pushed_head = git(space, "rev-parse", "HEAD")
  (space / ".git" / "furb-holder").write_text("chain-old\n", encoding="utf-8")
  (space / "b.txt").write_text("foreign\n", encoding="utf-8")
  subprocess.run(["sh", "-c", conductor.ns["SETTLE"], "sh", str(space), "chain-new"], check=True, capture_output=True)
  assert git(origin, "rev-parse", "w/pr") == pushed_head
  wip = [x.split()[-1] for x in git(origin, "branch", "--list", "w/wip-*").splitlines()]
  assert len(wip) == 1 and wip[0].startswith(f"w/wip-{pushed_head[:7]}-")
  assert git(origin, "show", f"{wip[0]}:b.txt") == "foreign"
  assert (space / ".git" / "furb-holder").read_text(encoding="utf-8") == "chain-new\n"


@POSIX
async def test_hire_writes_the_chain_of_the_worker_in_its_workspace_before_its_prompt(conductor: Round) -> None:
  space = Path(f"{conductor.tmp}-w1")
  subprocess.run(["git", "init", "-q", str(space)], check=True)
  (space / ".git" / "furb-holder").write_text("chain-old\n", encoding="utf-8")
  name = await conductor.step("hire", "cap", "x")
  assert (space / ".git" / "furb-holder").read_text(encoding="utf-8") == f"{conductor.engine.get(name)[3]}\n"
  order = [x[1] for x in conductor.log if conductor.engine.question(x) and (x[1] == name or (x[0] == "bash" and "furb-holder" in str(x[4])))]
  assert order[-1] == name and len(order) > 1


async def test_a_worker_whose_chain_says_no_fact_for_quiet_seconds_is_no_live_worker_and_one_notice(conductor: Round) -> None:
  name = await conductor.busy("a")
  where = conductor.engine.get(name)[3]
  await conductor.word("tend()")
  assert await conductor.value("running()") == [name]
  conductor.sand.now += conductor.ns["QUIET"]
  await conductor.word("tend()")
  await conductor.word("tend()")
  assert await conductor.value("(running(), living())") == ([], [name])
  [quiet] = conductor.notices()
  assert quiet.startswith(f"The worker a ({name}) said no fact on its chain for 30 minutes, so the round counts it as no live worker. Its open acts: ")
  assert name in quiet
  await conductor.word(f"rung('x = 1', on={where!r})")
  await conductor.word("tend()")
  assert await conductor.value("running()") == [name]
  await conductor.end(name, (1, "r"))
  await conductor.word("tend()")
  assert conductor.ns["progress"] == {}


@POSIX
async def test_a_hire_takes_no_workspace_whose_holder_chain_has_a_live_worker(conductor: Round) -> None:
  worker = await conductor.busy("tui")
  held = conductor.engine.get(worker)[3]
  space = Path(f"{conductor.tmp}-w1")
  subprocess.run(["git", "init", "-q", str(space)], check=True)
  (space / ".git" / "furb-holder").write_text(f"{held}\n", encoding="utf-8")
  name = await conductor.step("hire", "names", "x")
  assert conductor.ns["slots"][name] == "w2"
  assert (space / ".git" / "furb-holder").read_text(encoding="utf-8") == f"{held}\n"


async def test_an_item_of_a_full_opus_medium_waits_for_its_family_and_never_moves_up_to_opus_high(conductor: Round) -> None:
  high = "claude-cli:opus/high"
  await conductor.word(f'limits["room"] = {{"opus/medium": 1, "opus/high": 3}}\nlimits["models"].update(audit={high!r}, lead={OPUS!r})\nefforts.update(two={OPUS!r}, core={high!r})')
  await conductor.busy("one", OPUS)
  await conductor.word('queue.extend([("two", "b", "t"), ("core", "b", "t")])')
  await conductor.step("staff")
  assert conductor.hired() == ["core"]
  assert conductor.ns["efforts"]["two"] == OPUS
  assert conductor.ns["heard"] == ["two waits: opus/medium has no room."]


@POSIX
async def test_the_disk_frees_the_target_of_the_workspace_that_ended_first_and_tells_once_when_it_stays_short(conductor: Round) -> None:
  for slot in ("w1", "w2", "w3"):
    (Path(f"{conductor.tmp}-{slot}") / "target").mkdir(parents=True)
  names = [await conductor.busy(f"k{n}") for n in (1, 2, 3)]
  await conductor.word("\n".join(f"slots[{name!r}] = 'w{n}'" for n, name in enumerate(names, 1)))
  await conductor.end(names[2], (1, "r"))
  await conductor.word("tend()")
  conductor.sand.now += 100
  await conductor.end(names[1], (2, "r"))
  await conductor.word("tend()")
  await conductor.step("keep_disk")
  assert all((Path(f"{conductor.tmp}-w{n}") / "target").exists() for n in (1, 2, 3))
  await conductor.word("KEEP = 10**9")
  await conductor.step("keep_disk")
  assert (Path(f"{conductor.tmp}-w3") / "target").exists()
  for _ in range(2):
    conductor.sand.now += conductor.ns["DISK_EVERY"]
    await conductor.step("keep_disk")
  assert [(Path(f"{conductor.tmp}-w{n}") / "target").exists() for n in (1, 2, 3)] == [True, False, False]
  [told] = conductor.notices()
  assert told.startswith("The disk holds ") and f"after the round deleted the target of each ended workspace: {conductor.tmp}-w3, {conductor.tmp}-w2.\n" in told


def conducted(made: Round) -> str:
  return next(x[1] for x in said(made.log, "conduct"))


def workers(made: Round) -> list[str]:
  return [str(x[1]) for x in said(made.log, "prompt") if x[4] == "tuple[int, str]"]


@POSIX
@JQ
@pytest.mark.parametrize("engine_of", ["python", "monty"], indirect=True)
async def test_a_round_tells_the_coordinator_only_its_notices_and_a_close_of_it_keeps_its_worker(tmp_path: Path, engine_of: str) -> None:
  made = Round(tmp_path)
  await made.load(TEST, 'queue.append(("one", "b", "t"))\nround = conduct(60)')
  made.github()
  await until(lambda: workers(made))
  outside = {x[1] for x in made.log if made.engine.question(x) and made.engine.scope(x[1]) != made.root}
  round_ = conducted(made)
  told = [p.split("\n", 1)[0].split(" ")[0][1:] for p in paragraphs(made.engine.turns(on=made.root))]
  assert outside and not (set(told) & outside)
  made.engine.close(None, round_)
  await settle()
  assert made.engine.over(round_) and not made.engine.over(workers(made)[0])


@POSIX
@JQ
@pytest.mark.parametrize("engine_of", ["python", "monty"], indirect=True)
async def test_a_step_that_raises_is_one_notice_and_the_round_waits_on_itself_and_each_worker_until_it_holds_nothing(tmp_path: Path, engine_of: str) -> None:
  made = Round(tmp_path)
  made.say(r"pr list --state open", "", 1)
  await made.load(TEST, 'queue.append(("one", "b", "t"))\nround = conduct(60)')
  await until(lambda: workers(made) and said(made.log, "wait"))
  round_, [worker] = conducted(made), workers(made)
  [fault] = made.notices()
  assert fault.startswith(f"The round {round_} met a fault in its step sweep: Refused: gh listed no PRs")
  assert [x[4:] for x in said(made.log, "wait")] == [(60, round_, worker)]
  made.github()
  await made.end(worker, (3, "done"))
  await until(lambda: made.engine.over(round_))
  assert made.engine.peek(round_) == f"The round holds no item and no open PR that it tends.\n\nSince the last notice:\n- The worker one ({worker}) ended with #3: done"
  assert len(made.notices()) == 1


@POSIX
@JQ
@pytest.mark.parametrize("engine_of", ["python", "monty"], indirect=True)
async def test_a_later_conduct_replaces_the_round_at_its_next_step(tmp_path: Path, engine_of: str) -> None:
  made = Round(tmp_path)
  made.github()
  await made.load(TEST, 'efforts["far"] = "claude-cli:opus/max"\nqueue.append(("far", "b", "t"))\nfirst = conduct(0.2)')
  await until(lambda: said(made.log, "wait"))
  await made.word("second = conduct(60)")
  first, second = (x[1] for x in said(made.log, "conduct"))
  await until(lambda: made.engine.over(first))
  assert made.engine.peek(first) is None and not made.engine.over(second)
  made.engine.close(None, second)
  await settle()


@POSIX
@JQ
@pytest.mark.parametrize("engine_of", ["python", "monty"], indirect=True)
async def test_a_later_conduct_starts_once_the_round_that_it_replaces_ends_so_one_fixer_takes_a_red_pr(tmp_path: Path, engine_of: str) -> None:
  made = Round(tmp_path)
  await made.load(TEST, 'ends["a"] = []')
  made.github(raw(7, "a", checks=(("the gates", "FAILURE"),)))
  made.sand.hold = r"gh pr list --state open"
  await made.word("first = conduct(60)")
  await until(lambda: made.sand.held)
  await made.word("second = conduct(60)")
  first, second = (x[1] for x in said(made.log, "conduct"))
  made.sand.release()
  await until(lambda: made.engine.over(first) and [x for x in said(made.log, "wait") if second in x[5:]])
  assert made.hired() == ["fix7"]
  assert made.engine.peek(first) is None
  made.engine.close(None, second)
  await settle()
