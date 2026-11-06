"""The round of the conductor, on the real engine, on a World that runs each command in sh with a gh of the test."""

import asyncio
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

from conftest import paragraphs, said, settle
from extensions.conftest import Bench
from furb.engine import OPERATOR, Refused, site

HERE = Path(__file__).resolve().parent.parent
IMPORT = re.compile(r"^from furb\.engine import \(.*?\)\n", re.S | re.M)
POSIX = pytest.mark.skipif(sys.platform == "win32", reason="the round runs its commands in sh")
JQ = pytest.mark.skipif(not shutil.which("jq"), reason="the gh of the test applies --jq with jq")
SONNET, OPUS, ASTRA = "claude-cli:sonnet/high", "claude-cli:opus/medium", "codex:gpt-6-astra/high"
ROSTER = [
  [OPERATOR, [], 200000],
  ["claude-cli:sonnet", ["medium", "high"], 1000000],
  ["claude-cli:opus", ["medium", "high", "max"], 1000000],
  ["codex:gpt-6-astra", ["high"], 258400],
]
RULED = [{"type": "required_status_checks", "parameters": {"required_status_checks": [{"context": "the gates"}]}}]
TEST = f'''limits = {{"room": {{"sonnet": 3, "opus": 2}}, "fixer": {SONNET!r}, "models": {{"editor": {SONNET!r}, "audit": {OPUS!r}}}}}
KEEP = 0


def busy(key, model={SONNET!r}):
  where = chain(f"w/{{key}}")
  name = prompt(tuple[int, str], "", to=model, on=where)
  working[name] = {{"key": key, "model": model}}
  return name


def held(*keys, pr=0):
  for key in keys:
    name = busy(key)
    close((pr, "ended"), name)
    working[name]["pr"] = pr


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


class Round(Bench):
  """The Bench of pr and the conductor, with the roster of the test."""

  def __init__(self, tmp: Path) -> None:
    super().__init__(tmp, [HERE / "../pr/pr.py", HERE / "conductor.py"], ROSTER, OPUS)

  async def step(self, name: str, *args: object) -> object:
    return await self.value(f"await stepped({', '.join([name, *map(repr, args)])})")

  async def busy(self, key: str, model: str = SONNET) -> str:
    return str(await self.value(f"busy({key!r}, {model!r})"))

  async def end(self, name: str, value: object) -> None:
    self.engine.close(value, name)
    await settle()

  def github(self, *open_prs: dict, merged: tuple = (), main: str = "success", ruled: list | None = None) -> None:
    self.say(r"pr list --state open", list(open_prs))
    self.say(r"pr list --state merged", [{"number": n, "headRefName": f"w/merged{n}"} for n in merged])
    self.say(r"run list --branch main", [{"headSha": "m1", "conclusion": main}])
    self.say(r"rules/branches/main", RULED if ruled is None else ruled)

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


async def test_staff_hires_queue_in_order_and_keeps_one_slot_for_a_fixer(conductor: Round) -> None:
  await conductor.word('queue.extend([{"key": f"q{n}", "brief": "b", "thread": "t"} for n in range(1, 7)])\nlimits["room"] = {"sonnet": 5}')
  await conductor.step("staff")
  assert conductor.hired() == ["q1", "q2", "q3", "q4"]
  assert [q["key"] for q in conductor.ns["queue"]] == ["q5", "q6"]


async def until(ready, tries: int = 200) -> None:
  for _ in range(tries):
    if ready():
      return
    await asyncio.sleep(0.02)
  raise AssertionError("the round did not get there")


async def test_the_file_loads_over_a_module_that_holds_the_state_of_the_old_one(tmp_path: Path) -> None:
  old = {
    "queue": [{"key": "a", "brief": "brief a", "thread": "t", "areas": ["src/furb"], "model": OPUS}],
    "working": {"prompt9": {"key": "c", "model": SONNET, "slot": "w2"}},
    "prs": {7: {"n": 7}},
    "merges": {1: "w/a", 2: "w/b"},
    "assigned": {7: {"sha": "x", "who": "prompt9", "at": 1.0, "fixer": False, "maker": "prompt9", "tries": 0}},
    "limits": {"room": {"opus": 1, "sonnet": 3, "opus/high": 2}, "fixer": SONNET, "models": {"actor": OPUS, "editor": SONNET, "audit": OPUS}},
    "lead": {"round": "conduct9"},
    "noticed": {"k"},
    "heard": ["line"],
    "landed": [3],
    "owner_asks": {"A1": {"prompt": "prompt8", "quote": "q", "items": ["a"], "state": "open"}},
    "align_state": {"base": 40, "running": "rung5"},
    "comments": {"c1"},
  }
  made = Round(tmp_path)
  await made.word("\n".join(f"{name} = {value!r}" for name, value in old.items()))
  kept = {name: id(made.ns[name]) for name in old}
  await made.load()
  assert {name: id(made.ns[name]) for name in old} == kept
  assert await made.value("(cap(), model_of(queue[0]))") == (6, OPUS)


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
  await conductor.word('queue.append({"key": "late", "brief": "b", "thread": "t", "after": ["first"]})')
  await conductor.step("staff")
  await conductor.step("staff")
  assert conductor.hired() == []
  assert conductor.ns["heard"] == ["late waits: first has not ended, and no item holds it."]
  await conductor.word('held("first", pr=12)')
  await conductor.step("staff")
  assert conductor.ns["heard"][-1] == "late waits: #12 of first has not merged."
  await conductor.word('merges[12] = "w/first"')
  await conductor.step("staff")
  assert conductor.hired() == ["late"]


async def test_an_item_waits_while_a_live_worker_holds_a_file_that_changes_alone_and_a_test_file_holds_none(conductor: Round) -> None:
  one = await conductor.busy("one")
  await conductor.word(f'working[{one!r}]["areas"] = ["src/furb/engine.py", "test/conftest.py"]')
  await conductor.word('queue.extend([{"key": "two", "brief": "b", "thread": "t", "areas": ["src/furb", "test/conftest.py"]}, {"key": "three", "brief": "b", "thread": "t", "areas": ["test/conftest.py"]}])')
  await conductor.step("staff")
  assert conductor.hired() == ["three"]
  assert conductor.ns["heard"] == ["two waits: the worker of one holds src/furb/engine.py."]


async def test_an_item_waits_while_an_open_pr_of_the_round_changes_a_file_that_changes_alone_but_not_for_its_own_pr(conductor: Round) -> None:
  await conductor.word(
    'prs[9] = {"n": 9, "labels": ["w:mine"], "files": ["DOCTRINE.md", "tui/src/app.ts"]}\n'
    'queue.extend([{"key": "other", "brief": "b", "thread": "t", "areas": ["DOCTRINE.md"]}, {"key": "mine", "brief": "b", "thread": "t", "areas": ["DOCTRINE.md"]}, {"key": "shared", "brief": "b", "thread": "t", "areas": ["tui/src"]}])'
  )
  await conductor.step("staff")
  assert conductor.hired() == ["mine", "shared"]
  assert conductor.ns["heard"] == ["other waits: #9 changes DOCTRINE.md."]


async def test_an_item_with_no_areas_holds_no_file_whatever_its_brief_names(conductor: Round) -> None:
  await conductor.word(
    'prs[9] = {"n": 9, "labels": ["w:x"], "files": ["src/furb/engine.py"]}\nqueue.append({"key": "x", "brief": "b", "thread": "t"})\n'
    'queue.append({"key": "free", "brief": "Change src/furb/engine.py, as https://github.com/a/b/blob/main/src/furb/engine.py shows.", "thread": "t"})'
  )
  await conductor.step("staff")
  assert conductor.hired() == ["x", "free"]


async def test_an_item_whose_model_no_worker_runs_waits_with_its_cause(conductor: Round) -> None:
  await conductor.word('queue.append({"key": "big", "brief": "b", "thread": "t", "model": "claude-cli:opus/max"})')
  await conductor.step("staff")
  assert conductor.hired() == []
  assert conductor.ns["heard"] == [f"big waits: it asks claude-cli:opus/max, and a worker runs only {SONNET}, {OPUS}."]


async def test_hire_starts_a_worker_on_its_chain_with_the_asks_on_top_and_its_workspace(conductor: Round) -> None:
  await conductor.word('owner_asks["A3"] = {"prompt": "prompt3", "quote": "The cap holds at five workers.", "items": ["cap"], "state": "open"}')
  name = await conductor.step("hire", {"key": "cap", "brief": "Hold the cap.", "thread": "round"})
  [message] = conductor.briefs()
  where, path = conductor.engine.get(name)[3], f"{conductor.tmp}-w1"
  assert conductor.engine.get(where)[4] == "round/cap"
  assert message.startswith('This item carries the asks A3 of the owner, in his words:\n- A3: "The cap holds at five workers." (prompt3)\n')
  assert f"Work in the workspace {path}," in message
  assert 'labels=" --label w:cap --label thread:round"' in message
  assert message.endswith("The item, cap:\nHold the cap.")
  assert "{" + "workspace}" not in message and f"echo {where} >" in message
  assert f'on="{conductor.root}"' in message
  assert conductor.words(where) == [f"actor = {SONNET!r}", f"try:\n  cd({path!r})\nexcept Refused:\n  pass"]
  assert conductor.ns["working"][name] == {"key": "cap", "thread": "round", "model": SONNET, "slot": "w1"}
  assert not [x for x in conductor.gh() if x.startswith("label create")]


async def test_an_audit_runs_in_no_workspace_and_a_fix_takes_its_brief_whole(conductor: Round) -> None:
  look = await conductor.step("hire", {"key": "look", "brief": "Count the lines.", "kind": "audit"})
  assert "scratch://audit/look.md" in conductor.briefs()[-1]
  assert conductor.words(conductor.engine.get(look)[3]) == [f"actor = {OPUS!r}"]
  await conductor.step("hire", {"key": "fix4", "brief": "Fix #4 in {workspace}", "thread": "t", "kind": "fix"})
  assert conductor.briefs()[-1].startswith(f"Fix #4 in Work in the workspace {conductor.tmp}-w1,")


async def test_each_brief_forbids_a_window_on_this_machine(conductor: Round) -> None:
  for brief in ("WORKER", "AUDIT", "FIX", "ANSWER"):
    assert "Never open a window on this machine" in conductor.ns[brief]


async def test_each_brief_sends_each_build_through_the_slot_of_the_machine(conductor: Round) -> None:
  for brief in ("WORKER", "AUDIT", "FIX", "ANSWER"):
    assert "sh script/build-slot.sh <command>" in conductor.ns[brief]
    assert "no CARGO_BUILD_JOBS above 4" in conductor.ns[brief]
  assert "sh {repo}/script/build-slot.sh {repo}/script/worktree.sh {path}" in conductor.ns["WORKSPACE"]


async def test_conducting_and_audit_hold_the_rules_of_rank_and_of_the_frozen_fixes(conductor: Round) -> None:
  rule = " ".join(conductor.ns["CONDUCTING"].split())
  assert "does not become its own item or PR" in rule
  assert "waits in scratch://round/frozen.md" in rule
  assert "land together in one change" in rule
  assert "in the order of the register after the items of queue" in rule
  assert "unless the coordinator asks" in rule
  audit = " ".join(conductor.ns["AUDIT"].split())
  assert "in order of rank, the hard work first: a row that reshapes" in audit
  assert "reshapes a module, a contract or a protocol ranks before a row of polish or a simple row" in audit


async def test_a_worker_that_ends_with_a_pr_is_a_line_with_its_whole_report(conductor: Round) -> None:
  report = "I pushed one commit to main by mistake, then restored it."
  name = await conductor.busy("k")
  await conductor.end(name, (40, report))
  await conductor.word("tend()")
  assert conductor.ns["heard"] == [f"The worker k ({name}) ended with #40: {report}"]
  assert conductor.ns["working"] == {name: {"key": "k", "model": SONNET, "pr": 40}}


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
  await conductor.word('held("align8-old", pr=3)')
  name = await conductor.busy("align8", OPUS)
  await conductor.word(f'working[{name!r}]["kind"] = "audit"')
  await conductor.end(name, (0, "Two rows."))
  await conductor.word("tend()")
  key = "align8-gap-one"
  [item] = conductor.ns["queue"]
  assert item["brief"].endswith("Fix the gap.\n\nThis item is row D1 of the register scratch://audit/align8.md.")
  assert {k: v for k, v in item.items() if k != "brief"} == {"key": key, "thread": key, "model": SONNET, "areas": ["src/furb/engine.py"]}
  assert conductor.ns["heard"] == [
    f"The auditor align8 ({name}) ended: Two rows.",
    "The row D2 of the register of align8 is align8-old, which the round holds already, so it queued nothing for it.",
    f"The register of align8 queued {key}.",
  ]


async def test_an_ask_that_a_worker_leaves_queues_one_follow_up_unless_another_item_carries_it(conductor: Round) -> None:
  await conductor.word(
    'owner_asks.update(A1={"prompt": "prompt1", "quote": "q1", "items": ["k"], "state": "open"}, '
    'A2={"prompt": "prompt2", "quote": "q2", "items": ["k", "later"], "state": "open"})\nqueue.append({"key": "later", "brief": "b", "thread": "t"})'
  )
  first = await conductor.busy("k")
  await conductor.end(first, (5, "Served: A3\nLeft: A1 (no room), A2 (the item later)"))
  await conductor.word("tend()")
  asks = conductor.ns["owner_asks"]
  assert [q["key"] for q in conductor.ns["queue"]] == ["later", "ask-a1-1"]
  assert (asks["A1"]["items"], asks["A1"]["state"], asks["A2"]["state"]) == (["k", "ask-a1-1"], "partly", "open")
  assert json.loads(conductor.sand.files["scratch://owner/asks.json"])[0]["id"] == "A1"
  await conductor.word("queue.remove(queue[1])")
  second = await conductor.busy("ask-a1-1")
  await conductor.end(second, (0, "Left: A1 (still no room)"))
  await conductor.word("tend()")
  assert conductor.ns["owner_asks"]["A1"]["state"] == "decision"
  assert conductor.notices()[-1].startswith("The follow-up ask-a1-1 of the ask A1 of the owner left it again")
  assert [q["key"] for q in conductor.ns["queue"]] == ["later"]


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
    'A5={"prompt": "prompt5", "items": ["k"], "state": "decision"})\nqueue.append({"key": "q", "brief": "b", "thread": "t"})'
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


async def test_an_audit_of_each_target_starts_at_four_merges_on_astra_and_the_model_of_audit_in_turn(conductor: Round) -> None:
  await conductor.word(f'limits["models"]["astra"] = {ASTRA!r}\nalign()')
  assert conductor.ns["align_state"] == {"align": {"base": "0"}, "tui": {"base": "0"}}
  await conductor.word('merges.update(dict.fromkeys(range(3), "w"))\nalign()')
  assert conductor.ns["queue"] == []
  await conductor.word('merges[3] = "w"\nheld(*[f"align{n}" for n in range(1, 8)])')
  await conductor.word('lead["main"] = "abc"\nalign()')
  one, two = conductor.ns["queue"]
  brief, told = one["brief"], two["brief"]
  assert (one["key"], one["thread"], one["kind"], await conductor.value("model_of(queue[0])")) == ("align8", "align", "audit", OPUS)
  assert "scratch://owner/words.md, scratch://owner/picks.md" in brief and "scratch://owner/asks.json" in brief
  assert "git log --oneline -40 origin/main" in brief and "align7" in brief
  assert conductor.ns["align_state"]["align"] == {"base": "4", "n": "8", "sha": "abc"}
  assert (two["key"], two["thread"], two["kind"], await conductor.value("model_of(queue[1])")) == ("tui1", "tui", "audit", ASTRA)
  assert 'skill("converge-tui")' in told and "in $TMPDIR/audit/tui." in told and "{workspace} Check out origin/main there" in told
  await conductor.word('merges.update(dict.fromkeys(range(4, 20), "w"))\nalign()')
  assert len(conductor.ns["queue"]) == 2
  await conductor.word('queue.clear()\nheld("align8", "tui1")')
  await conductor.word("align()")
  assert [q["key"] for q in conductor.ns["queue"]] == ["align9", "tui2"]
  assert await conductor.value("[model_of(q) for q in queue]") == [ASTRA, OPUS]


async def test_an_audit_whose_brief_names_a_workspace_takes_one(conductor: Round) -> None:
  await conductor.step("hire", {"key": "tui1", "brief": "Capture the TUI. {workspace}", "kind": "audit"})
  [brief] = conductor.briefs()
  assert f"Capture the TUI. Work in the workspace {conductor.tmp}-w1," in brief
  assert "make no worktree but the workspace that your item names" in brief


@POSIX
@JQ
async def test_the_round_turns_on_auto_merge_for_its_prs_behind_the_ruleset_and_off_for_a_held_one(conductor: Round) -> None:
  await conductor.word('held("a", "b", "c", "d")')
  boxed = raw(4, "d", body="- [ ] Its prose obeys AGENTS.md")
  conductor.github(raw(1, "a"), raw(2, "b", labels=("hold",), autoMergeRequest={}), raw(3, "c", isDraft=True), boxed, raw(5, "other"))
  await conductor.step("sweep")
  assert [x for x in conductor.gh() if x.startswith("pr merge")] == ["pr merge 1 --auto --squash", "pr merge 2 --disable-auto"]


@POSIX
@JQ
async def test_with_no_ruleset_on_main_the_round_turns_on_no_auto_merge_and_tells_once(conductor: Round) -> None:
  await conductor.word('held("a")')
  conductor.github(raw(1, "a"), ruled=[])
  await conductor.step("sweep")
  await conductor.step("sweep")
  assert not [x for x in conductor.gh() if x.startswith("pr merge")]
  assert conductor.notices() == ["main has no ruleset that requires the check the gates, so the round turns on no auto-merge and merges no PR."]


@POSIX
@JQ
async def test_a_pr_of_the_round_that_merged_is_named_in_the_next_notice(conductor: Round) -> None:
  await conductor.word('held("a", "b")')
  conductor.github(raw(1, "a"), raw(2, "b"))
  await conductor.step("sweep")
  conductor.github(raw(2, "b"), merged=(1, 99))
  await conductor.step("sweep")
  assert (conductor.ns["landed"], conductor.ns["merges"]) == ([1], {1: "w/merged1", 99: "w/merged99"})


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
async def test_a_red_pr_whose_told_maker_ends_goes_to_a_fixer_on_a_fork_of_the_chain_of_that_maker_at_the_next_sweep(conductor: Round) -> None:
  maker = await conductor.busy("a")
  conductor.github(raw(1, "a", checks=(("the gates", "FAILURE"),), labels=("thread:t",)))
  await conductor.step("sweep")
  assert conductor.told()[0][0].startswith("Your open PR #1 has a failed check at sha1.")
  await conductor.end(maker, (1, "pushed"))
  await conductor.step("sweep")
  assert conductor.hired() == ["fix1"]
  assert conductor.engine.get(conductor.engine.get(conductor.ns["assigned"][1]["who"])[3])[4:6] == ("t/fix1", conductor.engine.get(maker)[3])
  assert conductor.ns["assigned"][1]["fixer"] and conductor.ns["assigned"][1]["tries"] == 1


@POSIX
@JQ
async def test_a_red_pr_with_no_live_maker_gets_at_most_two_fixers_for_a_head_then_a_notice(conductor: Round) -> None:
  await conductor.word('held("a")')
  conductor.github(raw(1, "a", mergeable="CONFLICTING", checks=()))
  for _ in range(4):
    await conductor.step("sweep")
    for name in await conductor.value("living()"):
      await conductor.end(name, (1, "no move"))
  assert conductor.hired() == ["fix1", "fix1"]
  [maker] = [w for w, x in conductor.ns["working"].items() if x["key"] == "a"]
  assert {conductor.engine.get(conductor.engine.get(w)[3])[5] for w, x in conductor.ns["working"].items() if x["key"] == "fix1"} == {conductor.engine.get(maker)[3]}
  assert "it conflicts with its base at sha1" in conductor.briefs()[0]
  assert conductor.notices() == ["#1 conflicts with its base at sha1, and 2 fixers ended at this head and did not move it, so the round sends no third one."]


@POSIX
@JQ
async def test_a_red_main_sends_no_fixer_to_a_red_pr_and_tells_once(conductor: Round) -> None:
  await conductor.word('held("a")')
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
async def test_a_red_pr_whose_failed_checks_main_fails_too_gets_no_fixer_and_one_line(conductor: Round) -> None:
  await conductor.word('held("a", "b")')
  conductor.github(raw(1, "a", checks=(("the gates", "FAILURE"), ("bench", "FAILURE"))), raw(2, "b", checks=(("the gates", "FAILURE"), ("ruff", "FAILURE"))))
  conductor.say(r"commits/main/check-runs", {"check_runs": [{"name": "the gates", "conclusion": "failure"}, {"name": "bench", "conclusion": "failure"}, {"name": "ruff", "conclusion": "success"}]})
  await conductor.step("sweep")
  await conductor.step("sweep")
  assert conductor.hired() == ["fix2"]
  assert conductor.ns["heard"] == ["#1 is red at sha1 only on bench, the gates, which main fails too, so the round hires no fixer for it."]


@POSIX
@JQ
async def test_a_pr_whose_checks_still_run_is_no_red_pr(conductor: Round) -> None:
  await conductor.word('held("a")')
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
  await conductor.word('held("b")')
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
  item = conductor.ns["queue"][0]
  key, brief = item["key"], item["brief"]
  assert key == "fix2" and item["kind"] == "fix" and "#2 i11 -: And this." in brief and "w/b" in brief
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
  await conductor.word('held("a")')
  conductor.github(raw(1, "a"))
  conductor.say(r"pr merge 1 --auto", "", 1)
  conductor.say(r"pr merge 1 --squash$", "", 1)
  await conductor.step("sweep")
  assert [x for x in conductor.gh() if x.startswith(("pr merge", "stack merge"))] == ["pr merge 1 --auto --squash", "pr merge 1 --squash", "stack merge 1 --yes --squash"]


async def test_a_wait_on_an_item_that_has_not_ended_is_no_line(conductor: Round) -> None:
  await conductor.word('queue.extend([{"key": "dep", "brief": "b", "thread": "t", "model": "claude-cli:opus/max"}, {"key": "late", "brief": "b", "thread": "t", "after": ["dep"]}])')
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
  await conductor.word('held("look-old")\nregister("look")')
  assert [q["key"] for q in conductor.ns["queue"]] == ["look-base", "look-top"]
  assert conductor.ns["queue"][1]["brief"].endswith("Use it.\n\nThis item is row D3 of the register scratch://audit/look.md.")
  assert [q.get("after") for q in conductor.ns["queue"]] == [None, ["look-old", "look-base"]]


def git(where: Path, *words: str) -> str:
  return subprocess.run(["git", "-C", str(where), *words], capture_output=True, text=True, check=True).stdout.strip()


@POSIX
async def test_settle_gives_no_workspace_with_work_off_a_branch_and_gives_one_on_the_head_of_a_merged_pr(conductor: Round, tmp_path: Path) -> None:
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
  holder = space / ".git" / "furb-holder"
  holder.write_text("chain-old\n", encoding="utf-8")

  def settle_with(heads: str) -> int:
    return subprocess.run(["sh", "-c", conductor.ns["SETTLE"], "sh", str(space), "chain-new", heads], capture_output=True, check=False).returncode

  (space / "b.txt").write_text("b\n", encoding="utf-8")
  assert settle_with("w/pr") == 5
  git(space, "add", "b.txt")
  git(space, "commit", "-q", "-m", "b")
  assert settle_with("w/other") == 5
  assert holder.read_text(encoding="utf-8") == "chain-old\n"
  assert settle_with("w/other w/pr") == 0
  assert holder.read_text(encoding="utf-8") == "chain-new\n"


@POSIX
async def test_hire_writes_the_chain_of_the_worker_in_its_workspace_before_its_prompt(conductor: Round) -> None:
  space = Path(f"{conductor.tmp}-w1")
  subprocess.run(["git", "init", "-q", str(space)], check=True)
  (space / ".git" / "furb-holder").write_text("chain-old\n", encoding="utf-8")
  name = await conductor.step("hire", {"key": "cap", "brief": "x"})
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
  name = await conductor.step("hire", {"key": "names", "brief": "x"})
  assert conductor.ns["working"][name]["slot"] == "w2"
  assert (space / ".git" / "furb-holder").read_text(encoding="utf-8") == f"{held}\n"


async def test_two_hires_in_one_tick_take_two_workspaces(conductor: Round) -> None:
  await conductor.word('queue.extend([{"key": "one", "brief": "b {workspace}", "thread": "t", "kind": "fix"}, {"key": "two", "brief": "b {workspace}", "thread": "t", "kind": "fix"}])')
  await conductor.step("staff")
  assert [x.get("slot") for x in conductor.ns["working"].values()] == ["w1", "w2"]


async def test_a_workspace_that_holds_work_off_a_branch_is_skipped_and_told_once(conductor: Round) -> None:
  kept = f"{conductor.tmp}-w1"
  await conductor.word(f'SETTLE = {f'[ "$1" != {kept!r} ] || exit 5'!r}')
  for key in ("one", "two"):
    await conductor.step("hire", {"key": key, "brief": "b {workspace}", "kind": "fix"})
  assert [x["slot"] for x in conductor.ns["working"].values()] == ["w2", "w3"]
  assert conductor.ns["heard"] == [f"The workspace {kept} holds work that no branch holds, so the round gives it to no worker until it is pushed or dropped."]


async def test_an_item_of_a_full_family_waits_for_its_family(conductor: Round) -> None:
  high = "claude-cli:opus/high"
  await conductor.word(f'limits["room"] = {{"opus/medium": 1, "opus/high": 3}}\nlimits["models"].update(audit={high!r}, lead={OPUS!r})')
  await conductor.busy("one", OPUS)
  await conductor.word(f'queue.extend([{{"key": "two", "brief": "b", "thread": "t", "model": {OPUS!r}}}, {{"key": "core", "brief": "b", "thread": "t", "model": {high!r}}}])')
  await conductor.step("staff")
  assert conductor.hired() == ["core"]
  assert conductor.ns["heard"] == ["two waits: opus/medium has no room."]


@POSIX
async def test_the_disk_frees_the_target_of_the_idle_workspace_hired_first_and_tells_once_when_it_stays_short(conductor: Round) -> None:
  for slot in ("w1", "w2", "w3"):
    (Path(f"{conductor.tmp}-{slot}") / "target").mkdir(parents=True)
  names = [await conductor.busy(f"k{n}") for n in (1, 2, 3)]
  await conductor.word("\n".join(f"working[{name!r}]['slot'] = 'w{n}'" for n, name in enumerate(names, 1)))
  await conductor.end(names[2], (1, "r"))
  await conductor.word("tend()")
  conductor.sand.now += 100
  await conductor.end(names[1], (2, "r"))
  await conductor.word("tend()")
  await conductor.step("keep_disk")
  assert all((Path(f"{conductor.tmp}-w{n}") / "target").exists() for n in (1, 2, 3))
  await conductor.word("KEEP = 10**9")
  await conductor.step("keep_disk")
  await conductor.step("keep_disk")
  assert [(Path(f"{conductor.tmp}-w{n}") / "target").exists() for n in (1, 2, 3)] == [True, False, False]
  [told] = conductor.notices()
  assert told.startswith("The disk holds ") and f"after the round deleted the target of each idle workspace: {conductor.tmp}-w2, {conductor.tmp}-w3.\n" in told


def conducted(made: Round) -> str:
  return next(x[1] for x in said(made.log, "conduct"))


def workers(made: Round) -> list[str]:
  return [str(x[1]) for x in said(made.log, "prompt") if x[4] == "tuple[int, str]"]


@POSIX
@JQ
@pytest.mark.parametrize("engine_of", ["python", "monty"], indirect=True)
async def test_a_round_tells_the_coordinator_only_its_notices_and_a_close_of_it_keeps_its_worker(tmp_path: Path, engine_of: str) -> None:
  made = Round(tmp_path)
  await made.load(TEST, 'queue.append({"key": "one", "brief": "b", "thread": "t"})\nround = conduct(60)')
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
  await made.load(TEST, 'queue.append({"key": "one", "brief": "b", "thread": "t"})\nround = conduct(60)')
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
  await made.load(TEST, 'queue.append({"key": "far", "brief": "b", "thread": "t", "model": "claude-cli:opus/max"})\nfirst = conduct(0.2)')
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
  await made.load(TEST, 'held("a")')
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
