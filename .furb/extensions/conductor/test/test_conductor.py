import ast
import asyncio
import builtins
import hashlib
import json
import os
import re
import shutil
import subprocess
import symtable
import sys
import time
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


def iso(at: float) -> str:
  return time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(at))


def check(name: str, status: str, conclusion: str, started: str | None) -> dict:
  return {"__typename": "CheckRun", "name": name, "status": status, "conclusion": conclusion, "startedAt": started}


def raw(n: int, head: str, minutes: int, rollup: list[dict]) -> dict:
  return {
    "number": n,
    "isDraft": False,
    "labels": [],
    "mergeable": "MERGEABLE",
    "mergeStateStatus": "CLEAN",
    "baseRefName": "main",
    "headRefName": head,
    "headRefOid": f"sha{n}",
    "statusCheckRollup": rollup,
    "updatedAt": iso(time.time() - minutes * 60),
    "body": "",
  }


def run(n: int, workflow: str, at: str, conclusion: str) -> dict:
  return {"databaseId": n, "workflowName": workflow, "createdAt": f"2026-10-23T{at}:00Z", "conclusion": conclusion}


class Round:
  def __init__(self, tmp: Path, chain: str = "chain1", modules: dict | None = None) -> None:
    self.tmp, self.now = tmp, 1000.0
    self.chain, self.modules = chain, {} if modules is None else modules
    self.rungs: list[tuple[str, str]] = []
    self.targets: list[str] = []
    self.says: list[tuple[str, str, int]] = []
    self.prompts: list[tuple[str, str]] = []
    self.cancelled: list[str] = []
    self.paused_chains: list[str] = []
    self.closed: list[tuple[str, object]] = []
    self.hired: list[tuple[str, str]] = []
    self.hired_models: list[str] = []
    self.files: dict[str, str] = {}
    self.facts: dict[str, tuple] = {}
    self.done: dict[str, object] = {}
    self.said: list[tuple] = []
    self.words: dict[str, str] = {}
    folder = tmp / "bin"
    folder.mkdir()
    (folder / "gh").write_text(f"#!{sys.executable}\n{GH}", encoding="utf-8")
    (folder / "gh").chmod(0o755)
    # The commands of the round sleep to let GitHub settle. The stub ends at once, so no test waits for GitHub.
    (folder / "sleep").write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
    (folder / "sleep").chmod(0o755)
    (tmp / "home.gitconfig").write_text("", encoding="utf-8")
    who = {"GIT_AUTHOR_NAME": "t", "GIT_AUTHOR_EMAIL": "t@t", "GIT_COMMITTER_NAME": "t", "GIT_COMMITTER_EMAIL": "t@t"}
    self.env = {
      # A python that coverage patches starts slower, and the stub of gh starts for each call of gh.
      **{n: v for n, v in os.environ.items() if not n.startswith(("GIT_", "COVERAGE_PROCESS_"))},
      **who,
      "GIT_CONFIG_GLOBAL": str(tmp / "home.gitconfig"),
      "GIT_CONFIG_NOSYSTEM": "1",
      "PATH": f"{folder}{os.pathsep}{os.environ['PATH']}",
      "GH_LOG": str(tmp / "gh.log"),
      "GH_SAYS": str(tmp / "says.json"),
    }
    self.say("^$", "")
    self.says.clear()
    self.say(r"workflows/gates\.yml/runs\?", '{"workflow_runs": []}')
    ns: dict = {"__name__": "conductor", "re": re, "Text": Text, "Exit": Exit, "Refused": Refused}
    ns.update(HIDDEN=None, OPERATOR="operator", ROOT="chain1", bash=self.bash, clock=lambda: self.now)
    ns.update(cwd=lambda: str(tmp), get=self.facts.get, over=lambda name: name in self.done, prompt=self.prompt)
    ns.update(peek=lambda name, waiting=None: self.done.get(name, waiting), read=self.read, write=self.write)
    ns.update(transcript=lambda on="", of="": [], paused=lambda name: False, scope=lambda name: self.chain)
    ns.update(
      acting=lambda: "rung1",
      wake=lambda name: None,
      span=lambda lo, hi: None,
      module=lambda on="": self.modules.get(on, {}),
    )
    ns.update(chain=lambda label="", source="": f"chain-{label}", rung=self.rung)
    ns.update(program=lambda on="": self.words)
    ns.update(cancel=lambda name: self.cancelled.append(name), say=lambda *fact: self.said.append(fact))
    ns.update(pause=lambda name: self.paused_chains.append(name), paused=lambda name: name in self.paused_chains)
    ns.update(wake=lambda name: self.paused_chains.remove(name) if name in self.paused_chains else None)
    ns.update(act=lambda kind, on, ear, *words: kind, close=lambda value, id="": self.closed.append((id, value)))
    ns.update(grant=lambda usd=None, share=None, on="": "grant")
    ns.update(filled=lambda about: None, gate=lambda word, on="": [], wait=self.wait)
    # Existing behavior tests use an explicit multi-family room map.
    ns["limits"] = {
      "room": {"opus": 1, "sonnet": 3, "opus/high": 1},
      "fixer": "claude-cli:sonnet/high",
      "models": {
        "actor": "claude-cli:opus/medium",
        "handover": "claude-cli:sonnet/medium",
        "editor": "claude-cli:sonnet/high",
        "patcher": "claude-cli:opus/medium",
        "ruler": "claude-cli:opus/medium",
        "stronger": "claude-cli:opus/high",
        "audit": "claude-cli:opus/medium",
      },
    }
    # The root plays pr before the conductor, which uses its names, as .furb/config.json orders them.
    exec(compile(IMPORT.sub("", PULL.read_text(encoding="utf-8")), str(PULL), "exec"), ns)  # noqa: S102
    exec(compile(IMPORT.sub("", SOURCE), str(HERE / "conductor.py"), "exec"), ns)  # noqa: S102
    self.real_hire = ns["hire"]
    ns.update(quietly=self.quietly, hire=self.hire)
    self.ns = ns
    self.modules[chain] = ns

  def __getattr__(self, name: str) -> object:
    return self.ns[name]

  def quietly(self, verb: object, *args: object, **words: object) -> object:
    got = verb(*args, **words)
    if asyncio.iscoroutine(got) and verb == self.bash:
      got.close()
    return got

  async def wait(self, seconds: float = 0, *acts: str, on: str = "") -> None:
    return None

  def say(self, pattern: str, out: str, code: int = 0) -> None:
    self.says.append((pattern, out, code))
    (self.tmp / "says.json").write_text(json.dumps(self.says), encoding="utf-8")

  def log(self) -> str:
    path = self.tmp / "gh.log"
    return path.read_text(encoding="utf-8") if path.exists() else ""

  def bash(self, command: str, stdin: str | None = "", timeout: float = 600, show: object = None, **_: object):
    async def done() -> Exit:
      got = subprocess.run(
        ["sh", "-c", command],
        input=stdin or "",
        capture_output=True,
        text=True,
        env=self.env,
        cwd=self.tmp,
        check=False,
      )
      return Exit(got.returncode, Text("stdout", got.stdout), Text("stderr", got.stderr))

    return done()

  def prompt(self, shape: object, message: str = "", to: str = "", on: str = "") -> str:
    self.prompts.append((message, on))
    self.targets.append(to)
    name = f"prompt{100 + len(self.prompts)}"
    self.facts.setdefault(name, ("prompt", name, "rung1", on, "None", message, to))
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
    self.hired.append((key, brief))
    self.hired_models.append(model)
    name = f"worker{len(self.hired)}"
    self.ns["working"][name] = key
    self.ns["models"][name] = self.ns["model_of"](key)
    self.facts[name] = ("prompt", name, "rung1", f"chain-{key}", "None", "", "")
    return name

  def busy(self, name: str, key: str, model: str) -> None:
    self.ns["working"][name] = key
    self.ns["models"][name] = model
    self.facts[name] = ("prompt", name, "rung1", f"chain-{key}", "None", "", "")

  def pr(self, n: int = 5, checks: tuple = (), **more: object) -> dict:
    one = {
      "n": n,
      "sha": "abc",
      "head": f"w/k{n}",
      "base": "main",
      "labels": [],
      "mergeable": "MERGEABLE",
      "draft": False,
      "checks": list(checks),
      "names": [f"c{i}" for i in range(len(checks))],
      "open": [],
      "idle": 0,
      **more,
    }
    one["verdict"] = self.ns["verdict"](one)
    return one


@pytest.fixture
def conductor(tmp_path: Path) -> Round:
  return Round(tmp_path)


@JQ
def test_a_check_counts_by_its_newest_run(conductor: Round) -> None:
  program = conductor.PRS.split("--jq '", 1)[1].split("'\nc=$?", 1)[0]
  title = [
    check("the title", "COMPLETED", c, f"2026-10-23T0{h}:00:00Z")
    for h, c in ((1, "FAILURE"), (3, "SUCCESS"), (2, "FAILURE"))
  ]
  hooks = [check("the hooks", "COMPLETED", "CANCELLED", "2026-10-23T01:00:00Z"), check("the hooks", "QUEUED", "", None)]
  given = json.dumps([raw(252, "w/x", 5, title + hooks)])
  got = json.loads(subprocess.run(["jq", program], input=given, capture_output=True, text=True, check=True).stdout)[0]
  assert dict(zip(got["names"], got["checks"], strict=True)) == {"the title": "SUCCESS", "the hooks": ""}


@POSIX
@JQ
async def test_a_cancelled_head_runs_again_once_then_counts_as_failed(conductor: Round) -> None:
  conductor.say(r"pr view 5 --json headRefOid", '{"headRefOid": "abc"}')
  runs = [
    run(1, "gates", "10:00", "cancelled"),
    run(2, "gates", "09:00", "success"),
    run(3, "titles", "09:30", "success"),
  ]
  conductor.say(r"run list --commit abc", json.dumps(runs))
  first = conductor.pr(checks=("CANCELLED", "SUCCESS"))
  assert first["verdict"] == "pending"
  assert await conductor.mend(first, 0.0, False) == "running"
  assert "run rerun 1\n" in conductor.log()
  assert conductor.heard == ["#5 had a run cancelled at its head abc, and the round ran it again."]
  assert conductor.hired == []
  again = conductor.pr(checks=("CANCELLED", "SUCCESS"))
  assert again["verdict"] == "fail"
  await conductor.mend(again, 0.0, False)
  assert conductor.log().count("run rerun") == 1
  assert [key for key, _ in conductor.hired] == ["fix5"]


@POSIX
@JQ
async def test_a_failure_behind_a_green_main_updates_its_branch_before_a_fixer(conductor: Round) -> None:
  conductor.say(r"compare/main\.\.\.abc", '{"behind_by": 3}')
  await conductor.mend(conductor.pr(checks=("FAILURE", "FAILURE")), 0.0, False)
  assert "pr update-branch 5\n" in conductor.log()
  assert conductor.heard == ["#5 failed at abc behind main, which is green, so the round updated its branch."]
  assert conductor.hired == []
  await conductor.mend(conductor.pr(checks=("FAILURE", "FAILURE")), 0.0, False)
  assert conductor.log().count("update-branch") == 1
  assert [key for key, _ in conductor.hired] == ["fix5"]


@POSIX
@JQ
async def test_a_red_main_updates_no_branch(conductor: Round) -> None:
  conductor.say(r"compare/main\.\.\.abc", '{"behind_by": 3}')
  conductor.say(r"run list --commit abc", json.dumps([{"databaseId": 7, "conclusion": "failure"}]))
  failed_run(conductor, 7, {"unit": "FAILED test/test_x.py::test_y - boom\n"})
  await conductor.mend(conductor.pr(checks=("FAILURE", "FAILURE")), 0.0, True)
  assert "update-branch" not in conductor.log()
  assert [key for key, _ in conductor.hired] == ["fix5"]


@POSIX
@JQ
async def test_a_green_pr_behind_main_updates_its_branch_though_heal_updated_it_at_that_head(conductor: Round) -> None:
  conductor.say(r"compare/main\.\.\.abc", '{"behind_by": 3}')
  conductor.say(r"compare/abc\.\.\.main", '{"files": [{"filename": "src/furb/engine.py"}]}')
  conductor.say(r"pr view 5 --json files", '{"files": [{"path": "src/furb/engine.py"}]}')
  await conductor.mend(conductor.pr(checks=("FAILURE", "FAILURE")), 0.0, False)
  assert conductor.log().count("update-branch") == 1
  green = conductor.pr(checks=("SUCCESS",), names=["the hooks"])
  await conductor.land(green, False)
  assert conductor.log().count("update-branch") == 2
  await conductor.land(green, False)
  assert conductor.log().count("update-branch") == 2


@POSIX
@JQ
async def test_a_draft_or_a_held_or_a_paused_pr_gets_no_branch_update_from_heal(conductor: Round) -> None:
  conductor.say(r"compare/main\.\.\.abc", '{"behind_by": 3}')
  for more in ({"draft": True}, {"labels": ["hold"]}, {"labels": ["hold:owner-review"]}, {"labels": ["paused"]}):
    assert await conductor.heal(conductor.pr(checks=("FAILURE", "FAILURE"), **more), False) == ""
  assert "update-branch" not in conductor.log()
  assert "heal update abc" not in conductor.noticed
  assert "behind main" in await conductor.heal(conductor.pr(checks=("FAILURE", "FAILURE")), False)


@POSIX
@JQ
async def test_a_lone_failure_reruns_its_failed_jobs_once_before_a_fixer(conductor: Round) -> None:
  conductor.say(r"compare/main\.\.\.abc", '{"behind_by": 0}')
  conductor.say(r"pr view 5 --json headRefOid", '{"headRefOid": "abc"}')
  conductor.say(r"run list --commit abc", json.dumps([run(1, "gates", "10:00", "failure")]))
  await conductor.mend(conductor.pr(checks=("FAILURE", "SUCCESS")), 0.0, False)
  assert "run rerun 1 --failed\n" in conductor.log()
  assert conductor.heard == ["#5 failed one check alone at abc, and the round ran its failed jobs again."]
  assert conductor.hired == []
  await conductor.mend(conductor.pr(checks=("FAILURE", "SUCCESS")), 0.0, False)
  assert conductor.log().count("run rerun") == 1
  assert [key for key, _ in conductor.hired] == ["fix5"]


@POSIX
@JQ
async def test_a_pr_on_main_with_no_run_of_the_gates_at_its_head_gets_one_once(conductor: Round) -> None:
  conductor.say(r"compare/main\.\.\.abc", '{"behind_by": 3}')
  title = conductor.pr(checks=("SUCCESS",), idle=10)
  assert await conductor.heal(title, False) == "#5 had no run of the gates at abc, so the round updated its branch."
  assert conductor.log().count("update-branch") == 1
  assert await conductor.heal(title, False) == ""
  assert conductor.log().count("update-branch") == 1


@POSIX
@JQ
async def test_a_pr_with_no_run_of_the_gates_under_a_red_main_reruns_the_workflows_at_its_head(conductor: Round) -> None:
  conductor.say(r"pr view 5 --json headRefOid", '{"headRefOid": "abc"}')
  conductor.say(r"run list --commit abc", json.dumps([run(7, "titles", "10:00", "success")]))
  got = await conductor.heal(conductor.pr(checks=("SUCCESS",), idle=10), True)
  assert got == "#5 had no run of the gates at abc, and the round ran its workflows again."
  assert "update-branch" not in conductor.log()
  assert "run rerun 7\n" in conductor.log()


@POSIX
@JQ
async def test_a_pr_with_a_run_of_the_gates_or_a_fresh_push_or_a_stack_base_gets_none(conductor: Round) -> None:
  conductor.say(r"compare/main\.\.\.abc", '{"behind_by": 3}')
  gated = conductor.pr(checks=("SUCCESS",), names=["the hooks"], idle=10)
  assert await conductor.heal(gated, False) == ""
  assert await conductor.heal(conductor.pr(checks=("SUCCESS",), idle=1), False) == ""
  assert await conductor.heal(conductor.pr(checks=("SUCCESS",), idle=10, base="w/k4"), False) == ""
  assert "update-branch" not in conductor.log()


def test_new_items_wait_while_an_open_pr_needs_a_worker(conductor: Round) -> None:
  conductor.efforts.update({key: SONNET for key in "abcdef"})
  conductor.queue[:] = [("a", "", ""), ("b", "", "")]
  conductor.backlog[:] = [("c", "", ""), ("d", "", ""), ("e", "", ""), ("f", "", "")]
  conductor.prs[7] = conductor.pr(n=7, checks=("FAILURE",))
  conductor.staff()
  assert conductor.hired == []
  conductor.working["prompt9"] = "fix7"
  conductor.assigned[7] = {"sha": "abc", "who": "prompt9", "at": 0.0, "fixer": True, "maker": "", "tries": 1}
  conductor.staff()
  assert [key for key, _ in conductor.hired] == ["a", "b", "c"]
  assert conductor.queue == [("d", "", ""), ("e", "", "")]
  assert conductor.backlog == [("f", "", "")]


@POSIX
@JQ
async def test_each_brief_forbids_a_window_on_this_machine(conductor: Round) -> None:
  rule = "Never open a window on this machine, not even for a test"
  assert rule in conductor.WORKER
  assert rule in conductor.FIX
  await conductor.mend(conductor.pr(n=8, open=["- [ ] It is one change."]), 0.0, False)
  assert [key for key, _ in conductor.hired] == ["box8"]
  assert rule in conductor.hired[0][1]


@POSIX
async def test_a_held_green_pr_shows_s_held(conductor: Round) -> None:
  conductor.prs[9] = conductor.pr(n=9, checks=("SUCCESS",), names=["the hooks"], labels=["hold"])
  await conductor.mark()
  assert "pr edit 9 --add-label s:held " in conductor.log()


@POSIX
async def test_a_pr_with_the_label_hold_owner_review_is_held_and_tended_by_no_fixer(conductor: Round) -> None:
  pr = conductor.pr(n=9, checks=("SUCCESS",), names=["the hooks"], labels=["hold:owner-review"])
  assert conductor.on_hold(pr) and not conductor.tended(pr)
  conductor.prs[9] = pr
  await conductor.mark()
  assert "pr edit 9 --add-label s:held " in conductor.log()


@POSIX
@JQ
async def test_an_upstream_ready_fork_pr_is_told_once(conductor: Round) -> None:
  ready = [{"name": "upstream-ready"}]
  forks = [
    {"number": 4, "headRefName": "w/x", "title": "T", "url": "u4", "labels": ready},
    {"number": 5, "headRefName": "w/y", "title": "U", "url": "u5", "labels": [*ready, {"name": "upstreamed"}]},
  ]
  conductor.say(r"pr list --repo uael/monty", json.dumps(forks))
  await conductor.upstream_watch()
  await conductor.upstream_watch()
  assert len(conductor.prompts) == 1
  assert conductor.prompts[0][0].startswith("The owner put the label upstream-ready on the fork PR u4 (w/x: T).")


def test_a_worker_that_ends_with_a_pr_is_a_line_and_its_debrief_feeds_the_ledger(conductor: Round) -> None:
  conductor.working["prompt1"] = "item1"
  conductor.facts["prompt1"] = ("prompt", "prompt1", "rung1", "chain5", "tuple[int, str]", "brief", "")
  conductor.done["prompt1"] = (12, "report")
  conductor.tend()
  assert conductor.heard == ["The worker item1 (prompt1) ended with PR #12, and results['prompt1'] keeps its report."]
  assert conductor.prompts == [(conductor.DEBRIEF, "chain5")]
  conductor.done["prompt101"] = ["No jq on the PATH", "A  slow gate", "a slow gate"]
  conductor.tend()
  conductor.tend()
  ledger = json.loads(conductor.files[conductor.TOOLING])
  assert [(x["what"], x["count"], x["source"]) for x in ledger] == [
    ("No jq on the PATH", 1, "debrief of item1"),
    ("A  slow gate", 2, "debrief of item1"),
  ]
  assert len(conductor.prompts) == 1

  async def pruned() -> str:
    return "The prune found nothing to remove."

  conductor.ns["prune"] = pruned
  asyncio.run(conductor.audit(False))
  told = conductor.prompts[-1][0]
  assert "- e1 2x A  slow gate\n- e0 1x No jq on the PATH\nThe prune found nothing to remove." in told


def test_a_report_that_names_a_mistake_is_a_notice_of_its_own_and_not_a_line(conductor: Round) -> None:
  report = "IMPORTANT, a mistake of mine: I pushed them to origin/main, which deleted nearly all files from main."
  for name, key, pr, said in (("prompt1", "item1", 12, report), ("prompt2", "item2", 13, "Opened it. The gates pass.")):
    conductor.working[name] = key
    conductor.facts[name] = ("prompt", name, "rung1", "chain5", "tuple[int, str]", "brief", "")
    conductor.done[name] = (pr, said)
  conductor.tend()
  assert conductor.heard == ["The worker item2 (prompt2) ended with PR #13, and results['prompt2'] keeps its report."]
  told = [message for message, _ in conductor.prompts if "names a mistake" in message]
  assert len(told) == 1 and told[0].startswith("The report of the worker item1 (prompt1) names a mistake")
  assert report in told[0] and "The worker item1 (prompt1) ended with PR #12" in told[0]
  assert "item2" not in told[0]
  conductor.tend()
  assert len([message for message, _ in conductor.prompts if "names a mistake" in message]) == 1


@POSIX
@JQ
async def test_a_notice_carries_its_context_and_each_other_event_is_a_line(conductor: Round) -> None:
  prs = [
    raw(1, "feature", 5, []),
    raw(2, "w/idle", 60, [check("the title", "COMPLETED", "SUCCESS", "2026-10-23T10:00:00Z")]),
    raw(3, "w/busy", 60, [check("the hooks", "IN_PROGRESS", "", "2026-10-23T10:00:00Z")]),
  ]
  conductor.say(r"pr list --state open", json.dumps(prs))
  conductor.say(r"pr list --state merged", "[]")
  conductor.say(r"run list --branch main", json.dumps([{"headSha": "m1", "conclusion": "failure", "databaseId": 9}]))
  failed_run(conductor, 9, {"unit": "FAILED test/test_x.py::test_y - boom\nok\n"})
  conductor.say(r"issue list", "[]")
  conductor.say(r"pr list --repo uael/monty", "[]")
  await conductor.sweep(300)
  told = [message for message, _ in conductor.prompts]
  assert told[0] == (
    "main is red at m1. Find its cause first. The lines that failed:\nunit\tFAILED test/test_x.py::test_y - boom"
  )
  assert told[1].startswith("#2 has had no change for ")
  assert "Its checks at sha2: the title SUCCESS. Move it on, or say why it waits." in told[1]
  assert told[1].endswith(
    "- #1 (feature) was not made by a worker of the round, so the round neither mends nor merges it."
  )
  assert len(told) == 2


@POSIX
@JQ
async def test_a_pr_that_does_not_merge_tells_what_gh_said(conductor: Round) -> None:
  conductor.say(r"compare/sha7\.\.\.main", '{"files": [{"filename": "b.py"}]}')
  conductor.say(r"pr view 7 --json files", '{"files": [{"path": "a.py"}]}')
  conductor.say(r"pr merge 7", "X Pull request #7 is not mergeable: the merge commit cannot be cleanly created.\n", 1)
  conductor.say(r"pr view 7 --json state", '{"state": "OPEN"}')
  await conductor.land(conductor.pr(n=7, sha="sha7", checks=("SUCCESS",), names=["the hooks"]), False)
  assert conductor.prompts[0][0].startswith(
    "#7 did not merge at sha7, and gh said:\nX Pull request #7 is not mergeable: the merge commit cannot be cleanly"
  )


def git(*args: str, cwd: Path, env: dict) -> str:
  return subprocess.run(["git", *args], cwd=cwd, env=env, capture_output=True, text=True, check=True).stdout


def aged(path: Path, days: float) -> None:
  then = time.time() - days * 86400
  for one in [path, *path.rglob("*")]:
    os.utime(one, (then, then), follow_symlinks=False)


@POSIX
@pytest.mark.skipif(not shutil.which("lsof"), reason="the prune reads the open files with lsof")
def test_the_prune_removes_what_the_work_left_and_keeps_its_work(conductor: Round, tmp_path: Path) -> None:
  env = conductor.env
  origin, root = tmp_path / "origin.git", tmp_path / "furb"
  git("init", "-q", "--bare", "-b", "main", str(origin), cwd=tmp_path, env=env)
  git("init", "-q", "-b", "main", str(root), cwd=tmp_path, env=env)
  (root / "a.txt").write_text("a\n", encoding="utf-8")
  git("add", "-A", cwd=root, env=env)
  git("commit", "-q", "-m", "a", cwd=root, env=env)
  git("remote", "add", "origin", str(origin), cwd=root, env=env)
  git("push", "-q", "-u", "origin", "main", cwd=root, env=env)
  main = git("rev-parse", "main", cwd=root, env=env).strip()
  for key in ("merged", "open", "busy", "empty", "fresh"):
    git("worktree", "add", "-q", "-b", f"w/{key}", str(tmp_path / f"furb-w-{key}"), cwd=root, env=env)
  merged = tmp_path / "furb-w-merged"
  (merged / "b.txt").write_text("b\n", encoding="utf-8")
  git("add", "-A", cwd=merged, env=env)
  git("commit", "-q", "-m", "b", cwd=merged, env=env)
  (merged / "notes.txt").write_text("not committed\n", encoding="utf-8")
  for key in ("merged", "open", "busy", "empty"):
    aged(tmp_path / f"furb-w-{key}", 3)
  git("push", "-q", "origin", "w/open", "main:refs/heads/w/closed", "main:refs/heads/w/moved", cwd=root, env=env)
  listed = [
    {"number": 1, "state": "MERGED", "headRefName": "w/merged", "headRefOid": "x", "isCrossRepository": False},
    {"number": 2, "state": "OPEN", "headRefName": "w/open", "headRefOid": main, "isCrossRepository": False},
    {"number": 3, "state": "CLOSED", "headRefName": "w/busy", "headRefOid": main, "isCrossRepository": False},
    {"number": 4, "state": "CLOSED", "headRefName": "w/closed", "headRefOid": main, "isCrossRepository": False},
    {"number": 5, "state": "CLOSED", "headRefName": "w/moved", "headRefOid": "old", "isCrossRepository": False},
  ]
  conductor.say(r"pr list --state all", json.dumps(listed))
  scratch, folder = tmp_path / "scratch", tmp_path / "tmp"
  record = tmp_path / "life.jsonl"
  record.write_text("", encoding="utf-8")
  life = hashlib.sha256(os.path.realpath(record).encode()).hexdigest()[:16]
  for name in ("old1", "new1", "held1", life):
    (scratch / name).mkdir(parents=True)
    (scratch / name / "f").write_text("f", encoding="utf-8")
  for name in ("furb-old", "furb-new", "other-old"):
    (folder / name).mkdir(parents=True)
  for one in (scratch / "old1", scratch / "held1", scratch / life, folder / "furb-old", folder / "other-old"):
    aged(one, 3)
  spec = {"root": str(root), "keys": ["busy"], "text": "", "archive": ".furb/archive", "trashed": ".furb/trashed.tsv"}
  (tmp_path / "spec.json").write_text(json.dumps({**spec, "tmp": str(folder)}), encoding="utf-8")
  with (scratch / "held1" / "f").open(encoding="utf-8"), record.open(encoding="utf-8"):
    got = subprocess.run(
      ["python3", "-I", "-", str(tmp_path / "spec.json"), str(scratch)],
      input=conductor.PRUNE_PY,
      capture_output=True,
      text=True,
      env=env,
      cwd=tmp_path,
      check=True,
      timeout=25,
    )
  assert got.stdout.startswith("The prune removed the worktrees ")
  assert "; 1 remote branch, listed in .furb/trashed.tsv; 1 scratch directory; 1 folder of " in got.stdout
  assert sorted(p.name for p in tmp_path.glob("furb-w-*")) == ["furb-w-busy"]
  assert git("for-each-ref", "--format=%(refname:short)", "refs/heads/w", cwd=root, env=env).split() == ["w/busy"]
  kept = sorted((root / ".furb" / "archive").glob("*/*"))
  assert [re.sub(r"-\d{6}\.", ".", p.name) for p in kept] == ["merged.bundle", "merged.patch"]
  assert "notes.txt" in kept[1].read_text(encoding="utf-8")
  assert "refs/heads/w/merged" in git("bundle", "list-heads", str(kept[0]), cwd=root, env=env)
  heads = git("ls-remote", "--heads", "origin", cwd=root, env=env)
  assert sorted(line.split("\t")[1] for line in heads.splitlines()) == [
    "refs/heads/main",
    "refs/heads/w/moved",
    "refs/heads/w/open",
  ]
  assert (root / ".furb" / "trashed.tsv").read_text(encoding="utf-8").split("\t")[1:4] == ["w/closed", main, "4"]
  assert sorted(p.name for p in scratch.iterdir()) == sorted(["new1", "held1", life])
  assert sorted(p.name for p in folder.iterdir()) == ["furb-new", "other-old"]


@POSIX
@JQ
async def test_a_flake_of_the_gates_is_queued_at_once_and_read_once(conductor: Round) -> None:
  conductor.says.clear()
  conductor.say(r"workflows/gates\.yml/runs\?", '{"workflow_runs": []}')
  await conductor.watch_ci()
  assert conductor.queue == []
  conductor.says.clear()
  url = "https://github.com/o/r/actions/runs/50"
  flaky = [{"id": 50, "head_sha": "s1", "run_attempt": 2, "conclusion": "success", "html_url": url}]
  job = {
    "id": 7,
    "name": "the bindings and the TUI on ubuntu-latest",
    "conclusion": "failure",
    "started_at": "2026-10-23T10:00:00Z",
    "completed_at": "2026-10-23T10:01:00Z",
  }
  conductor.say(r"runs\?branch=main&status=success", '{"workflow_runs": []}')
  conductor.say(r"runs\?per_page=30", json.dumps({"workflow_runs": flaky}))
  conductor.say(r"runs/50/attempts/1/jobs", json.dumps({"jobs": [job]}))
  conductor.say(r"jobs/7/logs", "2026-10-23T10:00:30.0000000Z (fail) a host test [30000.17ms]\n")
  await conductor.watch_ci()
  await conductor.watch_ci()
  assert [(key, thread) for key, _, thread in conductor.queue] == [("flake-a-host-test", "ci")]
  assert f"{url}/attempts/1" in conductor.queue[0][1]
  assert "a host test" in conductor.queue[0][1]


@POSIX
@JQ
async def test_a_flake_that_the_newest_runs_of_main_healed_is_queued_by_no_item(conductor: Round) -> None:
  conductor.says.clear()
  conductor.say(r"workflows/gates\.yml/runs\?", '{"workflow_runs": []}')
  await conductor.watch_ci()
  conductor.says.clear()
  url = "https://github.com/o/r/actions/runs/50"
  name = "the bindings and the TUI on ubuntu-latest"
  flaky = [
    {
      "id": 50,
      "head_sha": "s1",
      "run_attempt": 2,
      "conclusion": "success",
      "html_url": url,
      "created_at": "2026-10-23T10:00:00Z",
    }
  ]
  green = [{"id": n, "run_attempt": 1, "created_at": "2026-10-24T10:00:00Z"} for n in range(1, 6)]
  ok = {
    "id": 1,
    "name": name,
    "conclusion": "success",
    "started_at": "2026-10-24T10:00:00Z",
    "completed_at": "2026-10-24T10:01:00Z",
  }
  bad = {**ok, "id": 7, "conclusion": "failure"}
  conductor.say(r"runs\?branch=main&status=success", json.dumps({"workflow_runs": green}))
  conductor.say(r"runs\?per_page=30", json.dumps({"workflow_runs": flaky}))
  conductor.say(r"runs/50/attempts/1/jobs", json.dumps({"jobs": [bad]}))
  conductor.say(r"runs/\d+/jobs", json.dumps({"jobs": [ok]}))
  conductor.say(r"jobs/7/logs", "fps p95 frame   21.3 ms <= 16.7 OVER\n")
  await conductor.watch_ci()
  assert conductor.queue == []
  assert conductor.flaky["bench: fps p95 frame"] == [f"{url}/attempts/1"]
  assert any("the 5 newest runs of main" in line for line in conductor.heard)


@POSIX
@JQ
async def test_a_job_over_its_budget_is_queued_once_until_new_runs_measure_it(conductor: Round) -> None:
  tree = conductor.tmp
  (tree / ".github").mkdir()
  (tree / ".github" / "budgets.json").write_text('{"the hooks": 60}\n', encoding="utf-8")
  git("init", "-q", "-b", "main", cwd=tree, env=conductor.env)
  git("add", ".github", cwd=tree, env=conductor.env)
  git("commit", "-q", "-m", "budgets", cwd=tree, env=conductor.env)
  git("update-ref", "refs/remotes/origin/main", "HEAD", cwd=tree, env=conductor.env)
  conductor.says.clear()
  green = {"workflow_runs": [{"id": n} for n in range(1, 6)]}
  job = {
    "id": 1,
    "name": "the hooks",
    "conclusion": "success",
    "started_at": "2026-10-23T10:00:00Z",
    "completed_at": "2026-10-23T10:03:00Z",
  }
  conductor.say(r"runs\?branch=main&status=success", json.dumps(green))
  conductor.say(r"runs/\d+/jobs", json.dumps({"jobs": [job]}))
  conductor.say(r"runs\?per_page=30", '{"workflow_runs": []}')
  await conductor.watch_ci()
  await conductor.watch_ci()
  assert [(key, thread) for key, _, thread in conductor.queue] == [("budget-the-hooks", "ci")]
  assert "took 180 s" in conductor.queue[0][1]
  assert "budget of 60 s" in conductor.queue[0][1]


@POSIX
@JQ
async def test_a_veto_keeps_a_green_pr_from_a_merge(conductor: Round) -> None:
  asked: list[int] = []

  async def deny(pr: dict) -> bool:
    asked.append(pr["n"])
    return True

  conductor.vetoes["judge"] = deny
  await conductor.land(conductor.pr(n=7, sha="sha7", checks=("SUCCESS",), names=["the hooks"]), False)
  assert asked == [7]
  assert "pr merge" not in conductor.log()


def judging(conductor: Round, diff: str = "+rung(read(path, HIDDEN))\n") -> list[str]:
  ran: list[str] = []

  async def bash(command: str, stdin: str | None = "", timeout: float = 600, show: object = None, **_: object) -> Exit:
    ran.append(command)
    return Exit(0, Text("stdout", diff), Text("stderr", ""))

  def quietly(verb: object, *args: object, **words: object) -> object:
    got = verb(*args, **words)  # ty: ignore[call-non-callable]
    if asyncio.iscoroutine(got):
      got.close()
    return got

  conductor.ns.update(bash=bash, quietly=quietly)
  conductor.rulings["k"] = ["an explicit crafted turn rung(read(path, HIDDEN))\nwith no hidden control flow"]
  return ran


def answered(conductor: Round, key: str, gaps: list, answer: str = "open") -> object:
  earlier = conductor.judged[key].get("earlier") or []
  return (gaps, [(gap, answer) for gap in earlier]) if earlier else gaps


def test_hire_puts_the_words_of_the_owner_at_the_top_of_the_brief(conductor: Round) -> None:
  conductor.ns["quietly"] = lambda verb, *args, **words: (
    asyncio.run(verb(*args, **words)) if verb == conductor.bash else verb(*args, **words)
  )
  conductor.kinds["look"] = "audit"
  conductor.rulings["look"] = ['say "no"\nthen stop', "one more"]
  conductor.real_hire("look", "find the leaks", "defects")
  assert conductor.prompts[-1][0].startswith(
    conductor.RULED + '\n\n> say "no"\n> then stop\n\n> one more\n\nYou are an auditor'
  )
  conductor.real_hire("other", "x", "defects", "A brief.")
  assert conductor.prompts[-1][0].startswith("A brief.")


async def test_a_pr_with_no_rulings_passes_the_veto_of_the_words_with_no_judge(conductor: Round) -> None:
  ran = judging(conductor)
  conductor.rulings.clear()
  pr = conductor.pr(n=7, sha="sha7", labels=["w:k"], checks=("SUCCESS",))
  conductor.prs[7] = pr
  assert await conductor.bent(pr) is False
  assert (ran, conductor.prompts, conductor.judged) == ([], [], {})


async def test_a_blind_judge_reads_the_words_and_the_diff_and_a_head_that_it_cleared_merges(conductor: Round) -> None:
  ran = judging(conductor)
  pr = conductor.pr(n=7, sha="sha7", labels=["w:k"], checks=("SUCCESS",))
  conductor.prs[7] = pr
  assert await conductor.bent(pr) is True
  assert ran == ["git fetch -q origin main refs/pull/7/head && git diff origin/main...sha7"]
  message, on = conductor.prompts[-1]
  assert (on, conductor.targets[-1]) == ("chain-judge/k/7", "claude-cli:opus/medium")
  assert "an explicit crafted turn rung(read(path, HIDDEN))\nwith no hidden control flow" in message
  assert message.endswith("The diff of #7 at sha7:\n\n+rung(read(path, HIDDEN))\n")
  assert ("actor = 'claude-cli:opus/medium'", "chain-judge/k/7") in conductor.rungs
  act = conductor.judged["7 sha7"]["act"]
  assert await conductor.bent(pr) is True
  assert len(conductor.prompts) == 1
  conductor.done[act] = []
  assert await conductor.bent(pr) is False
  assert await conductor.bent(pr) is False
  assert conductor.judged["7 sha7"]["state"] == "cleared"
  del conductor.prs[7]
  assert await conductor.bent(conductor.pr(n=8, sha="sha8")) is False
  assert conductor.judged == {}


async def test_a_head_with_a_gap_does_not_merge_and_its_worker_gets_the_gaps(conductor: Round) -> None:
  judging(conductor)
  pr = conductor.pr(n=7, sha="sha7", labels=["w:k"], checks=("SUCCESS",))
  conductor.prs[7] = pr
  conductor.busy("worker9", "k", "claude-cli:sonnet/high")
  await conductor.bent(pr)
  conductor.done[conductor.judged["7 sha7"]["act"]] = [("rung(read(path,  HIDDEN))", "It reads in an ear, in a.py.")]
  assert await conductor.bent(pr) is True
  message, on = conductor.prompts[-1]
  assert on == "chain-k"
  assert '- "rung(read(path, HIDDEN))": It reads in an ear, in a.py.' in message
  assert conductor.judged["7 sha7"]["told"] is True
  assert await conductor.bent(pr) is True
  assert len(conductor.prompts) == 2
  moved = conductor.pr(n=7, sha="sha9", labels=["w:k"], checks=("SUCCESS",))
  conductor.prs[7] = moved
  assert await conductor.bent(moved) is True
  assert conductor.prompts[-1][1] == "chain-judge/k/7"


async def test_a_judge_that_fails_twice_tells_the_coordinator_and_holds_the_pr(conductor: Round) -> None:
  judging(conductor)
  pr = conductor.pr(n=7, sha="sha7", labels=["w:k"], checks=("SUCCESS",))
  conductor.prs[7] = pr
  await conductor.bent(pr)
  conductor.done[conductor.judged["7 sha7"]["act"]] = Refused("no answer")
  assert await conductor.bent(pr) is True
  assert await conductor.bent(pr) is True
  assert len(conductor.prompts) == 2
  conductor.done[conductor.judged["7 sha7"]["act"]] = [("words that he never wrote", "A gap.")]
  assert await conductor.bent(pr) is True
  assert conductor.judged["7 sha7"]["state"] == "stuck"
  assert "failed twice on #7 at sha7" in conductor.prompts[-1][0]
  assert await conductor.bent(pr) is True
  assert len(conductor.prompts) == 3


@POSIX
@JQ
async def test_the_veto_of_the_words_keeps_a_green_pr_from_a_merge(conductor: Round) -> None:
  judging(conductor)
  pr = conductor.pr(n=7, sha="sha7", labels=["w:k"], checks=("SUCCESS",), names=["the hooks"])
  conductor.prs[7] = pr
  await conductor.land(pr, False)
  assert conductor.prompts[-1][1] == "chain-judge/k/7"
  assert "pr merge" not in conductor.log()


def test_conducting_says_the_veto_of_the_words_and_the_rule_of_a_statement(conductor: Round) -> None:
  for words in (
    "an item may hold his exact words in rulings[key]",
    "A judge that fails twice",
    "states only what a command of the same word proved, and names each other part as a hypothesis",
  ):
    assert words in conductor.CONDUCTING


def test_the_environment_of_each_git_holds_no_variable_of_a_commit(
  tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
  for name in ("GIT_INDEX_FILE", "GIT_DIR", "GIT_WORK_TREE", "GIT_PREFIX", "GIT_EXEC_PATH"):
    monkeypatch.setenv(name, "/elsewhere")
  env = Round(tmp_path).env
  assert sorted(name for name in env if name.startswith("GIT_")) == [
    "GIT_AUTHOR_EMAIL",
    "GIT_AUTHOR_NAME",
    "GIT_COMMITTER_EMAIL",
    "GIT_COMMITTER_NAME",
    "GIT_CONFIG_GLOBAL",
    "GIT_CONFIG_NOSYSTEM",
  ]


OPUS = "claude-cli:opus/medium"
SONNET = "claude-cli:sonnet/medium"
TOP = "claude-cli:opus/high"
SONNET_HIGH = "claude-cli:sonnet/high"


def test_the_default_round_uses_codex_luna_for_each_role(conductor: Round) -> None:
  luna = "codex:gpt-6-luna/medium"
  assert conductor.LIMITS["fixer"] == luna
  assert conductor.LIMITS["room"] == {"gpt-6-luna": 5}
  assert conductor.LIMITS["models"] == dict.fromkeys(
    ("actor", "handover", "editor", "patcher", "ruler", "stronger", "audit"), luna
  )


def test_the_rulings_of_the_owner_stand_in_conducting_in_his_words(conductor: Round) -> None:
  for words in ('limits["models"]', 'actor, handover, editor, patcher, ruler, stronger and audit', "reload keeps your choices"):
    assert words in conductor.CONDUCTING
  assert conductor.ACTOR == OPUS
  assert conductor.limits == {
    "room": {"opus": 1, "sonnet": 3, "opus/high": 1},
    "fixer": SONNET_HIGH,
    "models": {
      "actor": OPUS, "handover": SONNET, "editor": SONNET_HIGH, "patcher": OPUS,
      "ruler": OPUS, "stronger": TOP, "audit": OPUS,
    },
  }
  assert conductor.cap() == 5
  for words in ('limits["room"] gives the room', 'limits["fixer"]', "a reload keeps your choices"):
    assert words in conductor.CONDUCTING


def test_a_fixer_runs_on_sonnet_high_unless_its_effort_says_otherwise(conductor: Round) -> None:
  assert [conductor.model_of(key) for key in ("fix5", "box8", "k")] == [SONNET_HIGH, SONNET_HIGH, SONNET_HIGH]
  conductor.efforts["fix5"] = TOP
  assert conductor.model_of("fix5") == TOP
  assert conductor.model_of("fix50") == SONNET_HIGH


def test_a_reload_adds_model_roles_to_a_round_that_predates_them(conductor: Round) -> None:
  conductor.limits.pop("models")
  exec(compile(IMPORT.sub("", SOURCE), str(HERE / "conductor.py"), "exec"), conductor.ns)  # noqa: S102
  luna = "codex:gpt-6-luna/medium"
  assert conductor.limits["models"] == dict.fromkeys(
    ("actor", "handover", "editor", "patcher", "ruler", "stronger", "audit"), luna
  )


def test_the_limits_of_the_round_live_in_its_state_and_a_reload_keeps_them(conductor: Round) -> None:
  conductor.limits["room"]["opus"] = 2
  conductor.limits["fixer"] = OPUS
  luna = "codex:gpt-6-luna/medium"
  conductor.limits["models"] = dict.fromkeys(conductor.limits["models"], luna)
  conductor.limits["room"] = {"gpt-6-luna": 5}
  exec(compile(IMPORT.sub("", SOURCE), str(HERE / "conductor.py"), "exec"), conductor.ns)  # noqa: S102
  assert conductor.ACTOR == conductor.HANDOVER == conductor.EDITOR == luna
  assert conductor.PATCHER == conductor.RULER == conductor.STRONGER == luna
  assert conductor.KINDS == {"audit": luna, "edit": luna}
  assert luna in conductor.MODELS
  assert conductor.limits["room"] == {"gpt-6-luna": 5}
  assert (conductor.cap(), conductor.model_of("box8")) == (5, OPUS)
  conductor.busy("p1", "a", luna)
  assert conductor.room(luna)
  conductor.busy("p2", "b", luna)
  assert conductor.room(luna)
  conductor.busy("p3", "c", luna)
  conductor.busy("p4", "d", luna)
  conductor.busy("p5", "e", luna)
  assert not conductor.room(luna)
  assert conductor.family(luna) == "gpt-6-luna"


STATE = (
  "limits queue backlog working results relays bases efforts homes models lead custom prs merged noticed assigned silent"
  " comments measures timers noted debriefs briefed owns watched after parked awaited covering kinds pool slots areas touched serial round_faults vetoes rulings judged tips shifted parents redmain onmain hired paced landed heard bindings handled ends requeued"
  " statused secret updated budgeted flaky runs_read deferred"
).split()


def test_no_name_of_the_state_of_the_round_or_of_converge_is_a_name_of_the_engine() -> None:
  furb = HERE.parents[2]
  tree = ast.parse((furb / "src" / "furb" / "engine.py").read_text(encoding="utf-8"))
  engine: set[str] = set()
  for node in tree.body:
    if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
      engine.add(node.name)
    elif isinstance(node, ast.Assign):
      engine |= {x.id for x in node.targets if isinstance(x, ast.Name)}
    elif isinstance(node, ast.AnnAssign) and isinstance(node.target, ast.Name):
      engine.add(node.target.id)
    elif isinstance(node, ast.TypeAlias):
      engine.add(node.name.id)
    elif isinstance(node, (ast.Import, ast.ImportFrom)):
      engine |= {(x.asname or x.name).split(".")[0] for x in node.names}
  bound = set()
  for word in ("conductor/conductor.py", "converge/converge.py"):
    text = (furb / ".furb" / "extensions" / word).read_text(encoding="utf-8")
    bound |= set(re.findall(r'globals\(\)\.setdefault\("(\w+)"', text))
  assert "queue" in bound
  assert sorted(bound & engine) == []


def play(conductor: Round) -> None:
  exec(compile(IMPORT.sub("", SOURCE), str(HERE / "conductor.py"), "exec"), conductor.ns)  # noqa: S102


def test_the_round_plays_on_the_root_alone_and_pr_on_every_chain_before_it() -> None:
  furb = HERE.parent.parent
  manifest = lambda name: json.loads((furb / "extensions" / name / "furb.json").read_text(encoding="utf-8"))
  assert [manifest(name).get("plays", "every") for name in ("pr", "conductor", "converge")] == ["every", "root", "root"]
  order = list(json.loads((furb / "config.json").read_text(encoding="utf-8"))["extensions"])
  assert order.index("pr") < order.index("conductor")
  assert re.findall(r"^(?:async )?def (\w+)", PULL.read_text(encoding="utf-8"), re.M) == ["pr_box", "pr_body", "prcheck", "pr", "checked", "shq"]
  assert re.findall(r"^def (pr|prcheck|shq)\(", SOURCE, re.M) == []


def test_each_name_of_the_state_binds_its_default_only_when_the_module_lacks_it() -> None:
  bound = re.findall(r'^(\w+): .+ = globals\(\)\.setdefault\("\1", ', SOURCE, re.M)
  assert bound == STATE
  assert re.findall(r"^\w+: [^=\n]+ = (?:\[\]|\{\}|set\(\))$", SOURCE, re.M) == []


def test_the_word_plays_on_a_module_of_the_previous_conductor_that_binds_no_name_of_the_state(
  conductor: Round,
) -> None:
  for name in STATE:
    del conductor.ns[name]
  play(conductor)
  luna = "codex:gpt-6-luna/medium"
  assert conductor.limits == {
    "room": {"gpt-6-luna": 5},
    "fixer": luna,
    "models": dict.fromkeys(("actor", "handover", "editor", "patcher", "ruler", "stronger", "audit"), luna),
  }
  assert conductor.cap() == 5
  for name in STATE[1:]:
    assert conductor.ns[name] in ([], {}, set()), name
  assert type(conductor.ns["merged"]) is set
  assert type(conductor.ns["queue"]) is list
  assert type(conductor.ns["prs"]) is dict


def test_the_word_plays_on_a_module_that_holds_the_state_and_keeps_each_value() -> None:
  # A new Round binds the state once; the word then plays again on the same module with values in every name.
  import tempfile

  with tempfile.TemporaryDirectory() as folder:
    conductor = Round(Path(folder))
    held = {name: conductor.ns[name] for name in STATE}
    conductor.limits["room"]["opus"] = 2
    for name in STATE[1:]:
      one = held[name]
      if isinstance(one, list):
        one.append("kept")
      elif isinstance(one, set):
        one.add("kept")
      else:
        one["kept"] = "kept"
    play(conductor)
    for name in STATE:
      assert conductor.ns[name] is held[name], name
    assert conductor.limits["room"] == {"opus": 2, "sonnet": 3, "opus/high": 1}
    assert conductor.cap() == 6
    for name in STATE[1:]:
      one = held[name]
      assert "kept" in one, name


def test_the_gate_finds_no_name_of_the_state_undefined_in_the_word() -> None:
  import furb
  import furb_monty
  from furb import sheet

  word = IMPORT.sub("", SOURCE)
  found = sheet.gate(vars(furb.python), {}, word, furb_monty.gate)
  assert [x for x in found if any(f"`{name}`" in x for name in STATE)] == []


def stacked(conductor: Round, tmp_path: Path, clash: bool = False) -> dict[str, str]:
  env, origin = conductor.env, tmp_path / "origin.git"
  git("init", "-q", "--bare", str(origin), cwd=tmp_path, env=env)
  git("init", "-q", "-b", "main", cwd=tmp_path, env=env)
  git("remote", "add", "origin", str(origin), cwd=tmp_path, env=env)

  def commit(name: str, text: str, message: str) -> None:
    (tmp_path / name).write_text(text, encoding="utf-8")
    git("add", name, cwd=tmp_path, env=env)
    git("commit", "-q", "-m", message, cwd=tmp_path, env=env)

  commit("base.txt", "base\n", "base")
  git("push", "-q", "origin", "main", cwd=tmp_path, env=env)
  shas = {}
  for branch, name, text in (
    ("w/parent", "p.txt", "one\n"),
    ("w/kid", "k.txt", "kid\n"),
    ("w/deep", "d.txt", "deep\n"),
  ):
    git("checkout", "-q", "-b", branch, cwd=tmp_path, env=env)
    if branch == "w/kid" and clash:
      name, text = "p.txt", "kid\n"
    commit(name, text, branch[2:])
    git("push", "-q", "origin", branch, cwd=tmp_path, env=env)
    shas[branch] = git("rev-parse", "HEAD", cwd=tmp_path, env=env).strip()
  git("checkout", "-q", "main", cwd=tmp_path, env=env)
  git("merge", "--squash", "-q", "w/parent", cwd=tmp_path, env=env)
  git("commit", "-q", "-m", "parent (#7)", cwd=tmp_path, env=env)
  if clash:
    commit("p.txt", "main\n", "main moves")
  git("push", "-q", "origin", "main", cwd=tmp_path, env=env)
  shas["main"] = git("rev-parse", "HEAD", cwd=tmp_path, env=env).strip()
  shas["squash"] = git("rev-parse", "HEAD" if not clash else "HEAD^", cwd=tmp_path, env=env).strip()
  conductor.say(r"compare/", '{"files": [{"filename": "b.py"}]}')
  conductor.say(r"pr view 7 --json files", '{"files": [{"path": "a.py"}]}')
  conductor.say(r"pr view 7 --json mergeCommit", json.dumps({"mergeCommit": {"oid": shas["squash"]}}))
  conductor.say(r"pr view 7 --json state", '{"state": "MERGED"}')
  conductor.prs[7] = conductor.pr(n=7, head="w/parent", sha=shas["w/parent"], checks=("SUCCESS",), names=["the hooks"])
  conductor.prs[8] = conductor.pr(n=8, head="w/kid", base="w/parent", sha=shas["w/kid"])
  conductor.prs[9] = conductor.pr(n=9, head="w/deep", base="w/kid", sha=shas["w/deep"])
  return shas


def history(tmp_path: Path, conductor: Round, branch: str) -> list[str]:
  out = git("log", "--format=%s", branch, cwd=tmp_path / "origin.git", env=conductor.env)
  return out.splitlines()


CHECKS = ["checked"] * 6


def branch(conductor: Round, name: str = "w/k") -> None:
  subprocess.run(["git", "init", "-q", "-b", name], cwd=conductor.tmp, env=conductor.env, check=True)
  conductor.say(r"pr view --json number", '{"number": 311}')
  conductor.say(r"pr create", "https://github.com/o/r/pull/9\n")


@POSIX
@JQ
def test_a_worker_runs_only_sonnet_or_opus_at_medium_or_high(conductor: Round) -> None:
  conductor.ns["quietly"] = lambda verb, *args, **words: (
    asyncio.run(verb(*args, **words)) if verb == conductor.bash else verb(*args, **words)
  )
  for model in ("claude-cli:opus/xhigh", "claude-cli:opus/max", "claude-cli:sonnet/max", "claude-cli:haiku/medium"):
    conductor.efforts["k"] = model
    with pytest.raises(Refused, match="xhigh and max"):
      conductor.real_hire("k", "w")
  assert conductor.working == {}
  assert conductor.log() == ""
  for model in (SONNET, "claude-cli:sonnet/high", OPUS, TOP):
    conductor.efforts["k"] = model
    name = conductor.real_hire("k", "w")
    assert conductor.models[name] == model
    assert conductor.targets[-1] == model
  assert "label create w:k --force" in conductor.log()
  del conductor.efforts["k"]
  assert conductor.models[conductor.real_hire("k", "w")] == SONNET_HIGH
  assert conductor.targets[-1] == SONNET_HIGH
  assert conductor.files["scratch://build.sh"] == conductor.BUILD_SH


def test_an_item_that_asks_for_xhigh_waits_and_the_owner_hears(conductor: Round) -> None:
  conductor.efforts["x"] = "claude-cli:opus/xhigh"
  conductor.queue[:] = [("x", "", ""), ("y", "", "")]
  conductor.staff()
  assert [key for key, _ in conductor.hired] == ["y"]
  assert conductor.queue == [("x", "", "")]
  assert len(conductor.prompts) == 1
  assert "xhigh" in conductor.prompts[0][0]
  conductor.staff()
  assert len(conductor.prompts) == 1


def test_the_round_runs_one_on_opus_medium_three_on_sonnet_and_one_on_opus_high(conductor: Round) -> None:
  models = {"o1": OPUS, "t1": TOP, "o2": OPUS, "t2": TOP, "s1": SONNET, "s2": SONNET, "s3": SONNET}
  models["s4"] = SONNET_HIGH
  conductor.efforts.update(models)
  conductor.queue[:] = [(key, "", "") for key in models]
  conductor.staff()
  assert [key for key, _ in conductor.hired] == ["o1", "t1", "s1", "s2", "s3"]
  assert conductor.queue == [("o2", "", ""), ("t2", "", ""), ("s4", "", "")]
  conductor.done["worker1"] = None
  conductor.staff()
  assert [key for key, _ in conductor.hired][5:] == ["o2"]
  assert conductor.queue == [("t2", "", ""), ("s4", "", "")]
  conductor.done["worker2"] = None
  conductor.staff()
  assert [key for key, _ in conductor.hired][6:] == ["t2"]


def test_an_item_hires_on_its_own_room_while_a_pr_waits_for_a_fixer_that_no_room_allows(conductor: Round) -> None:
  conductor.limits["room"].update(sonnet=0, opus=0)
  conductor.efforts.update(s=SONNET, o=TOP)
  conductor.queue[:] = [("s", "", ""), ("o", "", "")]
  conductor.prs[7] = conductor.pr(n=7, checks=("FAILURE",))
  conductor.staff()
  assert [key for key, _ in conductor.hired] == ["o"]
  assert conductor.queue == [("s", "", "")]
  assert len(conductor.prompts) == 1
  assert conductor.prompts[0][0].startswith("#7 needs a fixer on claude-cli:sonnet/high")
  conductor.staff()
  assert len(conductor.prompts) == 1


def test_an_item_with_no_room_keeps_its_place_while_the_next_with_room_starts(conductor: Round) -> None:
  conductor.busy("p1", "a", OPUS)
  conductor.busy("p2", "b", OPUS)
  conductor.busy("p3", "e", TOP)
  conductor.efforts.update(c=OPUS, d=SONNET)
  conductor.queue[:] = [("c", "", ""), ("d", "", "")]
  conductor.staff()
  assert [key for key, _ in conductor.hired] == ["d"]
  assert conductor.queue == [("c", "", "")]


@POSIX
@JQ
async def test_a_fixer_goes_before_a_new_item_and_waits_for_room_on_sonnet(conductor: Round) -> None:
  for name, key in (("p1", "a"), ("p2", "b"), ("p3", "e")):
    conductor.busy(name, key, SONNET)
  conductor.busy("p4", "f", OPUS)
  conductor.prs[8] = conductor.pr(n=8, open=["- [ ] It is one change."], verdict="open")
  conductor.efforts["c"] = SONNET
  conductor.queue[:] = [("c", "", "")]
  await conductor.mend(conductor.prs[8], 0.0, False)
  conductor.staff()
  assert conductor.hired == []
  conductor.done["p1"] = None
  await conductor.mend(conductor.prs[8], 0.0, False)
  assert [key for key, _ in conductor.hired] == ["box8"]
  assert conductor.models["worker1"] == SONNET_HIGH


@POSIX
@JQ
async def test_a_fixer_that_ends_with_its_head_unmoved_is_replaced_at_once_and_twice_at_most(conductor: Round) -> None:
  pr = conductor.pr(n=8, open=["- [ ] It is one change."], verdict="open")
  conductor.prs[8] = pr
  conductor.efforts["c"] = SONNET
  for at in (0.0, 10.0, 20.0):
    await conductor.mend(pr, at, False)
    if conductor.assignee(pr):
      conductor.done[conductor.assignee(pr)] = (0, "nothing to fix")
      conductor.tend()
  assert [key for key, _ in conductor.hired] == ["box8", "box8"]
  conductor.queue[:] = [("c", "", "")]
  conductor.staff()
  assert [key for key, _ in conductor.hired] == ["box8", "box8", "c"]


@POSIX
@JQ
async def test_an_ended_worker_goes_on_in_its_own_chain_on_sonnet(conductor: Round) -> None:
  conductor.homes["k8"] = "chain7"
  pr = conductor.pr(n=8, open=["- [ ] It is one change."], labels=["w:k8"])
  for index, key in enumerate("abc"):
    conductor.busy(f"p{index}", key, SONNET)
  conductor.busy("p4", "d", OPUS)
  await conductor.mend(pr, 0.0, False)
  assert (conductor.hired, conductor.rungs, conductor.prompts) == ([], [], [])
  conductor.done["p0"] = None
  await conductor.mend(pr, 0.0, False)
  assert conductor.hired == []
  assert conductor.rungs[0] == (f"actor = {SONNET!r}", "chain7")
  assert [on for _, on in conductor.rungs] == ["chain7", "chain7"] and conductor.rungs[1][0].startswith("try:\n  cd(")
  assert conductor.targets == [SONNET]
  message, on = conductor.prompts[0]
  assert on == "chain7"
  assert message.startswith(f"Your work on this chain goes on, now on {SONNET}. Your open PR #8")
  name = next(name for name, key in conductor.working.items() if key == "k8")
  assert conductor.models[name] == SONNET
  await conductor.mend(pr, 0.0, False)
  assert len(conductor.prompts) == 1


@POSIX
@JQ
async def test_an_ended_worker_goes_on_on_its_own_model_when_sonnet_has_no_room(conductor: Round) -> None:
  conductor.homes["k8"] = "chain7"
  conductor.ends["k8"] = [("p9", 0)]
  conductor.models["p9"] = OPUS
  pr = conductor.pr(n=8, open=["- [ ] It is one change."], labels=["w:k8"])
  assert conductor.mender(pr) == SONNET
  for index, key in enumerate("abc"):
    conductor.busy(f"p{index}", key, SONNET)
  conductor.busy("p4", "e", OPUS)
  assert conductor.mender(pr) == SONNET
  conductor.done["p4"] = None
  assert conductor.mender(pr) == OPUS
  await conductor.mend(pr, 0.0, False)
  assert conductor.hired == []
  assert conductor.rungs[0] == (f"actor = {OPUS!r}", "chain7")
  assert [on for _, on in conductor.rungs] == ["chain7", "chain7"] and conductor.rungs[1][0].startswith("try:\n  cd(")
  assert conductor.targets == [OPUS]
  message, on = conductor.prompts[0]
  assert on == "chain7"
  assert message.startswith(f"Your work on this chain goes on, now on {OPUS}. Your open PR #8")
  name = next(name for name, key in conductor.working.items() if key == "k8")
  assert conductor.models[name] == OPUS


def test_the_round_follows_the_compaction_of_its_orchestrator(conductor: Round, tmp_path: Path) -> None:
  (tmp_path / "new").mkdir()
  new = Round(tmp_path / "new", "chain2", conductor.modules)
  calls: list[str] = []

  async def waits(every: float, *acts: str) -> None:
    new.lead.update(conductor.lead)
    assert new.follow() == "chain2"

  async def swept(every: float) -> None:
    calls.append("sweep")

  conductor.queue.append(("a", "", ""))
  conductor.ns.update(tend=lambda: calls.append("tend"), staff=lambda: None, wait=waits, sweep=swept)
  assert asyncio.run(conductor.conduct()) == {}
  assert calls == ["tend", "sweep"]
  assert conductor.lead == {"home": "chain2"}
  assert new.lead == {"home": "chain2"}
  assert conductor.prompts == []


@POSIX
@JQ
def test_a_gh_that_answers_nothing_once_ends_no_round(conductor: Round) -> None:
  conductor.say(r"issue list", "[]")
  conductor.say(r"pr list --repo uael/monty", "[]")
  conductor.queue.append(("a", "", ""))
  rounds: list[int] = []

  async def waits(every: float, *acts: str) -> None:
    rounds.append(len(rounds))
    if len(rounds) == 1:
      assert "sweep" not in conductor.timers
      conductor.facts["worker1"] = ("prompt", "worker1", "rung1", "chain-a", "None", "", "")
      conductor.say(r"pr list --state open", json.dumps([raw(4, "w/four", 5, [])]))
      conductor.say(r"pr list --state merged", '[{"number": 3}]')
      conductor.say(r"run list --branch main", json.dumps([{"headSha": "m1", "conclusion": "success"}]))
    else:
      conductor.lead["home"] = "chain2"

  conductor.ns["wait"] = waits
  assert asyncio.run(conductor.conduct()) == {}
  assert rounds == [0, 1]
  assert [message for message, _ in conductor.prompts] == [
    "The command that lists the PRs failed at the open PRs, the merged PRs, the gates of main: gh said nothing."
  ]
  assert "sweep" in conductor.timers
  assert list(conductor.prs) == [4]
  assert 3 in conductor.merged
  assert [key for key, _ in conductor.hired] == ["a"]


def test_a_fault_of_a_step_is_told_once_and_the_round_goes_on(conductor: Round) -> None:
  calls: list[str] = []

  def tend() -> None:
    calls.append("tend")
    json.loads("")

  async def swept(every: float) -> None:
    calls.append("sweep")

  async def waits(every: float, *acts: str) -> None:
    if calls.count("sweep") == 3:
      conductor.lead["home"] = "chain2"

  conductor.queue.append(("a", "", ""))
  conductor.ns.update(tend=tend, sweep=swept, staff=lambda: None, wait=waits)
  assert asyncio.run(conductor.conduct()) == {}
  assert calls == ["tend", "sweep"] * 3
  assert [message for message, _ in conductor.prompts] == [
    "The round on chain1 met a fault in its step tend: JSONDecodeError: Expecting value: line 1 column 1 (char 0)."
    " The round goes on at the next round, and tells this fault again only after a round of this step with no fault."
  ]


def test_a_fault_of_a_step_is_told_again_with_its_count_after_a_round_with_no_fault(conductor: Round) -> None:
  calls: list[str] = []

  def tend() -> None:
    calls.append("tend")
    if len(calls) != 2:
      json.loads("")

  async def swept(every: float) -> None:
    return None

  async def waits(every: float, *acts: str) -> None:
    if len(calls) == 4:
      conductor.lead["home"] = "chain2"

  conductor.queue.append(("a", "", ""))
  conductor.ns.update(tend=tend, sweep=swept, staff=lambda: None, wait=waits)
  asyncio.run(conductor.conduct())
  told = [message for message, _ in conductor.prompts]
  assert len(told) == 2 and "It is the round" not in told[0]
  assert "JSONDecodeError" in told[1] and "It is the round 2 with this fault in this life." in told[1]
  assert conductor.round_faults == {"fault tend JSONDecodeError": 3}


def test_a_round_that_ends_with_no_work_tells_it_as_a_decision(conductor: Round) -> None:
  async def swept(every: float) -> None:
    return None

  conductor.ns.update(tend=lambda: None, sweep=swept, staff=lambda: None)
  assert asyncio.run(conductor.conduct()) == {}
  assert conductor.prompts == [
    (
      "The round on chain1 ended: it holds no item in queue, in backlog or in working, and no open PR that it tends."
      " Decide whether it goes on: until you await conduct() again, no sweep, no heal, no merge, no fixer and no"
      " hire runs, and no worker is tended.",
      "",
    )
  ]


def test_a_fault_that_ends_the_round_tells_its_cause(conductor: Round) -> None:
  async def swept(every: float) -> None:
    return None

  async def waits(every: float, *acts: str) -> None:
    fault = RuntimeError("boom")
    raise fault

  conductor.queue.append(("a", "", ""))
  conductor.ns.update(tend=lambda: None, sweep=swept, staff=lambda: None, wait=waits)
  with pytest.raises(RuntimeError, match="boom"):
    asyncio.run(conductor.conduct())
  assert len(conductor.prompts) == 1
  assert conductor.prompts[0][0].startswith(
    "The round on chain1 ended: the fault RuntimeError: boom, in its step wait."
  )


@POSIX
@JQ
async def test_a_merge_rebases_the_child_onto_main_and_pushes_it(conductor: Round, tmp_path: Path) -> None:
  shas = stacked(conductor, tmp_path)
  del conductor.prs[9]
  await conductor.land(conductor.prs[7], False)
  assert history(tmp_path, conductor, "w/kid") == ["kid", "parent (#7)", "base"]
  assert git("rev-parse", "w/kid^", cwd=tmp_path / "origin.git", env=conductor.env).strip() == shas["squash"]
  assert "pr edit 8 --base main" in conductor.log()
  assert "update-branch" not in conductor.log()
  assert conductor.heard == []
  assert conductor.prompts == []
  assert len(git("worktree", "list", cwd=tmp_path, env=conductor.env).splitlines()) == 1


@POSIX
@JQ
async def test_pr_creates_a_new_pr_when_the_branch_held_only_a_merged_one(conductor: Round) -> None:
  branch(conductor)
  got = await conductor.ns["pr"](
    "feat(tui): Show records", "It matters.", ["It works."], CHECKS, where=str(conductor.tmp)
  )
  assert got.code == 0
  log = conductor.log()
  assert "pr create" in log
  assert "pr edit" not in log


@POSIX
@JQ
async def test_a_rebase_that_conflicts_pushes_nothing_and_leaves_the_child_to_a_fixer(
  conductor: Round, tmp_path: Path
) -> None:
  shas = stacked(conductor, tmp_path, clash=True)
  del conductor.prs[9]
  await conductor.land(conductor.prs[7], False)
  assert git("rev-parse", "w/kid", cwd=tmp_path / "origin.git", env=conductor.env).strip() == shas["w/kid"]
  assert len(conductor.heard) == 1
  assert conductor.heard[0].startswith("#8 was not rebased onto main, exit 2: w/kid conflicts with main")
  assert len(git("worktree", "list", cwd=tmp_path, env=conductor.env).splitlines()) == 1
  kid = conductor.pr(n=8, head="w/kid", mergeable="CONFLICTING", base="main", sha=shas["w/kid"])
  assert await conductor.mend(kid, 1e9, False) == "conflicting"
  assert [key for key, _ in conductor.hired] == ["fix8"]


@POSIX
@JQ
async def test_a_child_that_moved_since_the_sweep_is_left_as_it_is(conductor: Round, tmp_path: Path) -> None:
  shas = stacked(conductor, tmp_path)
  conductor.prs[8]["sha"] = "0" * 40
  del conductor.prs[9]
  await conductor.land(conductor.prs[7], False)
  assert git("rev-parse", "w/kid", cwd=tmp_path / "origin.git", env=conductor.env).strip() == shas["w/kid"]
  assert conductor.heard[0].startswith("#8 was not rebased onto main, exit 4: w/kid moved since the sweep")


@POSIX
@JQ
async def test_a_grandchild_follows_the_rebased_child(conductor: Round, tmp_path: Path) -> None:
  stacked(conductor, tmp_path)
  await conductor.land(conductor.prs[7], False)
  assert history(tmp_path, conductor, "w/kid") == ["kid", "parent (#7)", "base"]
  assert history(tmp_path, conductor, "w/deep") == ["deep", "kid", "parent (#7)", "base"]
  assert conductor.heard == []


@POSIX
@JQ
async def test_pr_edits_the_open_pr_of_the_branch(conductor: Round) -> None:
  branch(conductor)
  conductor.say(r"pr list --state open --head w/k", '[{"number": 7}]')
  conductor.say(r"pr view 7 --json url", '{"url": "https://github.com/o/r/pull/7"}')
  got = await conductor.ns["pr"](
    "feat(tui): Show records", "It matters.", ["It works."], CHECKS, where=str(conductor.tmp)
  )
  assert got.code == 0
  log = conductor.log()
  assert "pr edit 7 --title" in log
  assert "pr create" not in log


def viewing(sha: str, mergeable: str = "MERGEABLE") -> str:
  return json.dumps({"state": "OPEN", "headRefOid": sha, "mergeable": mergeable})


VIEW = r"pr view 5 --json state,headRefOid,mergeable"


@POSIX
@JQ
async def test_an_order_is_dropped_when_the_head_moved_before_it_goes(conductor: Round) -> None:
  conductor.say(VIEW, viewing("zzz", "CONFLICTING"))
  assert await conductor.mend(conductor.pr(mergeable="CONFLICTING"), 0.0, False) == "running"
  assert (conductor.hired, conductor.prompts) == ([], [])


@POSIX
@JQ
async def test_an_order_is_dropped_when_the_conflict_is_gone_before_it_goes(conductor: Round) -> None:
  conductor.say(VIEW, viewing("abc", "MERGEABLE"))
  assert await conductor.mend(conductor.pr(mergeable="CONFLICTING"), 0.0, False) == "running"
  assert (conductor.hired, conductor.prompts) == ([], [])


@POSIX
@JQ
async def test_an_order_goes_when_the_pr_still_holds_its_head_and_its_conflict(conductor: Round) -> None:
  conductor.say(VIEW, viewing("abc", "CONFLICTING"))
  await conductor.mend(conductor.pr(mergeable="CONFLICTING"), 0.0, False)
  assert [key for key, _ in conductor.hired] == ["fix5"]


@POSIX
@JQ
async def test_a_pr_with_a_fixer_gets_no_order_and_no_second_fixer(conductor: Round) -> None:
  conductor.working.update(p9="fix5", p8="box8")
  conductor.assigned[5] = {"sha": "abc", "who": "p9", "at": 0.0, "fixer": True, "maker": "", "tries": 1}
  conductor.assigned[8] = {"sha": "abc", "who": "p8", "at": 0.0, "fixer": True, "maker": "", "tries": 1}
  assert await conductor.mend(conductor.pr(sha="def", mergeable="CONFLICTING"), 0.0, False) == "conflicting"
  assert await conductor.mend(conductor.pr(n=8, open=["- [ ] It is one change."]), 0.0, False) == "red"
  assert (conductor.hired, conductor.prompts) == ([], [])


@POSIX
@JQ
async def test_the_round_leaves_the_branch_of_a_pr_with_a_fixer_alone_and_merges_it_not(conductor: Round) -> None:
  conductor.working["p9"] = "fix5"
  conductor.assigned[5] = {"sha": "abc", "who": "p9", "at": 0.0, "fixer": True, "maker": "", "tries": 1}
  conductor.say(r"compare/main\.\.\.abc", '{"behind_by": 3}')
  assert await conductor.heal(conductor.pr(checks=("FAILURE", "FAILURE")), False) == ""
  await conductor.land(conductor.pr(n=5, checks=("SUCCESS",), names=["the hooks"]), False)
  log = conductor.log()
  assert "update-branch" not in log
  assert "pr merge" not in log


@POSIX
@JQ
async def test_the_worker_that_owns_a_pr_hears_that_the_round_updated_its_branch(conductor: Round) -> None:
  conductor.busy("p1", "k5", SONNET)
  conductor.say(r"compare/main\.\.\.abc", '{"behind_by": 3}')
  pr = conductor.pr(checks=("FAILURE", "FAILURE"), labels=["w:k5"])
  assert "updated its branch" in await conductor.heal(pr, False)
  message, on = conductor.prompts[0]
  assert on == "chain-k5"
  assert message.startswith("The round updated the branch of your PR #5 with main. Fetch origin before you push")


@POSIX
@JQ
async def test_the_maker_gets_one_order_for_each_head_in_trouble(conductor: Round) -> None:
  conductor.busy("p1", "k5", SONNET)
  one = {"labels": ["w:k5"], "mergeable": "CONFLICTING"}
  await conductor.mend(conductor.pr(**one), 0.0, False)
  await conductor.mend(conductor.pr(**one), 100.0, False)
  assert len(conductor.prompts) == 1
  await conductor.mend(conductor.pr(sha="def", **one), 200.0, False)
  assert len(conductor.prompts) == 2 and conductor.assigned[5]["sha"] == "def"


@POSIX
@JQ
async def test_a_fixer_that_starts_on_a_pr_with_a_live_worker_tells_that_worker(conductor: Round) -> None:
  conductor.busy("p1", "k5", SONNET)
  pr = conductor.pr(labels=["w:k5"], mergeable="CONFLICTING")
  await conductor.mend(pr, 0.0, False)
  await conductor.mend(pr, 4000.0, False)
  assert [key for key, _ in conductor.hired] == ["fix5"]
  assert conductor.prompts[-1][0].startswith("A fixer starts on your PR #5. Leave its branch alone")



@POSIX
@JQ
async def test_a_conflict_whose_owner_does_not_move_its_head_in_fifteen_minutes_goes_to_a_fixer(conductor: Round) -> None:
  conductor.busy("p1", "k5", SONNET)
  pr = conductor.pr(labels=["w:k5"], mergeable="CONFLICTING")
  await conductor.mend(pr, 0.0, False)
  assert len(conductor.prompts) == 1 and "15 minutes" in conductor.prompts[0][0]
  await conductor.mend(pr, 800.0, False)
  assert conductor.hired == []
  await conductor.mend(pr, 1000.0, False)
  assert [key for key, _ in conductor.hired] == ["fix5"]

def test_the_brief_of_a_fixer_says_how_to_share_a_branch_and_to_wait_for_github(conductor: Round) -> None:
  for words in ("the workspace that the end of this brief names", "If the head of the PR is not {sha} any more", "headRefOid,mergeable", "20 seconds"):
    assert words in conductor.FIX


def test_a_stacked_pr_whose_base_moved_names_the_old_and_the_new_head_in_its_order(conductor: Round) -> None:
  conductor.prs[7] = conductor.pr(n=7, head="w/parent", sha="new")
  conductor.prs[8] = conductor.pr(n=8, head="w/kid", base="w/parent", mergeable="CONFLICTING", sha="kid1")
  conductor.tips["w/parent"] = "old"
  conductor.shift()
  assert conductor.shifted == {8: ("old", "new")}
  assert conductor.tips == {"w/parent": "new", "w/kid": "kid1"}
  assert conductor.based(conductor.prs[8]).endswith(
    "whose head changed from old to new: git rebase --onto origin/w/parent old w/kid puts the PR on it."
  )
  assert conductor.based(conductor.pr(n=9)) == ""


@POSIX
@JQ
async def test_pr_stops_when_where_is_missing_and_runs_no_gh(conductor: Round) -> None:
  branch(conductor)
  with pytest.raises(Refused):
    await conductor.ns["pr"](
      "feat(tui): Show records", "It matters.", ["It works."], CHECKS, where=str(conductor.tmp / "none")
    )
  assert conductor.log() == ""


@POSIX
@JQ
async def test_the_fixer_of_a_stacked_pr_gets_the_base_that_changed(conductor: Round) -> None:
  conductor.prs[7] = conductor.pr(n=7, head="w/parent", sha="new")
  kid = conductor.pr(n=8, head="w/kid", base="w/parent", mergeable="CONFLICTING", sha="kid1")
  conductor.prs[8] = kid
  conductor.tips["w/parent"] = "old"
  conductor.shift()
  await conductor.mend(kid, 0.0, False)
  assert "git rebase --onto origin/w/parent old w/kid" in conductor.hired[0][1]


@POSIX
@JQ
async def test_pr_gives_the_url_once_and_takes_labels_as_a_list_and_part_of(conductor: Round) -> None:
  branch(conductor)
  conductor.say(r"pr create", "https://github.com/o/r/pull/9\n")
  got = await conductor.ns["pr"](
    "feat(tui): Show records",
    "It matters.",
    ["It works."],
    CHECKS,
    labels=["w:k", "thread:t"],
    part_of=394,
    where=str(conductor.tmp),
  )
  assert got.code == 0
  assert got.stdout.content.count("https://github.com/o/r/pull/9") == 1
  assert "--head w/k" in conductor.log()
  assert "--label w:k --label thread:t" in conductor.log()


@POSIX
@JQ
async def test_a_mergeable_that_gh_gives_as_unknown_is_probed_with_git_merge_tree(
  conductor: Round, tmp_path: Path
) -> None:
  shas = stacked(conductor, tmp_path, clash=True)
  kid = conductor.pr(n=8, head="w/kid", mergeable="UNKNOWN", sha=shas["w/kid"])
  await conductor.probe(kid)
  assert kid["mergeable"] == "CONFLICTING"


@POSIX
@JQ
async def test_a_probe_leaves_a_clean_pr_as_unknown(conductor: Round, tmp_path: Path) -> None:
  shas = stacked(conductor, tmp_path)
  kid = conductor.pr(n=8, head="w/kid", mergeable="UNKNOWN", sha=shas["w/kid"])
  await conductor.probe(kid)
  assert kid["mergeable"] == "UNKNOWN"


def push_parent(conductor: Round, tmp_path: Path, force: bool) -> tuple[str, str]:
  env = conductor.env
  git("checkout", "-q", "w/parent", cwd=tmp_path, env=env)
  old = git("rev-parse", "HEAD", cwd=tmp_path, env=env).strip()
  if force:
    git("commit", "-q", "--amend", "-m", "parent2", cwd=tmp_path, env=env)
    git("push", "-q", "--force", "origin", "w/parent", cwd=tmp_path, env=env)
  else:
    (tmp_path / "more.txt").write_text("more\n", encoding="utf-8")
    git("add", "more.txt", cwd=tmp_path, env=env)
    git("commit", "-q", "-m", "parent2", cwd=tmp_path, env=env)
    git("push", "-q", "origin", "w/parent", cwd=tmp_path, env=env)
  new = git("rev-parse", "HEAD", cwd=tmp_path, env=env).strip()
  git("checkout", "-q", "main", cwd=tmp_path, env=env)
  conductor.prs[7]["sha"] = new
  conductor.shifted[8] = (old, new)
  return old, new


@POSIX
@JQ
async def test_a_child_follows_the_force_push_of_its_parent(conductor: Round, tmp_path: Path) -> None:
  stacked(conductor, tmp_path)
  conductor.prs[8]["labels"] = ["w:k8"]
  conductor.busy("p1", "k8", SONNET)
  push_parent(conductor, tmp_path, True)
  assert await conductor.restack_forced() == {8}
  assert history(tmp_path, conductor, "w/kid") == ["kid", "parent2", "base"]
  assert history(tmp_path, conductor, "w/deep") == ["deep", "kid", "parent2", "base"]
  assert 8 not in conductor.shifted
  assert conductor.prompts[0][0].startswith("The round rebased your PR #8 onto w/parent and pushed it.")
  assert await conductor.restack_forced() == set()


@POSIX
@JQ
async def test_a_child_stays_when_its_parent_only_gains_a_commit(conductor: Round, tmp_path: Path) -> None:
  stacked(conductor, tmp_path)
  push_parent(conductor, tmp_path, False)
  assert await conductor.restack_forced() == set()
  assert history(tmp_path, conductor, "w/kid") == ["kid", "parent", "base"]


def test_each_brief_holds_the_recipes_that_fixers_found(conductor: Round) -> None:
  for word in ("credential.helper", "actions/jobs", "sed -i ''", "timeout=1800", "hypothesis", "git stash", "cannot lock ref"):
    assert word in conductor.WORKER
    assert word in conductor.FIX
  assert "{workspace}" in conductor.WORKER and "Leave the workspace clean" in conductor.WORKSPACE


@POSIX
@JQ
async def test_pr_gives_the_url_of_a_pr_that_stands_when_gh_fails_late(conductor: Round) -> None:
  branch(conductor)
  conductor.say(r"pr create", "", 1)
  conductor.say(r"pr list --state open --head w/k --json url", '[{"url": "https://github.com/o/r/pull/9"}]')
  got = await conductor.ns["pr"](
    "feat(tui): Show records", "It matters.", ["It works."], CHECKS, where=str(conductor.tmp)
  )
  assert got.code == 0
  assert got.stdout.content.strip().endswith("https://github.com/o/r/pull/9")


def test_prcheck_gives_the_words_and_names_the_part_that_is_too_long(conductor: Round) -> None:
  check = conductor.ns["prcheck"]
  assert check("feat(tui): Show records", "one two", ["three"], ["four"]) == len("one two three four".split())
  with pytest.raises(Refused, match=r"301 words, over 300 \(why 300, outcome 1, breaking 0\)"):
    check("feat(tui): Show records", "w " * 300, ["x"])
  with pytest.raises(Refused, match="70 characters"):
    check("feat(tui): " + "A" * 70, "why", ["x"])



@POSIX
@JQ
def test_an_open_issue_whose_worker_ended_with_a_pr_is_queued_again_once_for_each_end(conductor: Round) -> None:
  owned = {"number": 394, "title": "T", "body": "b", "author": {"login": "uael"}, "assignees": [], "labels": [], "url": "u"}
  conductor.say(r"issue list", json.dumps([owned]))
  commands: list[str] = []

  def quiet(verb: object, *args: object, **words: object) -> object:
    got = verb(*args, **words)  # type: ignore[operator]
    if asyncio.iscoroutine(got):
      commands.append(str(args[0]))
      got.close()
    return got

  conductor.ns["quietly"] = quiet

  def ends(name: str, pr: int) -> None:
    conductor.working[name] = "issue394"
    conductor.done[name] = (pr, "report")
    conductor.tend()

  asyncio.run(conductor.take_issues())
  assert [key for key, _, _ in conductor.queue] == ["issue394"]
  assert "Their PRs" not in conductor.queue[0][1]
  conductor.queue.clear()
  ends("prompt1", 483)
  conductor.prs[500] = conductor.pr(500, labels=["w:issue394"])
  asyncio.run(conductor.take_issues())
  assert conductor.queue == []
  conductor.prs.clear()
  conductor.backlog.append(("issue394", "", ""))
  asyncio.run(conductor.take_issues())
  assert conductor.queue == []
  conductor.backlog.clear()
  conductor.working["prompt9"] = "issue394"
  asyncio.run(conductor.take_issues())
  assert conductor.queue == []
  del conductor.working["prompt9"]
  asyncio.run(conductor.take_issues())
  assert [key for key, _, _ in conductor.queue] == ["issue394"]
  assert "Their PRs: #483." in conductor.queue[0][1]
  assert "queued again as the item issue394" in conductor.heard[-1]
  assert "Taken again:" in commands[-1] and "Taken:" in commands[0]
  conductor.queue.clear()
  asyncio.run(conductor.take_issues())
  assert conductor.queue == []
  ends("prompt2", 440)
  ends("prompt3", 0)
  asyncio.run(conductor.take_issues())
  assert "Their PRs: #483, #440." in conductor.queue[0][1]
  conductor.queue.clear()
  ends("prompt4", 0)
  asyncio.run(conductor.take_issues())
  assert conductor.queue == []


@POSIX
@JQ
def test_an_issue_that_starts_with_a_mark_is_queued_once_the_owner_puts_a_thumb_on_it(conductor: Round) -> None:
  marked = {"number": 574, "title": "T", "body": "🤖 Idea", "author": {"login": "uael"}, "assignees": [], "labels": [], "url": "u"}
  conductor.say(r"issue list", json.dumps([marked]))
  conductor.say(r"issues/574/reactions", json.dumps([{"content": "heart", "user": {"login": "uael"}}, {"content": "+1", "user": {"login": "x"}}]))
  asyncio.run(conductor.take_issues())
  assert conductor.queue == [] and "issue574" not in conductor.handled
  conductor.says.pop()
  conductor.say(r"issues/574/reactions", json.dumps([{"content": "+1", "user": {"login": "uael"}}]))
  asyncio.run(conductor.take_issues())
  assert [key for key, _, _ in conductor.queue] == ["issue574"]
  assert "issue574" in conductor.handled


def named_issue(body: str = "b") -> dict:
  return {"number": 394, "title": "T", "body": body, "author": {"login": "uael"}, "assignees": [], "labels": [], "url": "u"}


@POSIX
@JQ
def test_an_issue_brief_names_the_prs_that_name_the_issue_and_marks_a_cut_body(conductor: Round) -> None:
  conductor.say(r"issue list", json.dumps([named_issue("x" * 6100)]))
  merged = [
    {"number": 575, "title": "feat(x): Add y", "state": "MERGED", "body": "🤖 Part of #394."},
    {"number": 9, "title": "other", "state": "MERGED", "body": "It names #3940 only."},
  ]
  conductor.say(r"pr list --state all", json.dumps(merged))
  asyncio.run(conductor.take_issues())
  [(key, brief, _)] = conductor.queue
  assert key == "issue394" and "  #575 MERGED feat(x): Add y" in brief and "#9 MERGED" not in brief
  assert "The body is cut here, at 6000 of 6100 characters" in brief and "--json body --jq .body" in brief
  assert "gh issue view 394 --json title,body,comments" in brief and 'git log origin/main --grep "#394"' in brief


@POSIX
@JQ
def test_an_issue_that_an_open_pr_names_is_not_queued_and_is_looked_at_again(conductor: Round) -> None:
  conductor.say(r"issue list", json.dumps([named_issue()]))
  conductor.say(r"pr list --state all", json.dumps([{"number": 570, "title": "T", "state": "OPEN", "body": "Fixes #394"}]))
  asyncio.run(conductor.take_issues())
  assert conductor.queue == [] and "issue394" not in conductor.handled
  conductor.says.pop()
  conductor.say(r"pr list --state all", json.dumps([{"number": 570, "title": "T", "state": "MERGED", "body": "Fixes #394"}]))
  asyncio.run(conductor.take_issues())
  assert [key for key, _, _ in conductor.queue] == ["issue394"]


def says_comment(conductor: Round, labels: list[str], body: str = "Fix this.") -> None:
  conductor.prs[7] = conductor.pr(n=7, labels=labels)
  conductor.say(r"pr list", json.dumps([{"number": 7}]))
  comment = {
    "pull_request_url": "u/pulls/7",
    "id": 55,
    "user": {"login": "uael"},
    "reactions": {"+1": 0},
    "path": "a.py",
    "line": 3,
    "body": body,
  }
  conductor.say(r"pulls/7/comments", json.dumps([comment]))


@POSIX
@JQ
def test_a_comment_on_a_pr_with_hold_owner_review_goes_to_its_live_maker_and_adds_no_label_hold(
  conductor: Round,
) -> None:
  says_comment(conductor, ["w:k7", "hold:owner-review"])
  conductor.busy("p1", "k7", SONNET)
  asyncio.run(conductor.review(False))
  [(message, on)] = conductor.prompts
  assert on == "chain-k7" and "hold:owner-review" in message and "Fix this." in message
  assert "pr edit 7 --add-label hold" not in conductor.log()


@POSIX
@JQ
def test_a_comment_on_a_pr_with_hold_owner_review_goes_to_the_chain_of_an_ended_maker_on_its_model(
  conductor: Round,
) -> None:
  says_comment(conductor, ["w:k7", "hold:owner-review"])
  conductor.homes["k7"] = "chain7"
  conductor.ends["k7"] = [("p9", 7)]
  conductor.models["p9"] = SONNET_HIGH
  asyncio.run(conductor.review(False))
  assert conductor.rungs[0] == (f"actor = {SONNET_HIGH!r}", "chain7")
  [(message, on)] = conductor.prompts
  assert on == "chain7" and message.startswith(f"Your work on this chain goes on, now on {SONNET_HIGH}.")
  assert "Fix this." in message and "hold:owner-review" in message
  assert "pr edit 7 --add-label hold" not in conductor.log()


@POSIX
@JQ
def test_a_comment_on_a_pr_without_the_label_still_gets_the_label_hold(conductor: Round) -> None:
  says_comment(conductor, ["w:k7"])
  conductor.busy("p1", "k7", SONNET)
  asyncio.run(conductor.review(False))
  assert "pr edit 7 --add-label hold\n" in conductor.log()


NOT_FOUND = '{"message":"Not Found","documentation_url":"https://docs.github.com/rest","status":"404"}'


def says_issues(conductor: Round, bodies: dict[int, str]) -> None:
  conductor.say(r"pr list", json.dumps([{"number": 7}]))
  conductor.say(r"issue list", json.dumps([{"number": n} for n in bodies]))
  conductor.say(r"pulls/39\d/", NOT_FOUND, 1)
  for n, body in bodies.items():
    comment = {"issue_url": f"u/issues/{n}", "id": n, "user": {"login": "uael"}, "reactions": {"+1": 0}, "body": body}
    conductor.say(rf"issues/{n}/comments", json.dumps([comment]))


@POSIX
@JQ
def test_a_404_body_of_gh_on_an_issue_loses_no_comment_of_the_next_number(conductor: Round) -> None:
  says_issues(conductor, {394: "first", 395: "second"})
  conductor.handled.update({"issue394", "issue395"})
  asyncio.run(conductor.review(False))
  assert len(conductor.prompts) == 1
  message = conductor.prompts[0][0]
  assert "#394 i394 - by uael: first" in message
  assert "#395 i395 - by uael: second" in message
  assert "Not Found" not in message
  assert "pulls/394" not in conductor.log() and "pulls/7/" in conductor.log()


@POSIX
@JQ
def test_each_row_of_the_reviews_ends_its_line_and_a_body_holds_no_carriage_return(conductor: Round) -> None:
  says_issues(conductor, {394: "a\r\nb\tc\rd"})
  conductor.say(r"pulls/7/reviews", NOT_FOUND, 1)
  got = asyncio.run(conductor.bash(conductor.REVIEWS))
  rows = [x for x in got.stdout.content.split("\n") if x]
  assert rows == [NOT_FOUND, "394\ti394\tuael\t0\t-\ta  b c d"]


def failed_run(conductor: Round, run: int, jobs: dict[str, str]) -> None:
  """Say the failed jobs of a run and the log of each, as the jobs API of gh gives them."""
  listed = [{"id": run * 10 + at, "name": name, "conclusion": "failure"} for at, name in enumerate(jobs, 1)]
  conductor.say(rf"actions/runs/{run}/jobs", json.dumps({"jobs": listed}))
  for one, log in zip(listed, jobs.values()):
    conductor.say(rf"actions/jobs/{one['id']}/logs", log)


def evidence_repo(conductor: Round, tmp_path: Path) -> str:
  env = conductor.env
  git("init", "-q", "-b", "main", cwd=tmp_path, env=env)
  (tmp_path / "test").mkdir()
  wall = tmp_path / "test" / "test_hygiene.py"
  wall.write_text("WALL = 6520\n", encoding="utf-8")
  git("add", "test/test_hygiene.py", cwd=tmp_path, env=env)
  git("commit", "-q", "-m", "base", cwd=tmp_path, env=env)
  sha = git("rev-parse", "HEAD", cwd=tmp_path, env=env).strip()
  wall.write_text("WALL = 6500\n", encoding="utf-8")
  git("commit", "-q", "-am", "main sets the wall to 6500", cwd=tmp_path, env=env)
  git("update-ref", "refs/remotes/origin/main", "HEAD", cwd=tmp_path, env=env)
  git("reset", "-q", "--hard", sha, cwd=tmp_path, env=env)
  return sha


@POSIX
@JQ
async def test_an_order_for_a_failed_check_carries_its_failed_lines_and_the_commits_main_changed(
  conductor: Round, tmp_path: Path
) -> None:
  sha = evidence_repo(conductor, tmp_path)
  runs = json.dumps([{"databaseId": 9, "conclusion": "failure"}])
  conductor.say(r"limit 30 --json databaseId,conclusion", runs)
  failed_run(conductor, 9, {"gates": "FAILED test/test_hygiene.py::test_wall - 6520 > 6500\nnoise\n"})
  conductor.busy("p1", "k5", SONNET)
  await conductor.mend(conductor.pr(checks=("FAILURE",), sha=sha, labels=["w:k5"]), 0.0, False)
  order = conductor.prompts[-1][0]
  assert "The first failed lines of its job log:\n  gates\tFAILED test/test_hygiene.py::test_wall - 6520 > 6500\n" in order
  assert "that change a file of the failure:\n  " in order and "main sets the wall to 6500" in order
  assert "noise" not in order


@POSIX
@JQ
async def test_a_fixer_brief_for_a_failed_check_carries_the_same_lines(conductor: Round, tmp_path: Path) -> None:
  sha = evidence_repo(conductor, tmp_path)
  conductor.say(r"limit 30 --json databaseId,conclusion", json.dumps([{"databaseId": 9, "conclusion": "failure"}]))
  failed_run(conductor, 9, {"gates": "FAILED test/test_hygiene.py::test_wall\n"})
  await conductor.mend(conductor.pr(checks=("FAILURE",), sha=sha), 0.0, False)
  assert [key for key, _ in conductor.hired] == ["fix5"]
  assert "FAILED test/test_hygiene.py::test_wall" in conductor.hired[0][1]
  assert "main sets the wall to 6500" in conductor.hired[0][1]


def squashed_repo(conductor: Round, tmp_path: Path, same: bool) -> tuple[str, str]:
  env, up = conductor.env, tmp_path / "up"
  up.mkdir()

  def commit(name: str, text: str, message: str) -> str:
    (up / name).write_text(text, encoding="utf-8")
    git("add", name, cwd=up, env=env)
    git("commit", "-q", "-m", message, cwd=up, env=env)
    return git("rev-parse", "HEAD", cwd=up, env=env).strip()

  git("init", "-q", "-b", "main", cwd=up, env=env)
  commit("f.txt", "a\n", "base")
  git("switch", "-q", "-c", "w/x", cwd=up, env=env)
  held = commit("g.txt", "g\n", "pre-squash one")
  head = commit("h.txt", "h\n", "its own change")
  git("update-ref", "refs/pull/5/head", head, cwd=up, env=env)
  git("switch", "-q", "main", cwd=up, env=env)
  commit("g.txt", "g\n" if same else "other\n", "squash of the first PR (#4)")
  git("init", "-q", "-b", "main", cwd=tmp_path, env=env)
  git("remote", "add", "origin", str(up), cwd=tmp_path, env=env)
  return head, held


@POSIX
async def test_an_order_for_a_conflict_names_the_commits_whose_patch_main_holds_already(
  conductor: Round, tmp_path: Path
) -> None:
  head, held = squashed_repo(conductor, tmp_path, True)
  note = await conductor.main_changes(conductor.pr(n=5, sha=head, head="w/x"))
  assert "This is no stack. The branch of this PR holds 1 commit whose patch main holds already" in note
  assert f"  {held[:7]} pre-squash one\n" in note
  assert f"Fix: git rebase --onto origin/main {held[:12]} w/x replays only the commits after them." in note
  assert "its own change" not in note and "squash of the first PR (#4)" in note


@POSIX
async def test_a_branch_whose_patches_main_lacks_gets_no_note_of_a_squash(conductor: Round, tmp_path: Path) -> None:
  head, _ = squashed_repo(conductor, tmp_path, False)
  note = await conductor.main_changes(conductor.pr(n=5, sha=head, head="w/x"))
  assert "no stack" not in note and "squash of the first PR (#4)" in note


def test_the_ledger_of_the_tooling_keeps_one_form_of_when_and_moves_the_old_entries_once(conductor: Round) -> None:
  import calendar

  conductor.now = 1793181153.0
  conductor.files[conductor.TOOLING] = json.dumps([{"when": "10-23", "what": "a wall", "state": "open", "count": 1}])
  conductor.flaw("a new friction")
  whens = [x["when"] for x in json.loads(conductor.files[conductor.TOOLING])]
  assert whens == [str(calendar.timegm((2026, 10, 23, 0, 0, 0))), "1793181153"]


def test_the_ledger_gives_each_entry_a_stable_id_and_one_closed_state(conductor: Round) -> None:
  conductor.files[conductor.TOOLING] = json.dumps(
    [
      {"when": "1", "what": "a", "state": "in-pr #497", "count": 1},
      {"when": "1", "what": "b", "state": "open: #364", "count": 1},
      {"when": "1", "what": "c", "state": "queued", "count": 1},
    ]
  )
  conductor.flaw("d")
  held = json.loads(conductor.files[conductor.TOOLING])
  assert [(x["id"], x["state"]) for x in held] == [("e0", "closed"), ("e1", "open"), ("e2", "queued"), ("e3", "open")]
  assert held[0]["note"] == "in-pr #497"
  assert conductor.ledger_text("e2", "e3") == "e2 [queued] 1x c\ne3 [open] 1x d"


def test_a_flaw_that_a_merged_pr_fixed_is_closed_with_that_pr_and_opens_again_when_it_comes_again(
  conductor: Round,
) -> None:
  conductor.flaw("The PATH of the World lacks /opt/homebrew/bin", "A second friction")
  assert conductor.settle_flaw("path of the world", 707) == 1
  assert conductor.settle_flaw("path of the world", 707) == 0
  held = {x["what"]: x for x in json.loads(conductor.files[conductor.TOOLING])}
  assert (held["The PATH of the World lacks /opt/homebrew/bin"]["state"], held["The PATH of the World lacks /opt/homebrew/bin"]["closed_by"]) == ("closed", "#707")
  assert held["A second friction"]["state"] == "open" and "closed_by" not in held["A second friction"]
  conductor.flaw("The PATH of the World lacks /opt/homebrew/bin")
  assert json.loads(conductor.files[conductor.TOOLING])[0]["state"] == "open"


def test_an_item_that_names_its_pr_owns_it_before_a_label_or_a_push_does(conductor: Round) -> None:
  conductor.busy("p1", "tui-stack", OPUS)
  one = conductor.pr(n=606, checks=("FAILURE",))
  assert conductor.owner_of(one) == ""
  conductor.owns["tui-stack"] = [606]
  assert conductor.owner_of(one) == "p1"


async def flaked(conductor: Round) -> None:
  conductor.says.clear()
  conductor.say(r"workflows/gates\.yml/runs\?", '{"workflow_runs": []}')
  await conductor.watch_ci()
  conductor.says.clear()
  url = "https://github.com/o/r/actions/runs/50"
  flaky = [{"id": 50, "head_sha": "s1", "run_attempt": 2, "conclusion": "success", "html_url": url}]
  job = {"id": 7, "name": "the TUI", "conclusion": "failure", "started_at": "2026-10-23T10:00:00Z", "completed_at": "2026-10-23T10:01:00Z"}
  conductor.say(r"runs\?branch=main&status=success", '{"workflow_runs": []}')
  conductor.say(r"runs\?per_page=30", json.dumps({"workflow_runs": flaky}))
  conductor.say(r"runs/50/attempts/1/jobs", json.dumps({"jobs": [job]}))
  conductor.say(r"jobs/7/logs", "2026-10-23T10:00:30.0000000Z (fail) a host test [30000.17ms]\n")
  await conductor.watch_ci()


@POSIX
@JQ
async def test_a_flake_that_a_queued_item_names_queues_no_second_item(conductor: Round) -> None:
  conductor.queue.append(("flake-host-about", "the race of a host test", "ci"))
  await flaked(conductor)
  assert [key for key, _, _ in conductor.queue] == ["flake-host-about"]
  assert any("flake-host-about" in x for x in conductor.heard)


@POSIX
@JQ
async def test_a_flake_that_a_working_item_names_goes_to_its_worker_in_place_of_a_new_item(conductor: Round) -> None:
  conductor.busy("p1", "host-about", OPUS)
  conductor.briefed["host-about"] = "the race of a host test"
  await flaked(conductor)
  assert conductor.queue == []
  assert conductor.prompts[-1][1] == "chain-host-about" and "Your item came again" in conductor.prompts[-1][0]


def test_a_round_with_debriefs_off_asks_no_debrief_of_an_ended_worker(conductor: Round) -> None:
  conductor.ns["DEBRIEFS"] = False
  conductor.working["prompt1"] = "item1"
  conductor.facts["prompt1"] = ("prompt", "prompt1", "rung1", "chain5", "tuple[int, str]", "brief", "")
  conductor.done["prompt1"] = (12, "report")
  conductor.tend()
  assert conductor.heard == ["The worker item1 (prompt1) ended with PR #12, and results['prompt1'] keeps its report."]
  assert conductor.prompts == []


@POSIX
async def test_pr_creates_each_label_that_it_adds_and_raises_refused_with_what_gh_said(
  conductor: Round, tmp_path: Path
) -> None:
  open_pr = conductor.ns["pr"]
  boxes = ["checked"] * 6
  conductor.say(r"pr create", "could not add label: not found", 1)
  with pytest.raises(Refused) as caught:
    await open_pr("feat(x): Y", "why", ["out"], boxes, labels=["w:k", "thread:t"], where=str(tmp_path))
  assert "not found" in str(caught.value)
  assert "label create w:k --force" in conductor.log() and "label create thread:t --force" in conductor.log()
  conductor.says.clear()
  conductor.say(r"pr create", "https://github.com/o/r/pull/9\n")
  got = await open_pr("feat(x): Y", "why", ["out"], boxes, labels=" --label w:k", where=str(tmp_path))
  assert got.stdout.content.strip() == "https://github.com/o/r/pull/9"


def bashing(conductor: Round, *acts: tuple[str, str]) -> None:
  conductor.busy("p1", "k1", SONNET)
  facts = [("bash", name, "rung1", "chain-k1", command, "", 600) for name, command in acts]
  conductor.ns["transcript"] = lambda on="", of="": [*facts] if on == "chain-k1" else []
  for name, _ in acts:
    conductor.files[f"{name}/stdout"] = ""
    conductor.files[f"{name}/stderr"] = ""


def test_a_watch_on_ci_that_runs_past_ten_minutes_is_cancelled_with_a_reason_that_keeps_the_worker_on_its_pr(
  conductor: Round,
) -> None:
  bashing(conductor, ("bash7", "gh pr checks 5 --watch"))
  conductor.tend()
  conductor.now += 540
  conductor.tend()
  assert conductor.cancelled == []
  conductor.now += 120
  conductor.tend()
  assert conductor.cancelled == ["bash7"]
  said = conductor.prompts[-1]
  assert said[1] == "chain-k1" and "wait on CI" in said[0] and "await checked(number)" in said[0]
  assert "close only when your PR has merged" in said[0]
  assert any("bash7" in x and "wait on CI" in x for x in conductor.heard)


def test_a_loop_of_sleep_around_gh_counts_as_a_wait_on_ci(conductor: Round) -> None:
  bashing(conductor, ("bash7", "while true; do gh run view 9; sleep 60; done"))
  conductor.tend()
  conductor.now += 700
  conductor.tend()
  assert conductor.cancelled == ["bash7"]


def test_a_command_that_prints_keeps_running_whatever_its_age(conductor: Round) -> None:
  bashing(conductor, ("bash8", "cargo test --workspace"))
  for _step in range(8):
    conductor.files["bash8/stdout"] += "line\n"
    conductor.tend()
    conductor.now += 600
  assert conductor.cancelled == [] and conductor.heard == []
  assert not [x for x in conductor.prompts if "bash8" in x[0]]


def test_a_silent_command_gets_one_line_at_fifteen_minutes_and_a_cancel_at_thirty(conductor: Round) -> None:
  bashing(conductor, ("bash9", "git commit -m x"))
  conductor.files["bash9/stdout"] = "hooks start\n"
  conductor.tend()
  conductor.now += 900
  conductor.tend()
  conductor.now += 300
  conductor.tend()
  lines = [x for x in conductor.heard if "bash9" in x]
  assert len(lines) == 1 and "15 minutes" in lines[0] and "hooks start" in lines[0]
  assert conductor.cancelled == []
  conductor.now += 600
  conductor.tend()
  assert conductor.cancelled == ["bash9"]
  assert "30 minutes" in conductor.prompts[-1][0] and "bash9" in conductor.prompts[-1][0]
  assert conductor.watched == {"bash9": [1000.0, 12, 1000.0]}


def test_a_pr_whose_maker_ended_gets_the_home_that_pushed_its_head(conductor: Round) -> None:
  one = conductor.pr(n=5, head="w/k9")
  conductor.prs[5] = one
  pushes = [("bash", "bash3", "rung1", "chain-k9", "git push origin HEAD:refs/heads/w/k9", "", 600)]
  chains = [("chain", "chain-k9", "rung1", "chain1", "defects/k9", "")]
  conductor.ns["transcript"] = lambda on="", of="": chains if on == "chain1" else pushes if on == "chain-k9" else []
  conductor.ns["quietly"] = lambda verb, *args, **words: asyncio.run(verb(*args, **words))
  conductor.invariants()
  assert conductor.homes == {"k9": "chain-k9"} and "w:k9" in one["labels"]
  assert "pr edit 5 --add-label w:k9" in conductor.log()
  assert any("#5" in x and "k9" in x for x in conductor.heard)


def test_a_pr_that_no_chain_pushed_is_listed_once_in_the_next_notice(conductor: Round) -> None:
  conductor.prs[5] = conductor.pr(n=5)
  conductor.invariants()
  conductor.invariants()
  assert [x for x in conductor.heard if "has no home" in x] == [
    "#5 (w/k5) has no home: no worker owns it, and no chain pushed its head."
  ]


def test_an_item_with_an_unmerged_after_stays_in_the_backlog_and_starts_once_it_merged(conductor: Round) -> None:
  conductor.backlog.append(("k2", "brief", "defects"))
  conductor.after["k2"] = ["k1"]
  conductor.ends["k1"] = [("worker1", 7)]
  conductor.staff()
  assert [x[0] for x in conductor.backlog] == ["k2"] and conductor.hired == [] and conductor.queue == []
  conductor.merged.add(7)
  conductor.staff()
  assert [key for key, _ in conductor.hired] == ["k2"] and conductor.backlog == []


@JQ
def test_a_change_of_main_that_is_no_merge_of_a_pr_is_told_once_with_its_before_and_after(conductor: Round) -> None:
  events = [
    {"id": 3, "activity_type": "force_push", "before": "a" * 40, "after": "b" * 40, "timestamp": "T3", "actor": {"login": "u"}},
    {"id": 2, "activity_type": "pr_merge", "before": "c" * 40, "after": "a" * 40, "timestamp": "T2", "actor": {"login": "u"}},
    {"id": 1, "activity_type": "push", "before": "d" * 40, "after": "c" * 40, "timestamp": "T1", "actor": {"login": "w"}},
  ]
  conductor.say(r"activity\?ref=refs/heads/main", json.dumps(events))
  asyncio.run(conductor.main_pushes())
  asyncio.run(conductor.main_pushes())
  [(told, _)] = conductor.prompts
  assert told.startswith(
    "main changed with no merge of a PR, which only the merge of a PR may do:\n"
    "- force_push by u at T3: aaaaaaaa..bbbbbbbb\n- push by w at T1: dddddddd..cccccccc\n"
  )


@JQ
def test_a_main_that_changed_only_by_merges_or_an_activity_that_fails_tells_nothing(conductor: Round) -> None:
  merge = {"id": 2, "activity_type": "pr_merge", "before": "c", "after": "a", "timestamp": "T", "actor": {"login": "u"}}
  conductor.say(r"activity\?ref=refs/heads/main", json.dumps([merge]))
  asyncio.run(conductor.main_pushes())
  conductor.says.clear()
  conductor.say(r"activity\?ref=refs/heads/main", "boom", 1)
  asyncio.run(conductor.main_pushes())
  assert conductor.prompts == []


def test_an_item_that_names_a_file_of_an_open_pr_waits_for_that_pr_and_starts_once_it_closed(conductor: Round) -> None:
  conductor.prs[7] = conductor.pr(n=7, labels=["w:reader-cache"])
  conductor.touched[7] = {"sha": "abc", "paths": ["src/world/reader.rs", "docs/x.md"]}
  conductor.queue.append(("reader-stream", "Make src/world/reader.rs stream its facts.", "defects"))
  conductor.staff()
  conductor.staff()
  assert conductor.hired == [] and [x[0] for x in conductor.queue] == ["reader-stream"]
  assert conductor.serial == {"reader-stream": 7}
  assert [x for x in conductor.heard if "#7" in x] == [
    "reader-stream waits for #7, which changes src/world/reader.rs that it names, so two designs of one file do not race."
  ]
  del conductor.prs[7]
  conductor.staff()
  assert [key for key, _ in conductor.hired] == ["reader-stream"] and conductor.serial == {}


def test_an_item_waits_for_a_pr_that_changes_a_folder_of_its_areas_or_its_absolute_path(conductor: Round) -> None:
  conductor.prs[7] = conductor.pr(n=7)
  conductor.touched[7] = {"sha": "abc", "paths": ["src/extension/mcp/config.rs"]}
  conductor.areas["k1"] = ["src/extension/mcp"]
  conductor.areas["k3"] = ["src"]
  conductor.queue.extend([("k1", "brief", "defects"), ("k2", "Read ~/repo/src/extension/mcp/config.rs.", "defects")])
  conductor.queue.append(("k3", "Read AGENTS.md and src/other.rs.", "defects"))
  conductor.staff()
  assert [key for key, _ in conductor.hired] == ["k3"] and conductor.serial == {"k1": 7, "k2": 7}


def test_an_item_never_waits_for_the_pr_that_it_names_labels_or_owns(conductor: Round) -> None:
  for n in (7, 8, 9):
    conductor.prs[n] = conductor.pr(n=n, labels=["w:k2"] if n == 8 else [])
    conductor.touched[n] = {"sha": "abc", "paths": ["src/life.rs"]}
  conductor.owns["k3"] = [9]
  briefs = {"k1": "Fix src/life.rs on #7, #8 and #9.", "k2": "Fix src/life.rs on #7 and #9.", "k3": "Fix src/life.rs on #7 and #8."}
  assert [conductor.serialized(key, brief) for key, brief in briefs.items()] == [False, False, False]
  assert conductor.serialized("k4", "Fix src/life.rs on #7.") and conductor.serial == {"k4": 8}


@JQ
def test_a_sweep_reads_the_files_of_each_open_pr_of_the_round_once_for_each_head(conductor: Round) -> None:
  conductor.say(r"pr view 7 --json files", json.dumps({"files": [{"path": "a/b.py"}, {"path": "c.md"}]}))
  conductor.prs[7] = conductor.pr(n=7)
  conductor.prs[8] = conductor.pr(n=8, head="other")
  conductor.touched[6] = {"sha": "old", "paths": ["x/y.py"]}
  asyncio.run(conductor.changed_files())
  asyncio.run(conductor.changed_files())
  assert conductor.touched == {7: {"sha": "abc", "paths": ["a/b.py", "c.md"]}}
  assert conductor.log().count("pr view 7 --json files") == 1 and "pr view 8" not in conductor.log()


def test_the_items_held_by_the_coordinator_move_into_the_backlog_with_their_keys(conductor: Round) -> None:
  item = {"key": "k3", "model": "claude-cli:sonnet/high", "after": ["k1"], "brief": "do it"}
  conductor.ns["held_items"] = {"k3": item}
  conductor.ns["RULING"] = "Ruling. "
  conductor.invariants()
  assert conductor.ns["held_items"] == {}
  assert conductor.backlog == [("k3", "Ruling. do it", "defects")]
  assert conductor.after == {"k3": ["k1"]} and conductor.efforts["k3"] == "claude-cli:sonnet/high"
  assert any("held_items" in x and "k3" in x for x in conductor.heard)
  assert any("k1" in x and "k3" in x for x in conductor.heard)


def cancelled_worker(conductor: Round) -> None:
  conductor.working["prompt1"] = "item1"
  conductor.models["prompt1"] = SONNET
  conductor.facts["prompt1"] = ("prompt", "prompt1", "rung1", "chain5", "tuple[int, str]", "brief", "")
  conductor.done["prompt1"] = asyncio.CancelledError()


def test_a_prompt_that_ended_with_cancelled_error_and_no_cancel_of_the_round_goes_on_and_tells_nobody(
  conductor: Round,
) -> None:
  cancelled_worker(conductor)
  conductor.tend()
  [(told, on)] = conductor.prompts
  assert told.startswith(conductor.RESUME + "\n\nWork in the workspace ") and on == "chain5"
  assert conductor.targets[-1] == SONNET
  assert list(conductor.working.values()) == ["item1"] and "prompt1" not in conductor.working
  assert conductor.models[next(iter(conductor.working))] == SONNET
  assert conductor.heard == [] and conductor.results == {} and conductor.ends == {}


def test_a_prompt_with_a_cancel_over_it_ends_as_before(conductor: Round) -> None:
  cancelled_worker(conductor)
  conductor.ns["transcript"] = lambda on="", of="": [("cancel", "prompt1", "chain1")] if of == "cancel" else []
  conductor.tend()
  assert conductor.working == {} and "ended with no PR" in conductor.prompts[-1][0]


def test_a_prompt_that_ended_cancelled_three_times_ends_as_before(conductor: Round) -> None:
  cancelled_worker(conductor)
  conductor.noticed.update({"resume item1 a", "resume item1 b", "resume item1 c"})
  conductor.tend()
  assert conductor.working == {} and "ended with no PR" in conductor.prompts[-1][0]


def crowd(conductor: Round, n: int) -> dict:
  facts = [("chain", f"chain{i}", "rung1", "chain1", f"w/k{i}", "") for i in range(2, n + 2)]
  asks = {f"chain{i}": [("prompt", f"p{i}", "rung1", f"chain{i}", "tuple[int, str]", "b", "")] for i in range(2, n + 2)}
  conductor.facts.update({f[1]: f for f in facts})
  conductor.ns["transcript"] = lambda on="", of="": [] if of else facts if on == "chain1" else asks.get(on, [])
  return asks


def test_each_chain_with_an_open_prompt_counts_against_the_cap(conductor: Round) -> None:
  asks = crowd(conductor, conductor.cap())
  assert conductor.open_chains() == [f"chain{i}" for i in range(2, 7)]
  assert not conductor.room(SONNET)
  del asks["chain2"]
  assert conductor.room(SONNET)


def test_a_resume_over_the_cap_waits_for_a_slot_and_goes_on_when_one_frees(conductor: Round) -> None:
  cancelled_worker(conductor)
  asks = crowd(conductor, conductor.cap())
  conductor.tend()
  assert conductor.prompts == [] and conductor.working == {} and conductor.results == {}
  assert list(conductor.parked) == ["chain5"] and any("waits on chain5" in x for x in conductor.heard)
  del asks["chain2"]
  conductor.staff()
  [(told, on)] = conductor.prompts
  assert told.startswith(conductor.RESUME + "\n\nWork in the workspace ") and on == "chain5" and conductor.parked == {}
  assert list(conductor.working.values()) == ["item1"]


def test_the_round_over_its_cap_pauses_the_newest_chains_and_wakes_them_in_order(conductor: Round) -> None:
  asks = crowd(conductor, conductor.cap() + 2)
  conductor.working["pw"] = "k8"
  conductor.models["pw"] = SONNET
  conductor.facts["pw"] = ("prompt", "pw", "rung1", "chain8", "tuple[int, str]", "b", "")
  conductor.invariants()
  assert conductor.paused_chains == ["chain7", "chain8"] and list(conductor.parked) == ["chain7", "chain8"]
  assert conductor.prompts == [(conductor.PARKED, "chain7"), (conductor.PARKED, "chain8")]
  assert "pw" not in conductor.working and conductor.parked["chain8"]["key"] == "k8"
  del asks["chain2"]
  conductor.staff()
  assert conductor.paused_chains == ["chain8"] and list(conductor.parked) == ["chain8"]
  del asks["chain3"]
  conductor.staff()
  assert conductor.paused_chains == [] and conductor.parked == {}
  assert conductor.working == {"pw": "k8"} and conductor.models["pw"] == SONNET


def test_the_chains_that_the_live_round_parked_move_into_the_state_of_the_round(conductor: Round) -> None:
  conductor.ns["pk_parked"] = {"chain9": {"name": "", "key": "k9", "model": SONNET, "text": ""}}
  conductor.invariants()
  assert conductor.ns["pk_parked"] == {}
  assert conductor.parked == {"chain9": {"name": "", "key": "k9", "model": SONNET, "text": ""}}


def nested(conductor: Round) -> dict:
  asks = crowd(conductor, conductor.cap())
  conductor.busy("pw", "audit-a", OPUS)
  conductor.facts["pw"] = ("prompt", "pw", "rung1", "chain2", "tuple[int, str]", "b", "")
  subs = [("chain", f"chain{i}", "rung9", "chain2", f"audit {i}", "") for i in (20, 21)]
  conductor.facts.update({f[1]: f for f in subs})
  asks.update({f[1]: [("prompt", f"q{f[1]}", "rung9", f[1], "str", "b", "")] for f in subs})
  top = [("chain", f"chain{i}", "rung1", "chain1", f"w/k{i}", "") for i in range(2, conductor.cap() + 2)]
  facts = {"chain1": top, "chain2": [*subs, *asks["chain2"]]}
  conductor.ns["transcript"] = lambda on="", of="": [] if of else facts.get(on) or asks.get(on, [])
  return asks


def test_a_sub_chain_of_a_live_worker_counts_within_the_slot_of_that_worker(conductor: Round) -> None:
  nested(conductor)
  assert conductor.open_chains() == [f"chain{i}" for i in range(2, 7)]
  conductor.invariants()
  assert conductor.parked == {} and conductor.paused_chains == []
  conductor.parked["chain20"] = {"name": "", "key": "", "model": "", "text": ""}
  conductor.paused_chains.append("chain20")
  conductor.staff()
  assert conductor.parked == {} and conductor.paused_chains == []


def test_a_parked_chain_takes_a_free_slot_before_a_new_chain(conductor: Round) -> None:
  asks = crowd(conductor, conductor.cap())
  del asks["chain2"]
  assert conductor.room(SONNET)
  conductor.facts["chain9"] = ("chain", "chain9", "rung1", "chain1", "w/k9", "")
  conductor.parked["chain9"] = {"name": "", "key": "k9", "model": SONNET, "text": "go on"}
  assert not conductor.room(SONNET)
  assert conductor.room(SONNET, "chain9")


def build_slots(tmp_path: Path, pid: int) -> dict[str, str]:
  script = tmp_path / "build.sh"
  script.write_text(SOURCE_BUILD(), encoding="utf-8")
  slots = tmp_path / "slots"
  for i in (1, 2, 3):
    (slots / str(i)).mkdir(parents=True)
    (slots / str(i) / "pid").write_text(str(pid), encoding="utf-8")
    (slots / str(i) / "since").write_text(str(int(time.time()) - 90), encoding="utf-8")
  return {**os.environ, "FURB_BUILD_SLOTS": str(slots), "FURB_BUILD_NOTE": "1", "SCRIPT": str(script)}


def SOURCE_BUILD() -> str:
  start = SOURCE.index('BUILD_SH = """') + len('BUILD_SH = """')
  return SOURCE[start : SOURCE.index('"""', start)]


@POSIX
def test_build_sh_says_who_holds_the_slots_while_it_waits(tmp_path: Path) -> None:
  holder = subprocess.Popen(["sleep", "30"])
  out = tmp_path / "out.txt"
  try:
    env = build_slots(tmp_path, holder.pid)
    with out.open("w", encoding="utf-8") as f:
      waiting = subprocess.Popen(["sh", env["SCRIPT"], "echo", "ran"], env=env, stdout=f, stderr=subprocess.STDOUT)
      for _ in range(100):
        time.sleep(0.1)
        if "slot 3: pid" in out.read_text(encoding="utf-8"):
          break
      waiting.terminate()
      waiting.wait()
  finally:
    holder.kill()
    holder.wait()
  said = out.read_text(encoding="utf-8")
  assert "waited" in said
  assert f"slot 1: pid {holder.pid}, held " in said
  assert "sleep 30" in said
  assert "ran" not in said.replace("waited", "")


@POSIX
def test_build_sh_frees_a_slot_whose_pid_is_gone(tmp_path: Path) -> None:
  gone = subprocess.Popen(["true"])
  gone.wait()
  env = build_slots(tmp_path, gone.pid)
  got = subprocess.run(["sh", env["SCRIPT"], "echo", "ran"], env=env, capture_output=True, text=True, timeout=20)
  assert got.returncode == 0
  assert got.stdout.strip() == "ran"


async def test_a_failed_check_that_waits_on_a_decision_of_the_owner_gets_no_order_until_it_ends(conductor: Round) -> None:
  conductor.facts["prompt9"] = ("prompt", "prompt9", "rung1", "chain1", "str", "the wall?", "operator")
  conductor.awaited["prompt9"] = ["w:wall", "#6"]
  pr, other = conductor.pr(5, ("FAILURE",), labels=["w:wall"]), conductor.pr(6, ("FAILURE",))
  conductor.prs.update({5: pr, 6: other})
  assert await conductor.mend(pr, 1000.0, False) == "owner"
  assert await conductor.mend(other, 1000.0, False) == "owner"
  assert (conductor.hired, conductor.prompts) == ([], [])
  assert len(conductor.heard) == 1 and "prompt9" in conductor.heard[0] and "w:wall, #6" in conductor.heard[0]
  assert conductor.table(conductor.pr(5, ("FAILURE",), labels=["w:wall"], mergeable="CONFLICTING")) == "conflicting"
  conductor.done["prompt9"] = "ruled"
  conductor.invariants()
  assert conductor.awaited == {} and conductor.table(pr) == "red"
  assert "The decision prompt9 of the owner ended, so the round tends w:wall, #6 again." in conductor.heard


@POSIX
@JQ
async def test_a_flake_that_a_working_item_covers_goes_to_its_worker_by_its_test_or_its_key(conductor: Round) -> None:
  conductor.busy("p1", "bench-fix", OPUS)
  conductor.covering["bench-fix"] = ["a host test", "budget-the-tui"]
  await flaked(conductor)
  assert conductor.queue == []
  assert conductor.prompts[-1][1] == "chain-bench-fix" and "came again" in conductor.prompts[-1][0]
  conductor.covering["bench-fix"] = ["flake-a-host-test"]
  assert (conductor.taken("flake-a-host-test"), conductor.taken("budget-the-tui"), conductor.taken("bench-fix")) == ("p1", "", "p1")
  conductor.working.clear()
  conductor.queue.append(("bench-fix", "", "ci"))
  assert conductor.taken("flake-a-host-test") == "queued"


def test_an_item_has_a_kind_whose_model_and_brief_it_takes(conductor: Round) -> None:
  conductor.ns["quietly"] = lambda verb, *args, **words: (
    asyncio.run(verb(*args, **words)) if verb == conductor.bash else verb(*args, **words)
  )
  conductor.kinds.update(look="audit", odd="nonsense")
  assert [conductor.kind_of(key) for key in ("look", "odd", "k", "fix5")] == ["audit", "edit", "edit", "fix"]
  assert [conductor.model_of(key) for key in ("look", "odd", "fix5")] == [OPUS, SONNET_HIGH, SONNET_HIGH]
  name = conductor.real_hire("look", "find the leaks", "defects")
  assert (conductor.models[name], conductor.targets[-1], conductor.log()) == (OPUS, OPUS, "")
  assert "scratch://audit/look.md" in conductor.prompts[-1][0] and "worktree.sh" not in conductor.prompts[-1][0]
  assert "find the leaks" in conductor.prompts[-1][0]
  conductor.ns["DEBRIEFS"] = False
  conductor.done[name] = (0, "found two")
  conductor.tend()
  assert conductor.heard == [f"The auditor look ({name}) ended, and results[{name!r}] keeps its report."]
  assert len(conductor.prompts) == 1


REGISTER = """# Register of the audit look

| Dn | key | model | area | brief |
|----|-----|-------|------|-------|
| D1 | fix-a | sonnet/high | conductor | Fix the first thing. |
| D2 | Fix B | claude-cli:opus/high | pr | Fix the second thing. |
| D3 | fix-c | xhigh | | Fix the third thing. |
| D4 | fix-a | sonnet/medium | tui | A row whose key stands already. |
"""


def test_an_auditor_that_ends_queues_the_rows_of_its_register_once_at_the_end_of_the_queue(conductor: Round) -> None:
  conductor.ns["quietly"] = lambda verb, *args, **words: (
    asyncio.run(verb(*args, **words)) if verb == conductor.bash else verb(*args, **words)
  )
  conductor.kinds["look"] = "audit"
  conductor.queue.append(("early", "", ""))
  name = conductor.real_hire("look", "find the leaks", "defects")
  prompt = conductor.prompts[-1][0]
  assert "| Dn | key | model | area | brief |" in prompt and "the item triage-<key>" in prompt
  assert "claude-cli:sonnet/high" in prompt and "{models}" not in prompt
  conductor.ns["DEBRIEFS"] = False
  conductor.files["scratch://audit/look.md"] = REGISTER
  conductor.done[name] = (0, "found three")
  conductor.tend()
  assert [key for key, _, _ in conductor.queue] == ["early", "triage-fix-a", "triage-fix-b", "triage-fix-c"]
  assert conductor.queue[1] == (
    "triage-fix-a",
    "Fix the first thing.\n\nThis item is row D1 of the register scratch://audit/look.md of the audit look.",
    "conductor",
  )
  assert [conductor.queue[2][2], conductor.queue[3][2]] == ["pr", "triage"]
  assert conductor.efforts == {"triage-fix-a": SONNET_HIGH, "triage-fix-b": "claude-cli:opus/high"}
  assert conductor.heard[-1].startswith("The auditor look left 3 items in its register")
  conductor.register("look", name)
  assert len(conductor.queue) == 4


def test_an_auditor_whose_register_is_missing_or_has_no_table_queues_nothing(conductor: Round) -> None:
  conductor.ns["quietly"] = lambda verb, *args, **words: (
    asyncio.run(verb(*args, **words)) if verb == conductor.bash else verb(*args, **words)
  )
  conductor.kinds["look"] = "audit"
  name = conductor.real_hire("look", "find the leaks", "defects")
  conductor.ns["DEBRIEFS"] = False
  conductor.done[name] = (0, "found none")
  conductor.tend()
  assert conductor.queue == [] and len(conductor.heard) == 1
  conductor.files["scratch://audit/look.md"] = "No table here.\n| D1 | only | three |\n"
  conductor.register("look", "other")
  assert conductor.queue == []


def test_an_ended_worker_takes_the_next_item_of_its_area_until_half_of_its_window_and_a_hire_forks_the_base(
  conductor: Round,
) -> None:
  made: list[tuple[str, str]] = []
  conductor.ns["chain"] = lambda label="", source="": (made.append((label, source)), f"chain-{label}")[1]
  conductor.ns["quietly"] = lambda verb, *args, **words: (
    asyncio.run(verb(*args, **words)) if verb == conductor.bash else verb(*args, **words)
  )
  share = {"reply1": 0.3}
  conductor.ns["filled"] = share.get
  conductor.ns["transcript"] = lambda on="", of="": [("reply", "reply1", "", on)] if of == "reply" else []
  conductor.facts["chain-tui/a"] = ("chain", "chain-tui/a", "rung1", "chain1", "tui/a", "")
  conductor.facts["p1"] = ("prompt", "p1", "rung1", "chain-tui/a", "tuple[int, str]", "", "")
  conductor.working["p1"], conductor.models["p1"] = "a", SONNET_HIGH
  conductor.done["p1"] = (12, "done")
  conductor.ns["DEBRIEFS"] = False
  conductor.tend()
  assert conductor.pool == {"chain-tui/a": {"key": "a", "model": SONNET_HIGH, "thread": "tui", "pr": 12, "folders": None}}
  conductor.real_hire("b", "more", "tui")
  assert made == [] and conductor.homes["b"] == "chain-tui/a" and conductor.pool == {}
  assert "b goes on chain-tui/a, the chain of a, which worked on the same area." in conductor.heard
  conductor.pool["chain-tui/a"] = {"key": "b", "model": SONNET_HIGH, "thread": "tui", "pr": 13, "folders": ["tui"]}
  share["reply1"] = 0.6
  conductor.real_hire("c", "other", "tui")
  conductor.real_hire("d", "more", "gui")
  assert made == [("base/edit/sonnet/high", ""), ("tui/c", "chain-base/edit/sonnet/high"), ("gui/d", "chain-base/edit/sonnet/high")]
  assert conductor.bases == {f"edit {SONNET_HIGH}": "chain-base/edit/sonnet/high"}
  assert [on for word, on in conductor.rungs if "AGENTS.md" in word and "read(" in word] == ["chain-base/edit/sonnet/high"]
  share["reply1"] = 0.2
  conductor.areas["e"] = ["tui"]
  conductor.real_hire("e", "near", "ci")
  assert conductor.homes["e"] == "chain-tui/a"


def test_a_worker_takes_a_free_workspace_at_its_hire_and_gives_it_back_when_it_ends(conductor: Round) -> None:
  conductor.ns["quietly"] = lambda verb, *args, **words: (
    asyncio.run(verb(*args, **words)) if verb == conductor.bash else verb(*args, **words)
  )
  repo = str(conductor.tmp)
  first = conductor.real_hire("k1", "one", "tui")
  assert f"Work in the workspace {repo}-w1," in conductor.prompts[-1][0] and "{workspace}" not in conductor.prompts[-1][0]
  assert "-w-k1" not in conductor.prompts[-1][0]
  conductor.real_hire("fix5", "", "", conductor.FIX)
  assert conductor.prompts[-1][0].endswith(
    conductor.WORKSPACE.replace("{repo}", repo).replace("{slot}", "w2").replace("{chain}", "chain-w/fix5")
  )
  conductor.kinds["look"] = "audit"
  conductor.real_hire("look", "read")
  assert "workspace" not in conductor.prompts[-1][0]
  assert sorted(set(conductor.slots.values())) == ["w1", "w2"]
  conductor.ns["DEBRIEFS"] = False
  conductor.done[first] = (9, "done")
  conductor.tend()
  assert first not in conductor.slots
  conductor.real_hire("k3", "three", "gui")
  assert f"{repo}-w1," in conductor.prompts[-1][0]


@POSIX
@JQ
async def test_a_green_pr_whose_files_moved_on_main_merges_with_no_update_when_no_gate_sums_them(conductor: Round) -> None:
  conductor.say(r"compare/abc\.\.\.main", '{"files": [{"filename": "tui/src/app.ts"}]}')
  conductor.say(r"pr view 5 --json files", '{"files": [{"path": "tui/src/app.ts"}]}')
  conductor.say(r"pr view 5 --json state", '{"state": "MERGED"}')
  await conductor.land(conductor.pr(checks=("SUCCESS",), names=["the hooks"]), False)
  assert "update-branch" not in conductor.log()
  assert "pr merge 5 --squash" in conductor.log()
  assert 5 in conductor.merged


def two(conductor: Round, top: tuple = ("SUCCESS",)) -> None:
  conductor.prs[7] = conductor.pr(n=7, head="w/a", checks=("SUCCESS",), names=["the hooks"])
  conductor.prs[8] = conductor.pr(n=8, head="w/b", base="w/a", checks=top, names=["the hooks"])


@POSIX
@JQ
async def test_a_green_stack_merges_whole_at_once_through_gh_stack(conductor: Round) -> None:
  two(conductor)
  conductor.say(r"pr view [78] --json state", '{"state": "MERGED"}')
  await conductor.land(conductor.prs[7], False)
  assert "stack merge 8 --yes --squash" in conductor.log()
  assert "pr merge" not in conductor.log()
  assert {7, 8} <= conductor.merged
  assert conductor.heard == ["The stack #7 <- #8 merged whole."]


@POSIX
@JQ
async def test_a_stack_with_a_red_layer_does_not_merge_whole(conductor: Round) -> None:
  two(conductor, top=("FAILURE",))
  conductor.say(r"pr view 7 --json state", '{"state": "MERGED"}')
  await conductor.land(conductor.prs[7], False)
  assert "stack merge" not in conductor.log()
  assert "pr merge 7 --squash" in conductor.log()


@POSIX
@JQ
async def test_a_stack_that_gh_stack_does_not_hold_lands_one_layer_at_a_time_and_is_told_once(conductor: Round) -> None:
  two(conductor)
  conductor.say(r"pr view 8 --json state", '{"state": "OPEN"}')
  conductor.say(r"pr view 7 --json state", '{"state": "MERGED"}')
  await conductor.land(conductor.prs[7], False)
  assert conductor.log().count("stack merge 8") == 1
  assert "pr merge 7 --squash" in conductor.log()
  assert "no stack w/a" in conductor.noticed
  assert any("did not merge whole" in line for line in conductor.heard)


@POSIX
@JQ
async def test_each_chain_of_stacked_prs_is_linked_as_one_gh_stack_once(conductor: Round) -> None:
  two(conductor)
  conductor.say(r"stack link", "Created stack with 2 PRs")
  await conductor.linking()
  await conductor.linking()
  assert conductor.log().count("stack link 7 8") == 1
  assert conductor.heard == ["The stack #7 <- #8 is linked on GitHub."]
def seated(conductor: Round) -> tuple[str, str, str]:
  conductor.ns["quietly"] = lambda verb, *args, **words: (
    asyncio.run(verb(*args, **words)) if verb == conductor.bash else verb(*args, **words)
  )
  conductor.ns["DEBRIEFS"] = False
  first, second = conductor.real_hire("k1", "one", "tui"), conductor.real_hire("k2", "two", "gui")
  conductor.done.update({first: (1, "done"), second: (2, "done")})
  conductor.tend()
  return str(conductor.tmp), first, second


def test_a_chain_goes_on_in_the_workspace_that_it_held_last_when_no_live_worker_holds_it(conductor: Round) -> None:
  repo, first, second = seated(conductor)
  assert conductor.slots == {"chain-tui/k1": "w1", "chain-gui/k2": "w2"}
  again = conductor.resume("k2", "chain-gui/k2", conductor.models[second], "Go on.")
  assert conductor.prompts[-1][0].startswith("Go on.\n\nWork in the workspace " + repo + "-w2,")
  assert "earlier workspace" not in conductor.prompts[-1][0]
  assert conductor.slots[again] == conductor.slots["chain-gui/k2"] == "w2"


def test_a_chain_whose_workspace_went_to_a_live_worker_gets_a_new_one_and_is_told_to_leave_the_old_one(
  conductor: Round,
) -> None:
  repo, first, _ = seated(conductor)
  conductor.real_hire("k3", "three", "ci")
  assert f"workspace {repo}-w1," in conductor.prompts[-1][0]
  again = conductor.resume("k1", "chain-tui/k1", conductor.models[first], "Go on.")
  told = conductor.prompts[-1][0]
  assert f"Work in the workspace {repo}-w2," in told and told.endswith(
    f"Your earlier workspace {repo}-w1 now belongs to another worker: touch nothing in it and run no command there."
  )
  assert conductor.slots[again] == conductor.slots["chain-tui/k1"] == "w2"
  assert conductor.slots["chain-tui/k1"] != "w1"


def test_a_workspace_tells_its_worker_to_write_its_chain_in_the_holder_file_and_to_read_it_before_a_command(
  conductor: Round,
) -> None:
  repo, *_ = seated(conductor)
  told = conductor.prompts[0][0]
  write_it = f'echo chain-tui/k1 > "$(git -C {repo}-w1 rev-parse --absolute-git-dir)/furb-holder"'
  assert write_it in told and "{chain}" not in told
  assert "Before your first command of each prompt in a workspace, read that file: if it names another chain than chain-tui/k1" in told


def test_no_worker_gets_the_workspace_of_the_coordinator(conductor: Round) -> None:
  conductor.slots["coordinator"] = "w1"
  text, slot = conductor.placed("k", "{workspace}")
  assert slot == "w2"
  assert "-w2" in text


def test_the_triage_folds_like_items_into_the_earlier_one_before_a_hire(conductor: Round) -> None:
  words = "\n".join(f"The owner wants the line {i} of the feed to keep its color when it scrolls." for i in range(6))
  conductor.queue[:] = [
    ("converge-pr1", f"The open PR #1.\n{words}\nGap: the first row is cut off at column eighty of the screen.", "tui"),
    ("other", "Fix the flake of the TUI in the bench of a new act, which fails one run in ten.", "tui"),
    ("converge-pr2", f"The open PR #2.\n{words}\nGap: the second row has the wrong tone in the dark theme.", "tui"),
    ("converge-pr3", f"The open PR #3.\n{words}\nGap: a third row.", "gui"),
  ]
  conductor.triage()
  folded = conductor.queue[0][1]
  assert folded.startswith(conductor.TRIAGED) and folded.count("line 3 of the feed") == 1
  assert "## The item converge-pr2, folded in by the triage\nThe open PR #2.\nGap: the second row" in folded
  conductor.staff()
  assert [key for key, _ in conductor.hired] == ["converge-pr1", "other", "converge-pr3"]
  assert conductor.covering == {"converge-pr1": ["converge-pr2"]}
  assert conductor.heard == ["The triage folded the item converge-pr2 into converge-pr1, which it is like."]
  assert conductor.taken("converge-pr2") == "worker1"


@POSIX
@JQ
async def test_a_child_in_a_linked_gh_stack_is_left_to_gh_stack_when_its_parent_moves(conductor: Round) -> None:
  two(conductor)
  conductor.noticed.add("linked 7 8")
  conductor.shifted[8] = ("old", "new")
  assert await conductor.restack_forced() == set()
  assert "rebase" not in conductor.log()


@POSIX
@JQ
async def test_a_lone_failure_behind_a_green_main_reruns_its_failed_jobs_before_it_updates_the_branch(
  conductor: Round,
) -> None:
  conductor.say(r"compare/main\.\.\.abc", '{"behind_by": 3}')
  conductor.say(r"pr view 5 --json headRefOid", '{"headRefOid": "abc"}')
  conductor.say(r"run list --commit abc", json.dumps([run(1, "gates", "10:00", "failure")]))
  await conductor.mend(conductor.pr(checks=("FAILURE", "SUCCESS")), 0.0, False)
  assert "run rerun 1 --failed\n" in conductor.log()
  assert "update-branch" not in conductor.log()
  await conductor.mend(conductor.pr(checks=("FAILURE", "SUCCESS")), 0.0, False)
  assert "pr update-branch 5\n" in conductor.log()


@POSIX
@JQ
async def test_queued_runs_that_can_serve_no_merge_are_cancelled_and_a_run_above_a_red_layer_waits(conductor: Round) -> None:
  two(conductor, top=())
  conductor.prs[7]["verdict"] = "fail"
  conductor.prs[9] = conductor.pr(n=9, head="w/c", sha="new", mergeable="CONFLICTING")
  conductor.prs[10] = conductor.pr(n=10, head="w/d", sha="d1")
  runs = [
    {"databaseId": 1, "headBranch": "w/b", "headSha": "abc", "event": "pull_request"},
    {"databaseId": 2, "headBranch": "w/c", "headSha": "old", "event": "pull_request"},
    {"databaseId": 3, "headBranch": "w/gone", "headSha": "x", "event": "pull_request"},
    {"databaseId": 4, "headBranch": "w/d", "headSha": "d1", "event": "pull_request"},
    {"databaseId": 5, "headBranch": "main", "headSha": "m", "event": "push"},
  ]
  conductor.say(r"run list --workflow gates.yml --status queued", json.dumps(runs))
  await conductor.throttle()
  assert [x for x in conductor.log().splitlines() if x.startswith("run cancel")] == ["run cancel 1", "run cancel 2", "run cancel 3"]
  assert conductor.deferred == {"1": "8 abc"}
  conductor.prs[7]["verdict"] = "pass"
  conductor.says.clear()
  conductor.say(r"run list --workflow gates.yml --status queued", "[]")
  await conductor.throttle()
  assert "run rerun 1" in conductor.log()
  assert conductor.deferred == {}


@POSIX
@JQ
async def test_the_worker_of_a_draft_or_held_pr_hears_once_that_the_round_cancelled_its_gates(conductor: Round) -> None:
  conductor.busy("p1", "k7", SONNET)
  conductor.busy("p2", "k8", SONNET)
  conductor.prs[7] = conductor.pr(n=7, head="w/a", sha="s7", labels=["w:k7"], draft=True)
  conductor.prs[8] = conductor.pr(n=8, head="w/b", sha="s8", labels=["w:k8", "hold"])
  runs = [
    {"databaseId": 1, "headBranch": "w/a", "headSha": "s7", "event": "pull_request"},
    {"databaseId": 2, "headBranch": "w/b", "headSha": "s8", "event": "pull_request"},
  ]
  conductor.say(r"run list --workflow gates.yml --status queued", json.dumps(runs))
  await conductor.throttle()
  assert [x for x in conductor.log().splitlines() if x.startswith("run cancel")] == ["run cancel 1", "run cancel 2"]
  [(draft, draft_on), (held, held_on)] = conductor.prompts
  assert draft_on == "chain-k7" and "head s7 of your PR #7" in draft and "gh pr ready 7" in draft
  assert held_on == "chain-k8" and "head s8 of your PR #8" in held and "label hold" in held
  assert "checked() shows that run as cancelled" in draft
  await conductor.throttle()
  assert len(conductor.prompts) == 2


@POSIX
@JQ
async def test_a_run_of_a_pr_with_no_live_worker_is_cancelled_with_no_word_to_a_worker(conductor: Round) -> None:
  conductor.prs[7] = conductor.pr(n=7, head="w/a", sha="s7", draft=True)
  runs = [{"databaseId": 1, "headBranch": "w/a", "headSha": "s7", "event": "pull_request"}]
  conductor.say(r"run list --workflow gates.yml --status queued", json.dumps(runs))
  await conductor.throttle()
  assert "run cancel 1" in conductor.log() and conductor.prompts == []


def test_a_render_of_the_gallery_runs_no_check_and_passes(conductor: Round) -> None:
  gallery = conductor.pr(n=703, head="w/gallery")
  assert gallery["verdict"] == "pass"
  assert conductor.pr(n=7, checks=("SUCCESS",), names=["the title"])["verdict"] == "pending"


@POSIX
@JQ
async def test_a_pr_that_still_fails_after_an_update_gets_no_second_update_soon(conductor: Round) -> None:
  conductor.say(r"compare/main\.\.\.", '{"behind_by": 3}')
  assert "behind main" in await conductor.heal(conductor.pr(checks=("FAILURE", "FAILURE")), False)
  assert await conductor.heal(conductor.pr(sha="def", checks=("FAILURE", "FAILURE")), False) == ""
  assert conductor.log().count("update-branch") == 1
  conductor.now += 6 * 3600
  assert "behind main" in await conductor.heal(conductor.pr(sha="ghi", checks=("FAILURE", "FAILURE")), False)
  assert conductor.log().count("update-branch") == 2


@POSIX
@JQ
async def test_a_run_that_the_round_defers_makes_no_red_and_heal_does_not_run_it_again(conductor: Round) -> None:
  two(conductor, top=())
  conductor.prs[7]["verdict"] = "pending"
  runs = [{"databaseId": 1, "headBranch": "w/b", "headSha": "abc", "event": "pull_request"}]
  conductor.say(r"run list --workflow gates.yml --status queued", json.dumps(runs))
  await conductor.throttle()
  assert "rerun abc" not in conductor.noticed
  cancelled = conductor.pr(n=8, head="w/b", base="w/a", checks=("CANCELLED",), names=["the hooks"])
  assert cancelled["verdict"] == "pending"
  assert await conductor.heal(cancelled, False) == ""
  assert "rerun" not in conductor.log().replace("run list", "")


@POSIX
@JQ
async def test_a_hold_that_comes_off_runs_again_once_the_gates_that_were_cancelled_under_it(conductor: Round) -> None:
  conductor.prs[8] = conductor.pr(n=8, head="w/b", sha="s8", labels=["hold"])
  runs = [{"databaseId": 1, "headBranch": "w/b", "headSha": "s8", "event": "pull_request"}]
  conductor.say(r"run list --workflow gates.yml --status queued", json.dumps(runs))
  await conductor.throttle()
  assert "run cancel 1" in conductor.log().splitlines()
  held = conductor.pr(n=8, head="w/b", sha="s8", labels=["hold"], checks=("CANCELLED",), names=["the hooks"])
  assert held["verdict"] == "pending"
  assert await conductor.heal(held, False) == ""
  conductor.prs[8]["labels"] = []
  conductor.says.clear()
  conductor.say(r"run list --workflow gates.yml --status queued", "[]")
  await conductor.throttle()
  await conductor.throttle()
  free = conductor.pr(n=8, head="w/b", sha="s8", checks=("CANCELLED",), names=["the hooks"])
  assert await conductor.heal(free, False) == ""
  said = conductor.log().splitlines()
  assert [x for x in said if "rerun" in x] == ["run rerun 1"] and not [x for x in said if "headRefOid" in x]


@POSIX
@JQ
async def test_a_failure_waits_for_the_end_of_its_run_before_a_rerun_an_update_or_a_fixer(conductor: Round) -> None:
  conductor.say(r"compare/main\.\.\.abc", '{"behind_by": 3}')
  running = conductor.RUNNING[0]
  await conductor.mend(conductor.pr(checks=("FAILURE", running)), 0.0, False)
  assert "rerun" not in conductor.log() and "update-branch" not in conductor.log()
  assert conductor.hired == []


@POSIX
@JQ
async def test_the_bottom_of_a_stack_waits_while_a_layer_above_runs_its_checks(conductor: Round) -> None:
  two(conductor, top=("IN_PROGRESS",))
  await conductor.land(conductor.prs[7], False)
  assert "merge" not in conductor.log()


def test_a_red_pr_whose_checks_still_run_blocks_no_family(conductor: Round) -> None:
  conductor.efforts["c"] = SONNET
  conductor.queue[:] = [("c", "", "")]
  conductor.prs[9] = conductor.pr(n=9, checks=("FAILURE", "QUEUED"))
  assert conductor.table(conductor.prs[9]) == "running"
  conductor.staff()
  assert [key for key, _ in conductor.hired] == ["c"]


@POSIX
@JQ
async def test_three_failed_checks_get_a_fixer_while_two_jobs_wait_for_runners(conductor: Round) -> None:
  pr = conductor.pr(checks=("FAILURE", "FAILURE", "FAILURE", "QUEUED", "QUEUED"))
  assert await conductor.mend(pr, 0.0, False) == "red"
  assert [key for key, _ in conductor.hired] == ["fix5"]


@POSIX
@JQ
async def test_a_pr_whose_base_moved_to_main_with_no_run_of_the_gates_gets_one_from_the_table(conductor: Round) -> None:
  conductor.say(r"compare/main\.\.\.abc", '{"behind_by": 3}')
  pr = conductor.pr(checks=("SUCCESS",), names=["the title"], idle=10)
  assert await conductor.mend(pr, 0.0, False) == "running"
  assert "pr update-branch 5\n" in conductor.log()
  assert conductor.hired == []


@POSIX
@JQ
async def test_a_conflict_whose_maker_works_on_another_item_goes_to_a_fixer_after_fifteen_minutes(conductor: Round) -> None:
  conductor.busy("p1", "other", SONNET)
  conductor.owns["other"] = [5]
  pr = conductor.pr(labels=["w:k5"], mergeable="CONFLICTING")
  await conductor.mend(pr, 0.0, False)
  assert conductor.prompts[0][1] == "chain-other" and conductor.hired == []
  await conductor.mend(pr, conductor.OWNED - 1, False)
  assert conductor.hired == []
  await conductor.mend(pr, conductor.OWNED, False)
  assert [key for key, _ in conductor.hired] == ["fix5"] and conductor.assignee(pr) == "worker1"


@POSIX
@JQ
async def test_a_pr_in_trouble_that_no_worker_acts_on_is_told_to_main_as_a_defect_of_the_round(conductor: Round) -> None:
  conductor.limits["room"].update(sonnet=0, opus=0)
  four = raw(4, "w/red", 5, [check("the hooks", "COMPLETED", "FAILURE", "2026-10-23T10:00:00Z")])
  four["mergeable"], empty = "CONFLICTING", raw(6, "w/empty", 5, [])
  empty["changedFiles"] = 0
  conductor.say(r"pr list --state open", json.dumps([four, empty]))
  conductor.say(r"pr list --state merged", "[]")
  conductor.say(r"run list --branch main", json.dumps([{"headSha": "m1", "conclusion": "success", "databaseId": 9}]))
  conductor.say(r"issue list", "[]")
  conductor.say(r"pr list --repo uael/monty", "[]")
  await conductor.sweep(300)
  told = [message for message, _ in conductor.prompts]
  assert any(m.startswith("#4 is conflicting at sha4 and no worker acts on it, a defect of the round:") for m in told)
  assert "pr close 6 --comment" in conductor.log()
  assert "#6 (w/empty) changed no file, so the round closed it." in "\n".join([*told, *conductor.heard])


@POSIX
@JQ
async def test_a_child_whose_parent_merged_outside_the_round_is_rebased_from_its_fork_point(
  conductor: Round, tmp_path: Path
) -> None:
  shas = stacked(conductor, tmp_path)
  del conductor.prs[9]
  _, new = push_parent(conductor, tmp_path, False)
  conductor.shifted.clear()
  conductor.tips["w/parent"], conductor.parents[8] = new, "w/parent"
  del conductor.prs[7]
  conductor.prs[8]["base"] = "main"
  assert [(kid["n"], old) for kid, old in conductor.shift()] == [(8, new)]
  await conductor.restack(new, "main", conductor.prs[8])
  assert history(tmp_path, conductor, "w/kid") == ["kid", "parent (#7)", "base"]
  assert git("rev-parse", "w/kid^", cwd=tmp_path / "origin.git", env=conductor.env).strip() == shas["squash"]


LIVE_WORD = '''BRIEF = """Quoted earlier:
TROUBLE = ("red", "conflicting")
parents: dict = globals().setdefault("parents", {})
"""


def tended(pr: dict) -> bool:
  return True


def table(pr: dict) -> str:
  return "old"
'''
NEW_FILE = '''import json

# The pair of states that a round tends.
TROUBLE = ("red", "conflicting")


parents: dict = globals().setdefault("parents", {})


def tended(pr: dict) -> bool:
  return True


def table(pr: dict) -> str:
  return "new"
'''


def test_the_parts_of_a_word_are_its_top_level_definitions(conductor: Round) -> None:
  tree = ast.parse(SOURCE)
  expected: set[str] = set()
  for node in tree.body:
    if isinstance(node, ast.FunctionDef | ast.AsyncFunctionDef | ast.ClassDef):
      expected.add(node.name)
    elif isinstance(node, ast.Assign) and len(node.targets) == 1 and isinstance(node.targets[0], ast.Name):
      expected.add(node.targets[0].id)
    elif isinstance(node, ast.AnnAssign) and isinstance(node.target, ast.Name):
      expected.add(node.target.id)
  parts = conductor.parts_of(SOURCE)
  assert set(parts) == expected
  for text in parts.values():
    ast.parse(text)
  assert conductor.parts_of(LIVE_WORD) == {
    "BRIEF": LIVE_WORD.split("\n\n\ndef tended")[0],
    "tended": "def tended(pr: dict) -> bool:\n  return True",
    "table": 'def table(pr: dict) -> str:\n  return "old"',
  }


def loaded(conductor: Round, *given: tuple, after: tuple = ()) -> None:
  # Each given run is (name, word, end): end is None for a run that ended with no fault, an exception, or "open".
  facts = [("run", name, "rung0", "chain1", f"rung{i}", word, "") for i, (name, word, _) in enumerate(given, 1)]
  conductor.ns["transcript"] = lambda on="", of="": [*facts, *after]
  conductor.done.clear()
  conductor.done.update({name: end for name, _, end in given if end != "open"})


def test_a_reload_runs_each_definition_that_the_module_lacks_or_that_changed_and_gives_the_rung_and_the_names(
  conductor: Round,
) -> None:
  # A short definition that a brief quoted earlier is no live definition, and a name that the module lacks is run.
  del conductor.ns["TROUBLE"], conductor.ns["parents"]
  conductor.files["ext.py"] = LIVE_WORD
  conductor.reload("ext.py")
  run_load(conductor, conductor.rungs[-1][0])
  conductor.files["ext.py"] = NEW_FILE
  assert conductor.reload("ext.py") == ("rung2", ["TROUBLE", "parents", "table"])
  word, where = conductor.rungs[-1]
  assert where == "chain1" and word == "\n\n".join(
    [
      '# The pair of states that a round tends.\nTROUBLE = ("red", "conflicting")',
      'parents: dict = globals().setdefault("parents", {})',
      'def table(pr: dict) -> str:\n  return "new"',
      "seal('ext.py')",
    ]
  )
  run_load(conductor, word)
  assert conductor.reload("ext.py") == ("", [])
  assert conductor.TROUBLE == ("red", "conflicting")


def test_reload_starts_its_load_through_quietly_so_that_no_end_of_the_caller_cancels_it(conductor: Round) -> None:
  made: list[object] = []
  inner = conductor.ns["quietly"]
  conductor.ns["quietly"] = lambda verb, *args, **words: (made.append(verb), inner(verb, *args, **words))[1]
  conductor.files["ext.py"] = NEW_FILE
  conductor.ns["tended"] = None
  got, names = conductor.reload("ext.py")
  assert made == [conductor.ns["rung"]] and got == "rung1" and "tended" in names
  assert [on for _, on in conductor.rungs] == ["chain1"] and 'return "new"' in conductor.rungs[0][0]


ENGINE_FILE = '''from furb.engine import (
  bash,
)


async def lister() -> None:
  await bash("ls", {call})
'''


def test_a_reload_refuses_a_file_that_calls_the_engine_with_a_parameter_that_the_bash_of_the_life_lacks(
  conductor: Round,
) -> None:
  import furb
  import furb_monty
  from furb import sheet

  conductor.ns["gate"] = lambda word, on="": sheet.gate(vars(furb.python), {}, word, furb_monty.gate)
  # The bash of the engine that the gate reads has no parameter nope, as the bash of an older build had no ok.
  conductor.files["ext.py"] = ENGINE_FILE.format(call="nope=True")
  with pytest.raises(conductor.Refused) as why:
    conductor.reload("ext.py")
  assert str(why.value) == (
    "ext.py needs the engine of a newer build, since bash do not fit the engine of this life:"
    " restart the life on that build to load it"
  )
  assert conductor.rungs == []
  conductor.files["ext.py"] = ENGINE_FILE.format(call="timeout=1.0")
  assert conductor.reload("ext.py") == ("rung1", ["lister"])


def test_a_reload_leaves_a_finding_on_a_name_that_the_file_does_not_import_from_the_engine_to_the_gate(
  conductor: Round,
) -> None:
  conductor.ns["gate"] = lambda word, on="": ["line 1: error[unresolved-reference] Name `lister2` used when not defined"]
  conductor.files["ext.py"] = ENGINE_FILE.format(call="timeout=1.0")
  assert conductor.reload("ext.py") == ("rung1", ["lister"])


def test_a_load_whose_end_did_not_run_counts_for_nothing_and_loads_again(conductor: Round) -> None:
  # The definitions ran and seal did not, as in a load that was cancelled or failed between two definitions.
  exec(NEW_FILE, conductor.ns)  # noqa: S102
  conductor.files["ext.py"] = NEW_FILE
  names = ["TROUBLE", "parents", "tended", "table"]
  assert conductor.reload("ext.py")[1] == names
  run_load(conductor, conductor.rungs[-1][0])
  assert conductor.reload("ext.py") == ("", [])
  conductor.reload("ext.py")
  conductor.ns["tended"] = lambda pr: False
  assert conductor.reload("ext.py") == ("rung2", ["tended"])


def test_conducting_says_that_a_load_is_done_only_when_its_rung_is_over_with_no_fault(conductor: Round) -> None:
  for words in ("gives (rung, names)", "The load is done only when that rung is over and peek(rung) is no exception"):
    assert words in conductor.CONDUCTING


OLD_TABLE = 'def table(pr: dict) -> str:\n  return "old"\n'


def run_load(conductor: Round, word: str) -> None:
  # The load runs as the word of the rung that acts, so program() of the chain holds it under that rung.
  conductor.words["rung1"] = word
  exec(word, conductor.ns)  # noqa: S102


def test_a_reload_compares_each_definition_with_the_binding_that_the_module_holds_now(conductor: Round) -> None:
  # A later word bound an older table and ended with a fault, as an older owner_review(pr) stood in the root on 10-31.
  conductor.files["ext.py"] = NEW_FILE
  conductor.reload("ext.py")
  load = conductor.rungs[-1][0]
  run_load(conductor, load)
  exec(OLD_TABLE, conductor.ns)  # noqa: S102
  loaded(conductor, ("run1", load, None), ("run2", OLD_TABLE, ValueError("late")))
  assert conductor.reload("ext.py") == ("rung2", ["table"])


def test_a_load_ends_with_a_check_that_refuses_each_name_that_the_module_binds_with_another_text_than_the_file(
  conductor: Round,
) -> None:
  conductor.files["ext.py"] = NEW_FILE
  conductor.reload("ext.py")
  first = conductor.rungs[-1][0]
  run_load(conductor, first)
  conductor.files["ext.py"] = NEW_FILE.replace('"new"', '"newer"')
  assert conductor.reload("ext.py") == ("rung2", ["table"])
  # Another word binds tended before the load runs, as converge.py bound faulted of conductor.py in the root.
  exec("def tended(pr: dict) -> bool:\n  return False\n", conductor.ns)  # noqa: S102
  with pytest.raises(conductor.Refused, match=r"ext\.py .*: tended$"):
    run_load(conductor, conductor.rungs[-1][0])
  assert conductor.table({}) == "newer"
  assert conductor.reload("ext.py") == ("rung3", ["tended"])


def failing(conductor: Round, main: str, tip: str = "m1") -> None:
  conductor.says.clear()
  conductor.say(r"workflows/gates\.yml/runs\?", '{"workflow_runs": []}')
  stamp = "2026-10-29T10:00:00.12Z"
  conductor.say(r"run list --commit sha4", json.dumps([{"databaseId": 12, "conclusion": "failure"}]))
  failed_run(conductor, 12, {"unit": "2026-10-29T11:30:00.9Z FAILED test/test_x.py::test_y - boom\n"})
  conductor.say(r"run list --commit sha5", json.dumps([{"databaseId": 13, "conclusion": "failure"}]))
  failed_run(conductor, 13, {"unit": "FAILED test/test_x.py::test_z - bang\n"})
  failed_run(conductor, 9, {"unit": f"{stamp} FAILED test/test_x.py::test_y - boom\n"})
  runs = [{"headSha": tip, "conclusion": main, "databaseId": 9}]
  conductor.say(r"compare/main\.\.\.sha4", '{"behind_by": 2}')
  conductor.say(r"pr list --state open", json.dumps(four_five()))
  conductor.say(r"pr list --state merged", "[]")
  conductor.say(r"run list --branch main", json.dumps(runs))
  conductor.say(r"issue list", "[]")
  conductor.say(r"pr list --repo uael/monty", "[]")


def four_five() -> list[dict]:
  hooks = [check("the hooks", "COMPLETED", "FAILURE", "2026-10-29T10:00:00Z")]
  return [raw(4, "w/waits", 5, hooks), raw(5, "w/real", 5, hooks)]


@POSIX
@JQ
async def test_a_pr_whose_failed_lines_all_fail_on_main_waits_on_main_until_main_is_green(conductor: Round) -> None:
  failing(conductor, "failure")
  await conductor.sweep(300)
  assert (conductor.prs[4]["state"], conductor.prs[5]["state"]) == ("main", "red")
  assert [key for key, _ in conductor.hired] == ["fix5"] and conductor.assigned.keys() == {5}
  told = [message for message, _ in conductor.prompts]
  assert len(told) == 1 and told[0].startswith("main is red at m1.")
  assert told[0].endswith("the round updates their branches once main is green: #4.")
  assert conductor.log().count("actions/runs/12/jobs") == 1
  await conductor.sweep(300)
  assert conductor.log().count("actions/runs/12/jobs") == 1 and conductor.prs[4]["state"] == "main"
  assert (len(conductor.prompts), len(conductor.hired), conductor.heard) == (1, 1, [])
  failing(conductor, "success")
  await conductor.sweep(300)
  assert "pr update-branch 4\n" in conductor.log() and conductor.prs[4]["state"] == "running"
  assert conductor.heard == [
    "#4 waited on a red main at sha4, and main is green again, so the round updated its branch."
  ]
  assert [key for key, _ in conductor.hired] == ["fix5"] and conductor.onmain == {}


@POSIX
@JQ
async def test_a_tip_of_main_whose_run_still_runs_keeps_the_verdict_of_the_last_tip_whose_run_ended(
  conductor: Round,
) -> None:
  failing(conductor, "failure")
  await conductor.sweep(300)
  assert list(conductor.redmain) == ["m1"] and conductor.prs[4]["state"] == "main"
  failing(conductor, "", "m2")
  await conductor.sweep(300)
  assert list(conductor.redmain) == ["m1"] and conductor.prs[4]["state"] == "main"
  assert "update-branch" not in conductor.log() and conductor.heard == [] and conductor.onmain != {}
  assert (len(conductor.prompts), len(conductor.hired)) == (1, 1)
  failing(conductor, "success", "m2")
  await conductor.sweep(300)
  assert "pr update-branch 4\n" in conductor.log() and conductor.redmain == {}
  assert conductor.heard == [
    "#4 waited on a red main at sha4, and main is green again, so the round updated its branch."
  ]


@POSIX
@JQ
async def test_a_red_draft_whose_worker_ended_goes_to_its_maker_or_a_fixer_and_the_orphan_notice_names_it(
  conductor: Round,
) -> None:
  conductor.busy("p1", "k7", SONNET)
  draft = conductor.pr(7, ("FAILURE", "FAILURE"), labels=["w:k7"], draft=True)
  assert conductor.tended(draft) is False
  assert await conductor.mend(draft, 0.0, False) == "red"
  assert (conductor.hired, conductor.prompts) == ([], [])
  conductor.done["p1"] = (0, "ended")
  assert conductor.tended(draft) is True
  assert await conductor.mend(draft, 0.0, False) == "red"
  assert [key for key, _ in conductor.hired] == ["fix7"] and conductor.assignee(draft) == "worker1"
  conductor.limits["room"].update(sonnet=0, opus=0)
  seven = raw(8, "w/draft", 5, [])
  seven["isDraft"], seven["mergeable"] = True, "CONFLICTING"
  conductor.say(r"pr list --state open", json.dumps([seven]))
  conductor.say(r"pr list --state merged", "[]")
  conductor.say(r"run list --branch main", json.dumps([{"headSha": "m1", "conclusion": "success", "databaseId": 9}]))
  conductor.say(r"issue list", "[]")
  conductor.say(r"pr list --repo uael/monty", "[]")
  await conductor.sweep(300)
  told = [message for message, _ in conductor.prompts]
  assert any(m.startswith("#8 is conflicting at sha8 and no worker acts on it, a defect of the round:") for m in told)


@POSIX
@JQ
async def test_the_order_of_a_stacked_pr_reads_git_for_the_rebase_that_fits_the_base_now(
  conductor: Round, tmp_path: Path
) -> None:
  stacked(conductor, tmp_path)
  old, new = push_parent(conductor, tmp_path, False)
  said = await conductor.based_now(conductor.prs[8])
  gained = git("rev-parse", "--short", new, cwd=tmp_path, env=conductor.env).strip()
  assert f"whose head changed from {old} to {new}: git rebase origin/w/parent puts its 1 commit on a base that only advanced." in said
  assert "Its checks run on refs/pull/8/merge, the merge of its head into w/parent" in said
  assert said.endswith(f"The base gained:\n  {gained} parent2\n")
  assert await conductor.based_now(conductor.pr(n=9)) == ""


@POSIX
@JQ
async def test_the_order_of_a_stacked_pr_whose_base_was_rewritten_names_the_commit_that_it_shares_with_the_old_base(
  conductor: Round, tmp_path: Path
) -> None:
  stacked(conductor, tmp_path)
  old, _ = push_parent(conductor, tmp_path, True)
  said = await conductor.based_now(conductor.prs[8])
  assert f"git rebase --onto origin/w/parent {old} w/kid replays its 1 commit." in said
  assert "The base gained" not in said


def conflict_repo(conductor: Round, tmp_path: Path) -> str:
  env, origin = conductor.env, tmp_path / "origin.git"
  git("init", "-q", "--bare", str(origin), cwd=tmp_path, env=env)
  git("init", "-q", "-b", "main", cwd=tmp_path, env=env)
  git("remote", "add", "origin", str(origin), cwd=tmp_path, env=env)
  (tmp_path / "app.ts").write_text("a\n", encoding="utf-8")
  (tmp_path / "other.txt").write_text("o\n", encoding="utf-8")
  git("add", ".", cwd=tmp_path, env=env)
  git("commit", "-q", "-m", "base", cwd=tmp_path, env=env)
  git("push", "-q", "origin", "main", cwd=tmp_path, env=env)
  git("checkout", "-q", "-b", "w/k5", cwd=tmp_path, env=env)
  (tmp_path / "app.ts").write_text("a edited\n", encoding="utf-8")
  git("commit", "-q", "-am", "edit app", cwd=tmp_path, env=env)
  sha = git("rev-parse", "HEAD", cwd=tmp_path, env=env).strip()
  git("push", "-q", "origin", "HEAD:refs/pull/5/head", cwd=tmp_path, env=env)
  git("checkout", "-q", "main", cwd=tmp_path, env=env)
  git("rm", "-q", "app.ts", cwd=tmp_path, env=env)
  git("commit", "-q", "-m", "Split app.ts", cwd=tmp_path, env=env)
  (tmp_path / "other.txt").write_text("o2\n", encoding="utf-8")
  git("commit", "-q", "-am", "Touch other", cwd=tmp_path, env=env)
  git("push", "-q", "origin", "main", cwd=tmp_path, env=env)
  return sha


@POSIX
@JQ
async def test_an_order_for_a_conflict_names_the_commits_of_main_on_its_files_and_the_files_that_they_delete(
  conductor: Round, tmp_path: Path
) -> None:
  sha = conflict_repo(conductor, tmp_path)
  await conductor.mend(conductor.pr(mergeable="CONFLICTING", sha=sha), 0.0, False)
  brief = conductor.hired[0][1]
  assert "The commits of main since the fork that changed a file of this PR" in brief
  assert "Split app.ts (1 of your files); it deletes app.ts" in brief
  assert "Touch other" not in brief
  assert "take the file of main and apply your change again in its new place" in brief


@POSIX
@JQ
async def test_an_order_for_a_failed_check_names_each_failed_check(conductor: Round) -> None:
  pr = conductor.pr(checks=("FAILURE", "FAILURE", "SUCCESS"), names=["the hooks", "the scope", "the title"])
  await conductor.mend(pr, 0.0, False)
  assert "Its failed checks: the hooks FAILURE, the scope FAILURE." in conductor.hired[0][1]


@POSIX
@JQ
async def test_a_conflict_of_the_gallery_branch_gets_no_order_until_the_wait_ends(conductor: Round) -> None:
  pr = conductor.pr(head="w/gallery", mergeable="CONFLICTING")
  assert await conductor.mend(pr, 1000.0, False) == "running"
  assert await conductor.mend(pr, 1000.0 + conductor.GALLERY_WAIT - 1, False) == "running"
  assert conductor.hired == [] and conductor.heard == [
    "#5 (w/gallery) conflicts, and the gallery workflow builds it again from main, so the round orders no fixer."
  ]
  assert await conductor.mend(pr, 1000.0 + conductor.GALLERY_WAIT, False) == "conflicting"
  assert [key for key, _ in conductor.hired] == ["fix5"]


def test_each_brief_tells_a_worker_to_wait_for_the_checks_with_checked_and_to_rerun_only_a_machine_fault(
  conductor: Round,
) -> None:
  for brief in (conductor.WORKER, conductor.FIX):
    assert "await checked(number)" in brief
    assert "Rerun a check only when its line says [machine]" in brief
    assert "a [defect] is yours to fix" in brief


def test_the_texts_of_an_order_say_how_to_read_a_rerun_a_lagging_branch_and_a_moved_file(conductor: Round) -> None:
  assert "Read the failed log before any rerun" in conductor.FIX
  assert "git merge --ff-only origin/<your branch>" in conductor.FIXOWN
  assert "a file that your hunks change is gone from main" in conductor.FIX


def owner_chain(conductor: Round, *acts: tuple, asked: bool = False, by: str = "operator") -> None:
  facts = [("prompt", "prompt955", by, "chain212", "None", "Are you there?", "")]
  facts.append(("rung", "rung11789", "prompt955", "chain212", "", "", ""))
  if asked:
    facts.append(("reply", "reply9", "rung11789", "chain212", ""))
  chains = {"chain1": [("chain", "chain212", "rung1", "chain1", "crafted", "")], "chain212": [*facts, *acts]}
  conductor.ns["transcript"] = lambda on="", of="": [*chains.get(on, [])]


WAIT = ("wait", "wait890", "rung7", "chain212", 1800.0)
RUN = ("run", "run18859", "rung1", "chain212", "rung7", "await prompt955", "")


def test_a_wait_that_holds_a_chain_while_a_prompt_of_the_owner_is_unasked_is_cancelled_after_two_minutes(
  conductor: Round,
) -> None:
  owner_chain(conductor, WAIT)
  conductor.tend()
  conductor.now += conductor.HELD - 1
  conductor.tend()
  assert (conductor.cancelled, conductor.prompts) == ([], [])
  conductor.now += 1
  conductor.tend()
  assert conductor.cancelled == ["wait890"]
  told, on = conductor.prompts[-1]
  assert on == "chain212" and "wait890" in told and "prompt955" in told
  assert "await nothing over 60 s" in told and "peek at the next advance" in told
  assert len(conductor.prompts) == 1
  assert conductor.heard == [
    "The round cut wait890 on chain212: it held the chain while the prompt prompt955 of the owner waited unasked."
  ]
  conductor.done["wait890"] = conductor.CancelledError()
  conductor.tend()
  assert conductor.cancelled == ["wait890"] and conductor.watched == {}


def test_a_prompt_whose_rung_waits_in_the_chain_with_no_reply_is_not_asked(conductor: Round) -> None:
  owner_chain(conductor, WAIT)
  conductor.tend()
  conductor.now += conductor.HELD
  conductor.tend()
  assert conductor.cancelled == ["wait890"]


def test_a_long_wait_on_a_chain_that_works_on_the_prompt_of_the_owner_is_left_alone(conductor: Round) -> None:
  owner_chain(conductor, WAIT, asked=True)
  for _ in range(4):
    conductor.tend()
    conductor.now += 600
  assert (conductor.cancelled, conductor.prompts, conductor.heard, conductor.said) == ([], [], [], [])


def test_a_prompt_that_was_asked_or_that_the_owner_did_not_make_cuts_nothing(conductor: Round) -> None:
  for kwargs in ({"asked": True}, {"by": "rung1"}):
    owner_chain(conductor, WAIT, RUN, **kwargs)
    conductor.tend()
    conductor.now += 600
    conductor.tend()
  owner_chain(conductor, WAIT)
  conductor.done["prompt955"] = "answered"
  conductor.tend()
  conductor.now += 600
  conductor.tend()
  assert (conductor.cancelled, conductor.prompts, conductor.heard, conductor.said) == ([], [], [], [])


def test_a_run_that_holds_the_chain_is_ended_with_cancelled_error_when_the_cancel_leaves_it_open(
  conductor: Round,
) -> None:
  owner_chain(conductor, RUN)
  conductor.tend()
  conductor.now += conductor.HELD
  conductor.tend()
  assert conductor.cancelled == ["run18859"]
  assert len(conductor.said) == 1 and conductor.said[0][:2] == ("done", "run18859")
  assert isinstance(conductor.said[0][2], conductor.CancelledError)
  assert any("run18859" in x and "chain212" in x for x in conductor.heard)


def test_a_cancel_that_ends_the_run_needs_no_done_and_a_wait_goes_before_a_run(conductor: Round) -> None:
  conductor.ns["cancel"] = lambda name: (conductor.cancelled.append(name), conductor.done.update({name: 1}))
  owner_chain(conductor, RUN, WAIT)
  conductor.tend()
  conductor.now += conductor.HELD
  conductor.tend()
  assert conductor.cancelled == ["wait890"] and conductor.said == []
  conductor.done.pop("run18859", None)
  conductor.tend()
  conductor.now += conductor.HELD
  conductor.tend()
  assert conductor.cancelled == ["wait890", "run18859"] and conductor.said == []


def test_the_chain_of_the_round_itself_is_left_alone(conductor: Round) -> None:
  owner_chain(conductor, WAIT)
  conductor.lead["home"] = "chain212"
  conductor.tend()
  conductor.now += 600
  conductor.tend()
  assert (conductor.cancelled, conductor.prompts) == ([], [])


def test_threads_reads_each_transcript_once_and_fits_300_chains(conductor: Round) -> None:
  calls: list[str] = []
  book: dict[str, list[tuple]] = {"chain1": []}
  for i in range(300):
    c, p = f"chain{i + 2}", f"prompt{i + 1}"
    conductor.facts[c] = ("chain", c, "rung1", "chain1", f"t/k{i}", "")
    conductor.facts[p] = ("prompt", p, "rung1", c, "str", "brief", "claude")
    tell = ("tell", p, "rung1", [f"see https://github.com/o/r/pull/{i + 1} done"])
    book["chain1"].append(conductor.facts[c])
    book[c] = [conductor.facts[p], tell]

  def transcript(on: str = "", of: str = "") -> list[tuple]:
    calls.append(on)
    return book.get(on, [])[:]

  conductor.ns["transcript"] = transcript
  start = time.monotonic()
  out = conductor.threads()
  assert time.monotonic() - start < 5
  assert len(calls) == 301 == len(set(calls))
  assert out.count(" working prompt") == 300
  assert "  PR https://github.com/o/r/pull/7 unseen" in out


def test_the_round_writes_the_new_chain_into_the_holder_file_of_the_workspace_it_gives(conductor: Round) -> None:
  conductor.ns["quietly"] = lambda verb, *args, **words: (
    asyncio.run(verb(*args, **words)) if verb == conductor.bash else verb(*args, **words)
  )
  folder = Path(f"{conductor.tmp}-w1")
  subprocess.run(["git", "init", "-q", str(folder)], check=True, env=conductor.env)
  holder = Path(subprocess.run(["git", "-C", str(folder), "rev-parse", "--absolute-git-dir"], check=True, capture_output=True, text=True, env=conductor.env).stdout.strip()) / "furb-holder"
  holder.write_text("chain273\n", encoding="utf-8")
  _, slot = conductor.placed("k", "{workspace}", "chain-tui/k")
  assert slot == "w1"
  assert holder.read_text(encoding="utf-8") == "chain-tui/k\n"
  conductor.busy("x", "k", "claude-cli:sonnet/medium")
  conductor.slots["x"] = "w1"
  _, other = conductor.placed("k2", "{workspace}", "chain-tui/k2")
  assert other == "w2" and not Path(f"{conductor.tmp}-w2").exists()


def test_the_round_saves_the_work_of_an_earlier_item_before_it_gives_a_workspace(conductor: Round) -> None:
  conductor.ns["quietly"] = lambda verb, *args, **words: (
    asyncio.run(verb(*args, **words)) if verb == conductor.bash else verb(*args, **words)
  )
  env = conductor.env

  def git(*args: str, at: Path) -> str:
    done = subprocess.run(["git", "-C", str(at), *args], check=True, capture_output=True, text=True, env=env)
    return done.stdout.strip()

  origin, folder = Path(f"{conductor.tmp}-origin.git"), Path(f"{conductor.tmp}-w1")
  subprocess.run(["git", "init", "-q", "--bare", "-b", "main", str(origin)], check=True, env=env)
  subprocess.run(["git", "clone", "-q", str(origin), str(folder)], check=True, env=env)
  for key, value in (("user.name", "t"), ("user.email", "t@t")):
    git("config", key, value, at=folder)
  (folder / "a").write_text("1\n", encoding="utf-8")
  git("add", "a", at=folder)
  git("commit", "-q", "-m", "one", at=folder)
  git("push", "-q", "origin", "HEAD:main", at=folder)
  git("checkout", "-q", "--detach", at=folder)
  (folder / "b").write_text("2\n", encoding="utf-8")
  git("add", "b", at=folder)
  git("commit", "-q", "-m", "two", at=folder)
  (folder / "c").write_text("3\n", encoding="utf-8")
  holder = Path(git("rev-parse", "--absolute-git-dir", at=folder)) / "furb-holder"
  holder.write_text("chain273\n", encoding="utf-8")
  conductor.placed("k", "{workspace}", "chain-tui/k")
  branch = git("branch", "--show-current", at=folder)
  assert branch.startswith("w/wip-")
  assert git("status", "--porcelain", at=folder) == ""
  assert git("log", "-1", "--format=%s", at=folder) == "chore(wip): Keep the work of an earlier item"
  assert branch in git("ls-remote", "--heads", "origin", branch, at=folder)
  assert holder.read_text(encoding="utf-8") == "chain-tui/k\n"


def test_a_workspace_says_that_a_holder_with_no_live_prompt_of_a_worker_is_stale() -> None:
  source = SOURCE
  assert "A holder that names a chain with no live prompt of a worker" in source


def test_the_round_puts_the_chain_of_a_worker_in_the_workspace_it_gives_so_that_a_relative_path_acts_there(
  conductor: Round,
) -> None:
  text, slot = conductor.placed("k", "{workspace}", "chain-tui/k")
  assert slot == "w1"
  assert conductor.rungs == [(f"try:\n  cd({str(conductor.tmp) + '-w1'!r})\nexcept Refused:\n  pass", "chain-tui/k")]
  assert "give cd(" + str(conductor.tmp) + "-w1) in a word of your own" in text
  conductor.placed("k2", "{workspace}")
  assert len(conductor.rungs) == 1


@POSIX
@JQ
async def test_a_pr_whose_worker_waits_in_parked_gets_no_second_order(conductor: Round) -> None:
  conductor.homes["k8"] = "chain7"
  pr = conductor.pr(n=8, open=["- [ ] It is one change."], labels=["w:k8"])
  conductor.parked["chain7"] = {"name": "", "key": "k8", "model": SONNET, "text": "Go on."}
  assert conductor.parking(pr) == "chain7"
  await conductor.mend(pr, 0.0, False)
  assert (conductor.hired, conductor.rungs, conductor.prompts) == ([], [], [])
  del conductor.parked["chain7"]
  assert conductor.parking(pr) == ""
  await conductor.mend(pr, 0.0, False)
  assert len(conductor.prompts) == 1


@POSIX
@JQ
async def test_the_orphan_notice_names_the_parked_worker_of_the_pr(conductor: Round) -> None:
  conductor.limits["room"]["sonnet"] = 0
  conductor.owns["k8"] = [8]
  conductor.parked["chain7"] = {"name": "", "key": "k8", "model": SONNET, "text": "Go on."}
  seven = raw(8, "w/draft", 5, [])
  seven["isDraft"], seven["mergeable"] = False, "CONFLICTING"
  conductor.say(r"pr list --state open", json.dumps([seven]))
  conductor.say(r"pr list --state merged", "[]")
  conductor.say(r"run list --branch main", json.dumps([{"headSha": "m1", "conclusion": "success", "databaseId": 9}]))
  conductor.say(r"issue list", "[]")
  conductor.say(r"pr list --repo uael/monty", "[]")
  await conductor.sweep(300)
  told = [message for message, _ in conductor.prompts]
  cause = f"its worker waits on chain7 for a free slot of the cap of {conductor.cap()} chains"
  assert any(m.startswith("#8 is conflicting at sha8 and no worker acts on it, a defect of the round: " + cause) for m in told)
  assert conductor.hired == []


def global_reads(text: str) -> set[str]:
  found: set[str] = set()

  def walk(table: symtable.SymbolTable) -> None:
    found.update(s.get_name() for s in table.get_symbols() if s.is_referenced() and (table.get_type() == "module" or s.is_global()))
    for one in table.get_children():
      walk(one)

  walk(symtable.symtable(text, "word", "exec"))
  return found


def test_the_harness_binds_each_name_that_a_word_it_runs_reads(conductor: Round) -> None:
  words = [IMPORT.sub("", PULL.read_text(encoding="utf-8")), IMPORT.sub("", SOURCE)]
  missing = {name for text in words for name in global_reads(text)} - set(conductor.ns) - set(dir(builtins))
  assert not missing, f"the harness binds no {sorted(missing)}"


@POSIX
@JQ
async def test_a_pr_in_trouble_takes_the_first_model_with_room_so_it_waits_only_when_no_family_has_room(
  conductor: Round,
) -> None:
  pr = conductor.pr(n=8, open=["- [ ] It is one change."], verdict="open")
  conductor.prs[8] = pr
  for index, key in enumerate("abc"):
    conductor.busy(f"p{index}", key, SONNET)
  assert conductor.mender(pr) == OPUS
  conductor.busy("p4", "d", OPUS)
  assert conductor.mender(pr) == SONNET_HIGH
  conductor.done["p4"] = None
  await conductor.mend(pr, 0.0, False)
  assert [key for key, _ in conductor.hired] == ["box8"] and conductor.hired_models == [OPUS]


def test_a_handover_takes_the_model_of_its_maker_before_the_fixer_and_waits_on_sonnet_when_no_family_has_room(
  conductor: Round,
) -> None:
  conductor.homes["k8"] = "chain7"
  conductor.ends["k8"] = [("p9", 0)]
  conductor.models["p9"] = TOP
  pr = conductor.pr(n=8, labels=["w:k8"])
  for index, key in enumerate("abc"):
    conductor.busy(f"p{index}", key, SONNET)
  assert conductor.mender(pr) == TOP
  conductor.busy("p4", "d", TOP)
  assert conductor.mender(pr) == OPUS
  conductor.busy("p5", "e", OPUS)
  assert conductor.mender(pr) == SONNET


def test_the_triage_folds_no_audit_and_writes_the_covering_key_in_each_after(conductor: Round) -> None:
  words = "\n".join(f"The owner wants the line {i} of the feed to keep its color when it scrolls." for i in range(6))
  conductor.kinds.update({"audit-a": "audit", "audit-b": "audit"})
  conductor.queue[:] = [("audit-a", f"The open PR #1.\n{words}", "tui"), ("audit-b", f"The open PR #1.\n{words}", "tui")]
  conductor.triage()
  assert [key for key, _, _ in conductor.queue] == ["audit-a", "audit-b"]
  conductor.queue[:] = [("e1", f"The open PR #2.\n{words}", "tui"), ("e2", f"The open PR #2.\n{words}", "tui")]
  conductor.after.update({"later": ["e2", "x"], "other": ["e1", "e2"]})
  conductor.triage()
  assert [key for key, _, _ in conductor.queue] == ["e1"]
  assert conductor.after["later"] == ["e1", "x"] and conductor.after["other"] == ["e1"]


@POSIX
@JQ
async def test_a_later_judge_reads_the_gaps_of_the_earlier_heads_and_the_round_judges_each_head_until_it_is_cleared(
  conductor: Round,
) -> None:
  ran = judging(conductor, "+x\n" * 10)
  conductor.busy("worker9", "k", "claude-cli:sonnet/high")
  quote = "no hidden control flow"

  def head(sha: str) -> dict:
    pr = conductor.pr(n=7, sha=sha, labels=["w:k"], checks=("SUCCESS",))
    conductor.prs[7] = pr
    return pr

  async def judge(sha: str, gap: str) -> str:
    pr = head(sha)
    await conductor.bent(pr)
    conductor.done[conductor.judged[f"7 {sha}"]["act"]] = answered(conductor, f"7 {sha}", [(quote, gap)])
    assert await conductor.bent(pr) is True
    return conductor.prompts[-1][0]

  told = await judge("sha7", "Gap one, in a.py.")
  assert not [p for p, _ in conductor.prompts if "Earlier judges" in p]
  assert "- \"no hidden control flow\": Gap one, in a.py." in told
  pr = head("sha8")
  await conductor.bent(pr)
  message = conductor.prompts[-1][0]
  assert f'At sha7 the judge found:\n  - "{quote}": Gap one, in a.py.\nThe worker pushed up to the next head:' in message
  assert "git log --format='  - %s' sha7..sha8" in " ".join(ran)
  conductor.done[conductor.judged["7 sha8"]["act"]] = answered(conductor, "7 sha8", [(quote, "Gap two.")])
  assert await conductor.bent(pr) is True
  told = conductor.prompts[-1][0]
  assert told.index("Gap one") < told.index("Gap two") and told.count("Gap one") == 1
  for sha, gap in (("sha9", "Gap three."), ("sha10", "Gap four.")):
    told = await judge(sha, gap)
  assert {v["state"] for v in conductor.judged.values()} == {"gaps"}
  assert not [x for x in ran if "--add-label hold" in x]
  assert [on for _, on in conductor.prompts].count("chain-k") == 4
  assert all(f"Gap {x}" in told for x in ("one", "two", "three", "four"))
  assert "The round holds" not in " ".join(p for p, _ in conductor.prompts)
  pr = head("sha11")
  await conductor.bent(pr)
  conductor.done[conductor.judged["7 sha11"]["act"]] = answered(conductor, "7 sha11", [], "fixed at `a.py:1`: `+x`")
  assert await conductor.bent(pr) is True
  second = conductor.judged["7 sha11"]
  assert conductor.prompts[-1][1] == "chain-judge/second/7" and second["state"] == "cleared"
  conductor.done[second["second"]] = [(gap, "fixed") for gap in second["earlier"]]
  assert await conductor.bent(pr) is False


@POSIX
@JQ
async def test_each_fix_after_judge_rounds_heads_with_gaps_goes_to_a_stronger_model(conductor: Round) -> None:
  judging(conductor)
  conductor.busy("worker9", "k", "claude-cli:sonnet/high")
  quote = "no hidden control flow"
  strong = f"actor = {conductor.ns['STRONGER']!r}"
  for i, sha in enumerate(("s1", "s2", "s3", "s4")):
    pr = conductor.pr(n=7, sha=sha, labels=["w:k"], checks=("SUCCESS",))
    conductor.prs[7] = pr
    await conductor.bent(pr)
    conductor.done[conductor.judged[f"7 {sha}"]["act"]] = answered(conductor, f"7 {sha}", [(quote, f"Gap {i}.")])
    assert await conductor.bent(pr) is True
    rungs = [w for w, on in conductor.rungs if on == "chain-k"]
    assert (strong in rungs) == (i == 3), (i, rungs)


@POSIX
@JQ
async def test_a_hold_that_an_earlier_word_put_after_judge_rounds_heads_comes_off_and_the_pr_is_judged_again(
  conductor: Round,
) -> None:
  ran = judging(conductor)
  conductor.busy("worker9", "k", "claude-cli:sonnet/high")
  conductor.judged.update({f"7 s{i}": {"state": "held", "act": "", "fails": 0, "gaps": [f"Gap {i}."]} for i in range(3)})
  conductor.judged["8 t"] = {"state": "held", "act": "", "fails": 0, "gaps": ["Gap."]}
  conductor.prs[7] = conductor.pr(n=7, sha="s2", labels=["w:k", "hold"], checks=("SUCCESS",))
  await conductor.unheld()
  assert {v["state"] for v in conductor.judged.values()} == {"gaps"}
  assert "gh pr edit 7 --remove-label hold >/dev/null 2>&1" in ran
  assert conductor.prs[7]["labels"] == ["w:k"]
  assert not [x for x in ran if "edit 8" in x]
  assert await conductor.bent(conductor.prs[7]) is True
  told = conductor.prompts[-1][0]
  assert all(f"Gap {i}." in told for i in range(3))


def regapped(conductor: Round) -> dict:
  conductor.busy("worker9", "k", "claude-cli:sonnet/high")
  first = conductor.pr(n=7, sha="sha7", labels=["w:k"], checks=("SUCCESS",))
  conductor.prs[7] = first
  return first


async def test_a_clear_that_leaves_out_a_gap_of_an_earlier_head_counts_as_a_failed_judge(conductor: Round) -> None:
  judging(conductor)
  first = regapped(conductor)
  await conductor.bent(first)
  conductor.done[conductor.judged["7 sha7"]["act"]] = [("no hidden control flow", "Gap one.")]
  assert await conductor.bent(first) is True
  pr = conductor.pr(n=7, sha="sha8", labels=["w:k"], checks=("SUCCESS",))
  conductor.prs[7] = pr
  await conductor.bent(pr)
  assert conductor.ns["ACCOUNT"] in conductor.prompts[-1][0]
  assert conductor.judged["7 sha8"]["earlier"] == ['"no hidden control flow": Gap one.']
  conductor.done[conductor.judged["7 sha8"]["act"]] = []
  assert await conductor.bent(pr) is True
  assert conductor.judged["7 sha8"]["fails"] == 1
  assert await conductor.bent(pr) is True
  conductor.done[conductor.judged["7 sha8"]["act"]] = ([], [])
  assert await conductor.bent(pr) is True
  assert conductor.judged["7 sha8"]["state"] == "stuck"


async def test_an_earlier_gap_that_the_account_names_open_holds_the_head(conductor: Round) -> None:
  judging(conductor)
  first = regapped(conductor)
  await conductor.bent(first)
  conductor.done[conductor.judged["7 sha7"]["act"]] = [("no hidden control flow", "Gap one.")]
  assert await conductor.bent(first) is True
  pr = conductor.pr(n=7, sha="sha8", labels=["w:k"], checks=("SUCCESS",))
  conductor.prs[7] = pr
  await conductor.bent(pr)
  conductor.done[conductor.judged["7 sha8"]["act"]] = ([], [('- "no hidden control flow":  Gap one.', "`open`.")])
  assert await conductor.bent(pr) is True
  assert conductor.judged["7 sha8"]["state"] == "gaps"
  assert conductor.judged["7 sha8"]["gaps"] == ['"no hidden control flow": Gap one.']


async def test_before_the_merge_of_a_head_after_gaps_the_round_posts_the_account_and_merges_once_it_stands(
  conductor: Round,
) -> None:
  judging(conductor)
  posts: list[str] = []
  codes = [1, 0]

  async def bash(command: str, stdin: str | None = "", timeout: float = 600, show: object = None, **_: object) -> Exit:
    if command.startswith("gh pr comment"):
      posts.append(stdin or "")
      return Exit(codes.pop(0), Text("stdout", ""), Text("stderr", ""))
    said = "  fixed = True\n" if "git show" in command else "+diff\n"
    return Exit(0, Text("stdout", said), Text("stderr", ""))

  conductor.ns["bash"] = bash
  first = regapped(conductor)
  await conductor.bent(first)
  conductor.done[conductor.judged["7 sha7"]["act"]] = [("no hidden control flow", "Gap one.")]
  assert await conductor.bent(first) is True
  pr = conductor.pr(n=7, sha="sha8", labels=["w:k"], checks=("SUCCESS",))
  conductor.prs[7] = pr
  await conductor.bent(pr)
  conductor.done[conductor.judged["7 sha8"]["act"]] = ([], [('"no hidden control flow": Gap one.', "fixed at a.py:3: fixed  =  True")])
  assert await conductor.bent(pr) is True
  assert conductor.judged["7 sha8"]["state"] == "cleared"
  assert posts == []
  message, on = conductor.prompts[-1]
  assert on == "chain-judge/second/7"
  assert '- "no hidden control flow": Gap one.\n  The first judge says it is fixed at a.py:3: fixed = True' in message
  conductor.done[conductor.judged["7 sha8"]["second"]] = [('"no hidden control flow": Gap one.', "Fixed.")]
  assert await conductor.bent(pr) is True
  assert len(posts) == 1
  assert await conductor.bent(pr) is False
  assert posts[1].startswith("🤖 The judge cleared sha8, and a second blind judge saw each fix below in the diff.")
  assert '- "no hidden control flow": Gap one.\n  Fixed at a.py:3: `fixed = True`' in posts[1]
  assert conductor.judged["7 sha8"]["shown"] == {'"no hidden control flow": Gap one.': ["a.py", "fixed = True"]}
  assert await conductor.bent(pr) is False
  assert len(posts) == 2


async def test_a_merged_pr_whose_heads_had_gaps_queues_a_follow_up_for_each_gap_that_main_does_not_show_fixed(
  conductor: Round,
) -> None:
  judging(conductor)
  ran: list[str] = []

  async def bash(command: str, stdin: str | None = "", timeout: float = 600, show: object = None, **_: object) -> Exit:
    ran.append(command)
    said = "MERGED m1234567 w:k,thread:extensions\n" if command.startswith("gh pr view 9") else "yes\nno\n"
    return Exit(0, Text("stdout", said), Text("stderr", ""))

  conductor.ns["bash"] = bash
  one, two, three = '"q": Gap one.', '"q": Gap two.', '"q": Gap three.'
  account = [(one, "fixed at a.py:3"), (two, "fixed at b.py:4"), (three, "open")]
  conductor.judged["9 h1"] = {"state": "gaps", "act": "", "fails": 0, "gaps": [one, two, three]}
  conductor.judged["9 h2"] = {
    "state": "cleared",
    "act": "",
    "fails": 0,
    "account": account,
    "shown": {one: ["a.py", "x = 1"], two: ["b.py", "y = 2"]},
    "posted": True,
  }
  conductor.ends["k"] = [("worker9", 9)]
  conductor.models["worker9"] = "claude-cli:sonnet/high"
  assert await conductor.bent(conductor.pr(n=8, sha="s8")) is False
  assert "git show 'm1234567:a.py' 2>/dev/null | grep -qF -- 'x = 1'" in ran[-1]
  assert [(k, t) for k, _, t in conductor.queue] == [("gap9-2", "extensions"), ("gap9-3", "extensions")]
  brief = conductor.queue[0][1]
  assert brief.startswith("#9 merged at m123456 over a gap") and '\n\n> "q": Gap two.\n\n' in brief
  assert "main at m123456 holds no line 'y = 2' in b.py" in brief
  assert conductor.rulings["gap9-2"] == conductor.rulings["k"]
  assert (conductor.efforts["gap9-3"], conductor.kinds["gap9-3"]) == ("claude-cli:sonnet/high", "edit")
  assert conductor.judged == {}


async def test_a_head_whose_earlier_gap_has_no_fix_that_it_shows_does_not_merge_and_goes_back_to_its_worker(
  conductor: Round,
) -> None:
  judging(conductor)
  posts: list[str] = []

  async def bash(command: str, stdin: str | None = "", timeout: float = 600, show: object = None, **_: object) -> Exit:
    if command.startswith("gh pr comment"):
      posts.append(stdin or "")
    said = "\n" if "git show" in command else "+diff\n"
    return Exit(0, Text("stdout", said), Text("stderr", ""))

  conductor.ns["bash"] = bash
  first = regapped(conductor)
  await conductor.bent(first)
  conductor.done[conductor.judged["7 sha7"]["act"]] = [("no hidden control flow", "Gap one.")]
  assert await conductor.bent(first) is True
  pr = conductor.pr(n=7, sha="sha8", labels=["w:k"], checks=("SUCCESS",))
  conductor.prs[7] = pr
  await conductor.bent(pr)
  conductor.done[conductor.judged["7 sha8"]["act"]] = ([], [('"no hidden control flow": Gap one.', "fixed at a.py:3: x = 1")])
  assert await conductor.bent(pr) is True
  assert posts == []
  assert conductor.judged["7 sha8"]["state"] == "gaps"
  assert conductor.judged["7 sha8"]["gaps"] == ['"no hidden control flow": Gap one.']
  assert await conductor.bent(pr) is True
  assert conductor.prompts[-1][1] == "chain-k"


async def test_a_fix_whose_line_at_the_head_differs_from_the_text_that_the_judge_copied_does_not_merge(
  conductor: Round,
) -> None:
  judging(conductor)
  posts: list[str] = []

  async def bash(command: str, stdin: str | None = "", timeout: float = 600, show: object = None, **_: object) -> Exit:
    if command.startswith("gh pr comment"):
      posts.append(stdin or "")
    said = "  other = 2\n" if "git show" in command else "+diff\n"
    return Exit(0, Text("stdout", said), Text("stderr", ""))

  conductor.ns["bash"] = bash
  first = regapped(conductor)
  await conductor.bent(first)
  conductor.done[conductor.judged["7 sha7"]["act"]] = [("no hidden control flow", "Gap one.")]
  assert await conductor.bent(first) is True
  pr = conductor.pr(n=7, sha="sha8", labels=["w:k"], checks=("SUCCESS",))
  conductor.prs[7] = pr
  await conductor.bent(pr)
  account = [('"no hidden control flow": Gap one.', "fixed at a.py:3: fixed = True")]
  conductor.done[conductor.judged["7 sha8"]["act"]] = ([], account)
  assert await conductor.bent(pr) is True
  assert posts == []
  assert conductor.judged["7 sha8"]["state"] == "gaps"


async def test_an_account_that_names_a_line_with_no_text_is_no_account(conductor: Round) -> None:
  gap = '"q": Gap one.'
  assert conductor.accounted([(gap, "fixed at a.py:3")], [gap]) is None
  assert conductor.accounted([(gap, "`fixed at a.py:3`: `x = 1`")], [gap]) == [(gap, "fixed at a.py:3: x = 1")]
  assert conductor.accounted([(gap, "Open.")], [gap]) == [(gap, "open")]


async def seconding(conductor: Round, posts: list[str]) -> dict:
  judging(conductor)

  async def bash(command: str, stdin: str | None = "", timeout: float = 600, show: object = None, **_: object) -> Exit:
    if command.startswith("gh pr comment"):
      posts.append(stdin or "")
    said = "  fixed = True\n" if "git show" in command else "+diff\n"
    return Exit(0, Text("stdout", said), Text("stderr", ""))

  conductor.ns["bash"] = bash
  first = regapped(conductor)
  await conductor.bent(first)
  conductor.done[conductor.judged["7 sha7"]["act"]] = [("no hidden control flow", "Gap one.")]
  await conductor.bent(first)
  pr = conductor.pr(n=7, sha="sha8", labels=["w:k"], checks=("SUCCESS",))
  conductor.prs[7] = pr
  await conductor.bent(pr)
  conductor.done[conductor.judged["7 sha8"]["act"]] = ([], [('"no hidden control flow": Gap one.', "fixed at a.py:3: fixed = True")])
  assert await conductor.bent(pr) is True
  return pr


async def test_a_gap_that_the_second_judge_does_not_see_fixed_sends_the_head_back_to_its_worker(conductor: Round) -> None:
  posts: list[str] = []
  pr = await seconding(conductor, posts)
  asked = len(conductor.prompts)
  conductor.done[conductor.judged["7 sha8"]["second"]] = [('"no hidden control flow": Gap one.', "open: it still hides the flow.")]
  assert await conductor.bent(pr) is True
  assert conductor.judged["7 sha8"]["state"] == "gaps"
  assert conductor.judged["7 sha8"]["gaps"] == ['"no hidden control flow": Gap one.']
  assert await conductor.bent(pr) is True
  assert posts == []
  assert len(conductor.prompts) == asked + 1
  assert conductor.prompts[-1][1] == "chain-k"


async def test_a_second_judge_that_fails_twice_holds_the_head_and_tells_the_coordinator(conductor: Round) -> None:
  posts: list[str] = []
  pr = await seconding(conductor, posts)
  conductor.done[conductor.judged["7 sha8"]["second"]] = []
  assert await conductor.bent(pr) is True
  assert conductor.judged["7 sha8"]["second_fails"] == 1
  assert await conductor.bent(pr) is True
  assert conductor.prompts[-1][1] == "chain-judge/second/7"
  conductor.done[conductor.judged["7 sha8"]["second"]] = [("another gap", "fixed")]
  assert await conductor.bent(pr) is True
  assert conductor.judged["7 sha8"]["state"] == "stuck"
  assert "The second judge of the account of #7 at sha8 failed twice" in conductor.prompts[-1][0]
  assert posts == []


async def test_a_head_whose_diff_the_second_judge_cannot_read_does_not_merge_and_tells_the_coordinator(
  conductor: Round,
) -> None:
  judging(conductor)
  broken = []

  async def bash(command: str, stdin: str | None = "", timeout: float = 600, show: object = None, **_: object) -> Exit:
    if "git diff" in command and broken:
      return Exit(1, Text("stdout", ""), Text("stderr", "no network"))
    said = "  fixed = True\n" if "git show" in command else "+diff\n"
    return Exit(0, Text("stdout", said), Text("stderr", ""))

  conductor.ns["bash"] = bash
  first = regapped(conductor)
  await conductor.bent(first)
  conductor.done[conductor.judged["7 sha7"]["act"]] = [("no hidden control flow", "Gap one.")]
  await conductor.bent(first)
  pr = conductor.pr(n=7, sha="sha8", labels=["w:k"], checks=("SUCCESS",))
  conductor.prs[7] = pr
  await conductor.bent(pr)
  broken.append(1)
  conductor.done[conductor.judged["7 sha8"]["act"]] = ([], [('"no hidden control flow": Gap one.', "fixed at a.py:3: fixed = True")])
  asked = len(conductor.prompts)
  assert await conductor.bent(pr) is True
  assert "could not read the diff of #7 at sha8" in conductor.prompts[-1][0] and "no network" in conductor.prompts[-1][0]
  assert await conductor.bent(pr) is True
  assert len(conductor.prompts) == asked + 1


def test_the_audit_leaves_a_pr_with_a_hold_label_out_of_the_idle_check(conductor: Round) -> None:
  held = ((1, []), (2, ["hold"]), (3, ["hold:owner-review"]), (4, ["hold:x"]), (5, ["paused"]))
  conductor.ns["prs"].update({n: conductor.pr(n, idle=90, labels=labels) for n, labels in held})

  async def pruned() -> str:
    return "nothing"

  conductor.ns["prune"] = pruned
  asyncio.run(conductor.audit(False))
  assert "PRs with no change for an hour: [1, 5]" in conductor.prompts[-1][0]


def test_the_namespace_of_the_round_binds_each_name_that_the_words_use(conductor: Round) -> None:
  import builtins

  used, bound = set(), set(dir(builtins))
  for path in (PULL, HERE / "conductor.py"):
    for node in ast.walk(ast.parse(path.read_text(encoding="utf-8"))):
      if isinstance(node, ast.Name):
        (used if isinstance(node.ctx, ast.Load) else bound).add(node.id)
      elif isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
        bound.add(node.name)
      elif isinstance(node, ast.arg):
        bound.add(node.arg)
      elif isinstance(node, ast.ExceptHandler) and node.name:
        bound.add(node.name)
      elif isinstance(node, (ast.Import, ast.ImportFrom)):
        bound |= {(x.asname or x.name).split(".")[0] for x in node.names}
  assert sorted(used - bound - set(conductor.ns)) == []


@POSIX
@JQ
def test_a_comment_that_the_round_marked_is_no_comment_of_the_owner(conductor: Round) -> None:
  says_comment(conductor, ["w:k7", "hold:owner-review"], "Status of the round.\n\n<!-- furb:round -->")
  conductor.busy("p1", "k7", SONNET)
  asyncio.run(conductor.review(False))
  assert conductor.prompts == []
  assert "pr edit 7 --add-label hold" not in conductor.log()
  assert "c55" in conductor.ns["comments"]


@POSIX
@JQ
@pytest.mark.parametrize("path", ["DOCTRINE.md", "src/furb/engine.py", ".github/budgets.json"])
async def test_a_green_pr_that_changes_a_file_that_a_wall_sums_updates_with_main_before_it_merges(
  conductor: Round, path: str
) -> None:
  conductor.say(r"compare/abc\.\.\.main", json.dumps({"files": [{"filename": path}]}))
  conductor.say(r"pr view 5 --json files", json.dumps({"files": [{"path": path}]}))
  await conductor.land(conductor.pr(checks=("SUCCESS",), names=["the hooks"]), False)
  assert conductor.log().count("update-branch") == 1
  assert "pr merge" not in conductor.log()
  assert 5 not in conductor.merged


MAIN_LINES = [
  "the suite on ubuntu-latest\t2026-10-31T19:23:18.8520831Z E     AssertionError: the doctrine costs 1008 tokens, over the 1000 budget",
  "the suite on ubuntu-latest\t2026-10-31T19:23:18.8535311Z test/test_hygiene.py:141: AssertionError",
  "the suite on ubuntu-latest\t2026-10-31T19:23:18.8546544Z FAILED test/test_hygiene.py::test_doctrine - AssertionError",
]


@POSIX
@JQ
async def test_the_failed_lines_of_two_runs_that_differ_only_in_the_system_the_time_and_the_attempt_are_alike(
  conductor: Round,
) -> None:
  conductor.redmain["m1"] = MAIN_LINES
  conductor.say(r"run list --commit sha4", json.dumps([{"databaseId": 41, "conclusion": "success"}, {"databaseId": 42, "conclusion": "failure"}]))
  failed_run(
    conductor,
    42,
    {
      "the suite on windows-latest": "2026-10-31T19:46:40.2794448Z E     AssertionError: the doctrine costs 1008 tokens, over the 1000 budget\n"
      "2026-10-31T19:46:40.2795539Z test\\test_hygiene.py:141: AssertionError\n"
      "2026-10-31T19:46:40.2815986Z FAILED test/test_hygiene.py::test_doctrine - AssertionError\n"
    },
  )
  pr = conductor.pr(4, ("FAILURE",), sha="sha4")
  assert await conductor.on_main(pr) is True
  assert conductor.onmain == {4: ("sha4", "m1", True)}


@POSIX
@JQ
async def test_a_pr_whose_failed_lines_the_round_cannot_read_yet_gets_no_order_until_it_reads_them(
  conductor: Round,
) -> None:
  conductor.redmain["m1"] = MAIN_LINES
  conductor.say(r"run list --commit sha4", json.dumps([{"databaseId": 12, "conclusion": ""}]))
  failed_run(conductor, 12, {"the suite on ubuntu-latest": ""})
  pr = conductor.pr(4, ("FAILURE",), sha="sha4")
  assert await conductor.mend(pr, 0.0, True) == "running"
  assert (conductor.hired, conductor.prompts, conductor.onmain) == ([], [], {})
  conductor.says.pop()
  conductor.say(r"actions/jobs/121/logs", "2026-10-31T19:46:40.27Z test/test_hygiene.py:141: AssertionError\n")
  assert await conductor.mend(pr, 10.0, True) == "main"
  assert (conductor.hired, conductor.onmain) == ([], {4: ("sha4", "m1", True)})


@POSIX
@JQ
async def test_a_pr_whose_failed_lines_stay_unread_gets_its_fixer_after_the_unread_wait(conductor: Round) -> None:
  conductor.redmain["m1"] = MAIN_LINES
  conductor.say(r"run list --commit sha4", json.dumps([{"databaseId": 12, "conclusion": ""}]))
  failed_run(conductor, 12, {"the suite on ubuntu-latest": ""})
  pr = conductor.pr(4, ("FAILURE",), sha="sha4")
  assert await conductor.mend(pr, 0.0, True) == "running"
  assert await conductor.mend(pr, conductor.UNREAD - 1, True) == "running"
  assert conductor.hired == []
  await conductor.mend(pr, conductor.UNREAD, True)
  assert [key for key, _ in conductor.hired] == ["fix4"] and conductor.onmain == {4: ("sha4", "m1", False)}


CRATE_GATES = "fmt (cargo fmt --all --check); clippy (cargo clippy --workspace --all-targets -- -D warnings); cargo test (cargo test --workspace)"


def gated(conductor: Round, files: list[str]) -> None:
  config = (HERE.parents[2] / ".pre-commit-config.yaml").read_text(encoding="utf-8")
  conductor.say(r"pr view 5 --json files", json.dumps({"files": [{"path": x} for x in files]}))
  conductor.say(r"contents/\.pre-commit-config\.yaml\?ref=main", config)


@POSIX
@JQ
async def test_the_gates_that_the_files_of_a_pr_touch_come_from_the_hooks_of_the_repository(conductor: Round) -> None:
  gated(conductor, ["src/lib.rs", "tui/src/app.ts"])
  assert await conductor.gates_for(conductor.pr()) == f" The gates that your files touch: {CRATE_GATES}."
  conductor.says.clear()
  gated(conductor, ["tui/src/app.ts", "docs/furb.gif"])
  assert await conductor.gates_for(conductor.pr()) == ""
  conductor.says.clear()
  gated(conductor, [".furb/extensions/conductor/conductor.py"])
  ty = "ty (uv run ty check --error-on-warning); pytest (uv run pytest -q)"
  assert await conductor.gates_for(conductor.pr()) == f" The gates that your files touch: {ty}."
  conductor.says.clear()
  gated(conductor, ["src/furb/engine.py"])
  assert await conductor.gates_for(conductor.pr()) == f" The gates that your files touch: {ty}; cargo test (cargo test --workspace)."


@POSIX
@JQ
async def test_a_fixer_brief_and_the_order_to_a_maker_name_the_gates_that_the_files_touch(
  conductor: Round, tmp_path: Path
) -> None:
  sha = evidence_repo(conductor, tmp_path)
  conductor.say(r"limit 30 --json databaseId,conclusion", json.dumps([{"databaseId": 9, "conclusion": "failure"}]))
  failed_run(conductor, 9, {"gates": "FAILED test/test_hygiene.py::test_wall\n"})
  gated(conductor, ["src/lib.rs", "tui/src/app.ts"])
  line = f"The gates that your files touch: {CRATE_GATES}."
  await conductor.mend(conductor.pr(checks=("FAILURE",), sha=sha), 0.0, False)
  assert [key for key, _ in conductor.hired] == ["fix5"] and line in conductor.hired[0][1]
  conductor.assigned.clear()
  conductor.hired.clear()
  conductor.busy("p1", "k5", SONNET)
  await conductor.mend(conductor.pr(checks=("FAILURE",), sha=sha, labels=["w:k5"]), 0.0, False)
  assert line in conductor.prompts[-1][0] and conductor.hired == []
