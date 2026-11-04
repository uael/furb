import json
import re
from asyncio import CancelledError
from typing import Any

from furb.engine import (
  HIDDEN,
  ROOT,
  Refused,
  Text,
  act,
  acting,
  bash,
  cd,
  chain,
  clock,
  cwd,
  get,
  module,
  operated,
  over,
  peek,
  prompt,
  read,
  rung,
  scope,
  transcript,
  wait,
  write,
)

CONDUCTING = """The round conducts the workers of the repository in one loop. It is a way for the work, not the work: a rule that a worker, a tool or GitHub can hold does not live here.
The round is autonomous, by the ruling of the owner on 10-31: "#916 should have never been queued. Make it clear in conductor doc that it's designed to be autonomous. The word of the owner (me) can be asynchronous". So it asks the owner nothing and waits for nothing of his. His word comes by his labels on a PR, his comments and his rulings, which change what the round does from then on.
Put each item in queue as (key, brief, thread), in the order of the work, the hard work first by prompt266, or in backlog to wait behind queue, then await conduct(). Each round runs tend(), sweep(), review(), staff(), asked() and align(), then waits every seconds or until a worker ends.
1. staff() hires the first item of queue, then of backlog, whose after[key] all merged, whose model has room, else the next model of UPGRADE that has room, and whose files that must change ALONE no live worker and no open PR of the round holds, while fewer than cap() - 1 workers live, so one slot stays for a fixer: cap() is at most MOST, by prompt574, and limits["room"][family] for each model family.
2. The files of an item are areas[key], which you write, and a test file holds no item. An item with no areas holds no file. A conflict on a file that need not change alone gets a fixer, by E3 of #1084. Each item that waits on a PR or a file is one line of the next notice, with its cause, once.
3. A worker runs on a chain thread/key, at model_of(key), and an edit or a fix in a workspace {repo}-wN of its own. Its brief starts with the asks of the owner that its item carries. kinds[key] is edit (a PR), audit (a register in scratch and no PR) or fix (one red PR), and efforts[key] names its model.
4. A worker that ends gives its report whole in the next notice. A worker that ends with no PR is a decision, and its notice wakes you.
5. A PR of the round is an open PR whose label w:key names an item that the round holds. A red or conflicting one gets one order, to its maker while the maker lives, else to one fixer, and at most 2 fixers for a head, then a notice. A maker whose head did not move in OWNED seconds gives the PR to a fixer. A PR whose head differs from its branch on origin for two sweeps is closed and reopened once for that head, since GitHub did not read the branch.
6. While the newest run of the gates on main failed, the round sends no order and no fixer for a red PR, and tells you once. A red PR whose failed checks all fail on main too gets no fixer either, and the round says so in one line.
7. A green PR merges by the auto-merge of GitHub, which the round turns on for each PR of the round, else by a plain merge, else by gh stack merge for a layer of a stack, behind a ruleset of main that requires the check GATE. With no such ruleset, the round turns on no auto-merge and tells you once. A draft, an open box of its checklist, or the label hold, hold:owner-review or owner-merge keeps a PR from the merge, and the label paused takes it out of the round.
8. At ALIGN_EVERY merged PRs since the last audit of alignment, by prompt260, the round queues the next free alignN, of kind audit, which checks main against the words of the owner, and each row of its register becomes an item, queued in the order of the register after the items of queue, unless the coordinator asks for the rows before the items that no worker holds yet.
9. Each message of the owner that no ask of owner_asks names is one notice, by prompt362, and so is each ask that is not done or a decision while no item in queue, in backlog, at work or on an open PR of the round carries it, by prompt361: no word of the owner drops between his message and main. A worker that ends with a line Left: <ids> queues one follow-up for each ask that no other item carries, and a follow-up that leaves it again is a decision.
10. Each new comment of the owner on a PR of the round goes to the worker on it, else to a fixer, and the label hold comes on the PR until its worker pushes the fix and takes hold off.
11. A fix of the round does not become its own item or PR, by pick R1 of the owner: it waits in scratch://round/frozen.md, while the live module carries it, and the waiting fixes land together in one change.
A step that fails is one notice, and the round goes on at the next round. The whole state lives in the module of your chain, so a word that a checkpoint cuts loses nothing: await conduct() again in a turn that shows.
Each act of the round goes through quietly(verb, ...), so its end wakes nobody: you hear only a notice. Make each act that you do not await the same way.
A change of the round loads as the whole file, run again as a rung: each name of the state binds with setdefault and keeps its value."""

MOST = 10
OWNED = 900.0
ALIGN_EVERY = 8
OWNER = "uael"
GATE = "the gates"
FAILS = ("FAILURE", "ERROR", "TIMED_OUT", "STARTUP_FAILURE", "ACTION_REQUIRED", "CANCELLED")
GREEN = ("SUCCESS", "SKIPPED", "NEUTRAL")
HOLDS = ("hold", "hold:owner-review", "owner-merge", "paused")
ASKS = "scratch://owner/asks.json"
WORDS = "scratch://owner/words.md"
PRIVATE = ".furb/extensions/conductor/private.json"
ASK_LEFT = re.compile(r"Left: ([^\n]*)")
ALONE = re.compile(r"^(src/furb/engine\.pyi?|DOCTRINE\.md|\.github/budgets\.json|\.furb/extensions/conductor/conductor\.py)$")
UPGRADE = {"claude-cli:sonnet/medium": "claude-cli:sonnet/high", "claude-cli:sonnet/high": "claude-cli:opus/medium", "claude-cli:opus/medium": "claude-cli:opus/high"}
STALE = 2700.0
TESTS = re.compile(r"(^|/)(tests?|__tests__)/|\.test\.[a-z]+$|_test\.[a-z]+$|(^|/)test_[^/]+\.py$|(^|/)conftest\.py$")

# The module of a chain may hold these names already, with values: a name binds its default only when it is not there.
limits: dict[str, Any] = globals().setdefault(
  "limits",
  {
    "room": {"opus/high": 2, "opus/medium": 2, "sonnet/high": 3, "sonnet/medium": 3},
    "fixer": "claude-cli:sonnet/medium",
    "models": {"editor": "claude-cli:sonnet/high", "audit": "claude-cli:opus/high", "lead": "claude-cli:opus/medium"},
  },
)
queue: list[tuple[str, str, str]] = globals().setdefault("queue", [])
backlog: list[tuple[str, str, str]] = globals().setdefault("backlog", [])
working: dict[str, str] = globals().setdefault("working", {})
results: dict[str, object] = globals().setdefault("results", {})
ends: dict[str, list[tuple[str, int]]] = globals().setdefault("ends", {})
after: dict[str, list[str]] = globals().setdefault("after", {})
areas: dict[str, list[str]] = globals().setdefault("areas", {})
efforts: dict[str, str] = globals().setdefault("efforts", {})
kinds: dict[str, str] = globals().setdefault("kinds", {})
homes: dict[str, str] = globals().setdefault("homes", {})
models: dict[str, str] = globals().setdefault("models", {})
slots: dict[str, str] = globals().setdefault("slots", {})
prs: dict[int, dict] = globals().setdefault("prs", {})
merged: set[int] = globals().setdefault("merged", set())
touched: dict[int, dict] = globals().setdefault("touched", {})
assigned: dict[int, dict] = globals().setdefault("assigned", {})
lead: dict[str, str] = globals().setdefault("lead", {})
noticed: set[str] = globals().setdefault("noticed", set())
heard: list[str] = globals().setdefault("heard", [])
landed: list[int] = globals().setdefault("landed", [])
align_state: dict[str, str] = globals().setdefault("align_state", {})
comments: set[str] = globals().setdefault("comments", set())
green_since: dict[int, float] = globals().setdefault("green_since", {})
lagged: dict[int, str] = globals().setdefault("lagged", {})
reopened: set[str] = globals().setdefault("reopened", set())


def asks_kept() -> dict[str, dict]:
  try:
    got = json.loads(read(ASKS, HIDDEN).content)
  except (Refused, ValueError):
    return {}
  return {str(x["id"]): x for x in got if isinstance(x, dict) and "id" in x} if isinstance(got, list) else {}


# The file of origin/main bound owner_asks to a function, and the live round to the dict of the asks.
owner_asks: dict[str, dict] = globals()["owner_asks"] if isinstance(globals().get("owner_asks"), dict) else asks_kept()

RULES = """Run no real life of a model. Never open a window on this machine, not even for a test: the owner works on it, and each window takes his mouse and his keyboard. Run a check of the app only on the runners of CI. Start each comment, review, issue and PR body that you write on GitHub with 🤖, since you write from the owner's account and his own words carry no mark; a title keeps its own rule. Never take off the label owner-merge: only the owner takes it off, and that is his approval to merge. The owner sleeps or is away: ask him nothing and wait for nothing of his. Take each decision yourself from the brief and the rulings, say it in your PR body, and go on. A cause that a brief names is a hypothesis: check it in the log before you build on it. Ask the one who gave you this work on its chain only when it alone can decide: await prompt(str, question, on="{home}"). Set each timeout and each wait to about twice the time that the step should take, never a round bound in case, and make a long step print its progress, so that a hang shows at once. Never wait on CI in a bash for more than 10 minutes: not gh pr checks --watch, not gh run watch, not a loop of sleep around gh. To wait for the checks of your PR, await checked(number) and never write a loop of your own. Rerun a check only when its line says [machine] or [cancelled], once for each head, with gh run rerun <run> --failed, and then await checked(number) again; a [defect] is yours to fix. Once your push is done, read the checks once and close with the PR and its checks as they stand, since the round tends them."""

WORKER = """You are a worker on the repository {repo}. Do one item, and open its PR yourself.
1. Your branch is w/{key}. {workspace} Build on origin/main, and stack your branch on an open PR only when your change needs its code, and say why in the body. Make a stack with gh stack, and run skill("gh-stack") in a word of your own before you make or touch one.
2. Change it as the AGENTS.md files of the repository say, and run only the tests that your change touches.
3. Commit with the title of the PR as its message, and push the branch. Then open the PR, or update it when it stands, with pr(), which fills the template of the owner from what you give it and checks each of its rules: await pr(title, why, outcome, checks, breaking=[], fixes=0, where=<your workspace>, labels="{labels}"). title is type(area): What changed, in 70 characters or fewer, with the type feat, fix, perf, or chore for a change that no user sees, and the area the part that owns the change. why is the problem and why it matters now, from your own work. outcome lists what is true now that was not, each with the evidence of what you ran, and each gap that you leave. breaking lists each change that forces a caller of a door or of the contract to change code. 300 words at most, and no how, which the diff holds: prcheck(title, why, outcome, breaking) checks them before the commit. checks is six lines, one for each box of the checklist of the template in its order: the evidence of how you verified that box, or not: and the work that it still needs. Never write the body of a PR by hand, and open it ready, never as a draft.
4. Open your first PR within about an hour of work, as soon as one coherent part serves the item: a large item lands as a gh stack of PRs, each layer one change, and you close with the number of the last one.
5. When the item asks for no change, answer it in a comment that starts with 🤖 on its issue or PR. Close with a tuple: the number of your PR, or 0 when the item asked for no change, and your report in 200 words at most: what you measured, what you decided, and what is left.
""" + RULES + """

The item, {key}:
{what}"""

WORKSPACE = """Work in the workspace {path}, which the round gives to you alone, and to a later item once you end, so that each build starts warm. If it does not stand, make it: {repo}/script/worktree.sh {path} <your branch>, then give cd({path!r}) in a word of your own and write your chain in it: echo {chain} > "$(git -C {path} rev-parse --absolute-git-dir)/furb-holder". If it stands, the round committed and pushed the work of the earlier item there and wrote your chain in it: git fetch origin, then git switch -C <your branch> onto origin/<your branch> when that branch is on origin, else onto origin/main. Before your first command of each prompt there, read the file furb-holder in its git dir: when it names another chain than {chain}, the round gave the workspace to another worker, so run nothing in it. Leave the workspace clean when you close, each change committed and pushed."""

AUDIT = """You are an auditor on the repository {repo}. Do one item: read the code, the records and the logs that it names, find each fact that it asks for, and write each in a register, scratch://audit/{key}.md ($TMPDIR/audit/{key}.md), with its evidence: the file and the line, or the command and the lines that it printed. Check each claim of the item in the code, since it is a claim. Change no file of the repository, make no worktree, push nothing and open no PR. When your work yields items for workers, write them in that register as a table with one row for each item, in order of rank, the hard work first by prompt266: a row that reshapes a module, a contract or a protocol ranks before a row of polish or a simple row: | Dn | key | model | area | after | brief |, where Dn is its rank, key a short name in letters and dashes, model one of {models}, area the folder or the file that the item changes, after the Dn of each row that it waits for, or nothing, and brief what the worker does, as a task. The round queues each row as an item when you end. Close with a tuple: 0, and your report in 200 words at most: the path of the register, what you found, and what is left.
""" + RULES + """

The item, {key}:
{what}"""

FIX = """You fix the open PR #{n} of the repository {repo}: it {problem} at {sha}. Read it with gh pr view {n} and await checked({n}). If the head of the PR is not {sha} any more, or the PR no longer {problem}, change nothing and close with its number and a report that says so. Else work at the head of its branch {head}. {workspace} Resolve a conflict with the meaning of both sides, run the checks that the change touches, and push with --force-with-lease. Close with a tuple: the number of the PR, and your report in 200 words at most.
""" + RULES

FIXOWN = """Your open PR #{n} {problem} at {sha}. Fetch origin, fix it on your branch, resolve each conflict with the meaning of both sides, run the checks that the change touches, push with --force-with-lease, and read its checks once. A fixer takes the PR when its head does not move in {minutes} minutes."""

ANSWER = """The owner commented on the open PR #{n} of the repository {repo}, at the head of its branch {head}, and no worker is on it. Do what the comment asks, answer it on GitHub with a reply that starts with 🤖, push the fix with --force-with-lease, and take the label hold off the PR. {workspace} The comment:
{line}
Close with a tuple: the number of the PR, and your report in 200 words at most.
""" + RULES

TRUTH = "The source of truth for this item is the words of the owner, scratch://owner/words.md, and the options that he picked, scratch://owner/picks.md: where a line of this brief bends them, his words and his picks rule.\n\n"

ALIGN = """The audit of alignment {key}: {count} PRs merged since the last one. Its window of commits is git log --oneline {window}. Read that window and the diff of each PR in it, and check main against the words of the owner, which are the one source of truth: {words}, scratch://owner/picks.md, and the ledger of his asks, {asks} ($TMPDIR/owner/asks.json). Check each ask of the ledger against origin/main as a claim: set its state to done, open, partly or missing, with its evidence and its gap, and write the ledger back in the same shape. The round holds the items below, so a row of your register queues none of them again: {held}."""

SETTLE = """cd "$1" 2>/dev/null || exit 0
if [ "$(cat "$(git rev-parse --absolute-git-dir)/furb-holder" 2>/dev/null)" != "$2" ]; then
  dirty=$(git status --porcelain)
  if [ -n "$dirty$(git rev-list HEAD --not --remotes | head -1)" ]; then
    b=w/wip-$(git rev-parse --short HEAD)-$(date +%s)
    git switch -q -c "$b" || exit 0
    [ -z "$dirty" ] || { git add -A && git commit -q --no-verify -m "chore(wip): Keep the work that an earlier item left in this workspace"; }
    git push -q origin "$b"
  fi
fi
echo "$2" > "$(git rev-parse --absolute-git-dir)/furb-holder"
"""

FREE = """[ "$(df -g "$HOME" | tail -1 | awk '{print $4}')" -lt 40 ] || exit 0
for p in "$@"; do rm -rf "$p/target"; done
"""

LABELS = "gh label create hold -c FBCA04 -d 'It waits for the fix of a review before a merge' 2>/dev/null; gh label create hold:owner-review -c FBCA04 -d 'The owner iterates with its worker: no merge until he takes it off' 2>/dev/null; gh label create owner-merge -c 5319E7 -d 'The owner merges it or not' 2>/dev/null; gh label create paused -c BFBFBF -d 'The owner paused it: no fix and no merge' 2>/dev/null; true"

PRS = """gh pr list --state open --limit 200 --json number,isDraft,labels,mergeable,baseRefName,headRefName,headRefOid,statusCheckRollup,body,files,autoMergeRequest --jq '[.[] | ([.statusCheckRollup[]?] | group_by(.name // .context // "") | map(max_by(if (.status // "COMPLETED") == "COMPLETED" then (.startedAt // .completedAt // "") else "~" end))) as $c | {n: .number, draft: .isDraft, labels: [.labels[].name], mergeable, base: .baseRefName, head: .headRefName, sha: .headRefOid, checks: [$c[] | (.conclusion // .state // "")], names: [$c[] | (.name // .context // "")], open: [(.body // "") | splits("\\n") | select(startswith("- [ ] "))], files: [.files[]?.path], auto: (.autoMergeRequest != null)}]'
c=$?; echo; echo "--- $c"
gh pr list --state merged --limit 200 --json number --jq '[.[].number]'
c=$?; echo; echo "--- $c"
gh run list --branch main --workflow gates.yml --limit 20 --json headSha,conclusion --jq '.[0].headSha as $s | {sha: ($s // ""), runs: [.[] | select(.headSha == $s) | .conclusion]}'
c=$?; echo; echo "--- $c"
gh api 'repos/{owner}/{repo}/rules/branches/main' --jq '[.[] | select(.type == "required_status_checks") | .parameters.required_status_checks[].context]'
c=$?; echo; echo "--- $c"
gh api 'repos/{owner}/{repo}/commits/main/check-runs?per_page=100' --jq '[.check_runs[] | select(.conclusion == "failure" or .conclusion == "timed_out") | .name]'
c=$?; echo; echo "--- $c"
"""

LAG = """for x in "$@"; do git ls-remote origin "refs/heads/${x#*=}" | awk -v n="${x%%=*}" '{print n "\\t" $1}'; done
"""

REVIEWS = r"""for n in {prs}; do
gh api "repos/{owner}/{repo}/pulls/$n/comments" --jq '.[] | "\(.pull_request_url | split("/") | last)\tc\(.id)\t\(.user.login)\t\(.path):\(.line // .original_line)\t\(.body | gsub("[\r\n\t]"; " "))"' || exit 1
gh api "repos/{owner}/{repo}/pulls/$n/reviews" --jq '.[] | select(.body != "") | "\(.pull_request_url | split("/") | last)\tr\(.id)\t\(.user.login)\t-\t\(.body | gsub("[\r\n\t]"; " "))"' || exit 1
gh api "repos/{owner}/{repo}/issues/$n/comments" --jq '.[] | "\(.issue_url | split("/") | last)\ti\(.id)\t\(.user.login)\t-\t\(.body | gsub("[\r\n\t]"; " "))"' || exit 1
done"""


def quietly(verb, *args, **words):
  got = []

  def ear(id):
    got.append(verb(*args, **words))
    yield "done", id, None

  act("quiet", "", ear)
  return got[0]


def shell(command: str, **words):
  return bash(command, show=HIDDEN, ok=True, on=str(lead.get("shell", "")), **words)


def merged_since() -> str:
  lines = [*heard, *(["Merged: " + " ".join(f"#{n}" for n in landed) + "."] if landed else [])]
  heard.clear()
  landed.clear()
  return ("\n\nSince the last notice:\n" + "\n".join(f"- {x}" for x in lines)) if lines else ""


def notice(key: str, message: str) -> None:
  if key not in noticed:
    noticed.add(key)
    quietly(prompt, None, message + merged_since())


def once(key: str, line: str) -> None:
  if key not in noticed:
    noticed.add(key)
    heard.append(line)


def ops() -> tuple[str, str]:
  # converge reads the words of the owner in his private repository through this name of the round.
  try:
    got = json.loads(read(PRIVATE, HIDDEN).content)
  except (Refused, ValueError):
    return "", ""
  return str(got.get("repo", "")), str(got.get("digest", ""))


def cap() -> int:
  return min(MOST, sum(limits["room"].values()))


def kind_of(key: str) -> str:
  kind = kinds.get(key) or ("fix" if re.fullmatch(r"(fix|box)\d+", key) else "edit")
  return kind if kind in ("edit", "audit", "fix") else "edit"


def model_of(key: str) -> str:
  kind = kind_of(key)
  return efforts.get(key) or (limits["fixer"] if kind == "fix" else limits["models"].get("audit" if kind == "audit" else "editor", limits["fixer"]))


def runs() -> list[str]:
  return list(dict.fromkeys([*limits["models"].values(), limits["fixer"]]))


def family(model: str) -> str:
  name = model.split(":")[-1]
  return name if name in limits["room"] else name.split("/")[0]


def living() -> list[str]:
  return [n for n in working if not over(n)]


def room(model: str) -> bool:
  mine = family(model)
  return len(living()) < cap() and len([n for n in living() if family(models.get(n, "")) == mine]) < limits["room"].get(mine, 0)


def held_keys() -> set[str]:
  return {q[0] for q in [*queue, *backlog]} | set(working.values()) | set(ends) | set(homes)


def taken(key: str) -> str:
  return next((w for w in living() if working[w] == key), "") or ("queued" if any(q[0] == key for q in queue) else "")


def maker_key(pr: dict) -> str:
  return next((m[2:] for m in pr["labels"] if m.startswith("w:")), "")


def ours(pr: dict) -> bool:
  return maker_key(pr) in held_keys()


def maker_of(pr: dict) -> str:
  return next((w for w in living() if working[w] == maker_key(pr)), "")


def owner_of(pr: dict) -> str:
  held = assigned.get(pr["n"]) or {}
  fixer = held.get("who", "") if held.get("fixer") and not over(held.get("who", "")) else ""
  return fixer or maker_of(pr)


def working_at(name: str) -> str:
  return str(get(name)[3])


def slug(text: str) -> str:
  return "-".join(re.findall(r"[a-z0-9]+", text.lower()))[:34].strip("-")


def files(key: str) -> set[str]:
  return {x for a in areas.get(key, []) if (x := a.strip("/ `")) and not TESTS.search(x)}


def overlap(mine: set[str], theirs: set[str]) -> list[str]:
  pairs = [(x, y) for x in mine for y in theirs if x == y or x.startswith(y + "/") or y.startswith(x + "/")]
  return sorted({p for x, y in pairs if ALONE.search(p := y if y.startswith(x + "/") else x)})


def fit(model: str) -> str:
  up = model
  while up and not room(up):
    up = UPGRADE.get(up, "")
  return up if up in runs() else model


def seated() -> set[str]:
  return {slots[n] for n in living() if n in slots} | {slots.get("coordinator", "")}


async def placed(where: str) -> tuple[str, str]:
  held, last = seated(), slots.get(where, "")
  slot = last if last and last not in held else next(f"w{i}" for i in range(1, MOST + 3) if f"w{i}" not in held)
  path = f"{cwd()}-{slot}"
  await shell(f"sh -c {shq(SETTLE)} sh {shq(path)} {shq(where)}", timeout=120)
  quietly(rung, f"try:\n  cd({path!r})\nexcept Refused:\n  pass", on=where)
  return WORKSPACE.replace("{path!r}", repr(path)).replace("{path}", path), slot


def carried(key: str) -> str:
  held = [(aid, a) for aid, a in owner_asks.items() if key in [str(i) for i in a.get("items") or []]]
  if not held:
    return ""
  lines = "\n".join(f'- {aid}: "{a.get("quote", "")}" ({a.get("prompt", "")})' for aid, a in held)
  ids = ", ".join(aid for aid, _ in held)
  return f'This item carries the asks {ids} of the owner, in his words:\n{lines}\nServe each of them whole. End your report with the line "Served: <ids>" and, for each part that stays undone, "Left: <id> (why)".\n\n'


async def hire(key: str, what: str, thread: str = "", brief: str = "", model: str = "") -> str:
  model, kind = model or model_of(key), kind_of(key)
  marks = f" --label w:{key}" + (f" --label thread:{thread}" if thread else "")
  where = str(quietly(chain, f"{thread or 'w'}/{key}"))
  quietly(rung, f"actor = {model!r}", on=where)
  text, slot = (brief or {"audit": AUDIT, "edit": WORKER, "fix": "{what}"}[kind]).replace("{what}", what), ""
  if kind != "audit":
    quietly(shell, "".join(f"gh label create {m} --force >/dev/null 2>&1; " for m in marks.split(" --label ")[1:]))
    space, slot = await placed(where)
    text = text.replace("{workspace}", space)
  for old, new in (("{labels}", marks), ("{repo}", cwd()), ("{key}", key), ("{home}", scope(acting())), ("{models}", ", ".join(runs())), ("{chain}", where)):
    text = text.replace(old, new)
  name = str(quietly(prompt, tuple[int, str], carried(key) + text, to=model, on=where))
  working[name], models[name], homes[key] = key, model, where
  if slot:
    slots[name] = slots[where] = slot
  return name


def waiting(key: str) -> str:
  model = model_of(key)
  if model not in runs():
    return f"it asks {model}, and a worker runs only {', '.join(runs())}"
  for dep in after.get(key, []):
    if dep not in ends or dep in {q[0] for q in [*queue, *backlog]} | {working[w] for w in living()}:
      return f"{dep} has not ended" + ("" if dep in held_keys() else ", and no item holds it")
    if shut := [n for _, n in ends[dep] if n and n not in merged]:
      return f"#{shut[0]} of {dep} has not merged"
  if not room(model):
    return f"{family(model)} has no room"
  mine = files(key)
  for w in living():
    if working[w] != key and (hit := overlap(mine, files(working[w]))):
      return f"the worker of {working[w]} holds {hit[0]}"
  for n, one in touched.items():
    if f"w:{key}" not in prs.get(n, {}).get("labels", []) and (hit := overlap(mine, set(one["paths"]))):
      return f"#{n} changes {hit[0]}"
  return ""


async def staff() -> None:
  for item in [*queue, *backlog]:
    if len(living()) >= cap() - 1:
      return
    cause = waiting(item[0])
    if cause.endswith("has no room") and (up := fit(model_of(item[0]))) != model_of(item[0]):
      efforts[item[0]] = up
      cause = waiting(item[0])
    if cause:
      if not cause.endswith("has not ended"):
        once(f"waits {item[0]} {cause}", f"{item[0]} waits: {cause}.")
      continue
    (queue if item in queue else backlog).remove(item)
    await hire(*item)


def register(key: str) -> None:
  try:
    rows = read(f"scratch://audit/{key}.md", HIDDEN).lines
  except Refused:
    return
  stem, made, ranks, waits = key if key.startswith("align") else "triage", [], {}, {}
  wide = int(any(re.fullmatch(r"\|\s*Dn\s*\|.*\|\s*after\s*\|.*", x.strip()) for x in rows))
  for line in rows:
    cells = [x.strip() for x in line.strip().strip("|").split("|")]
    rank, item = cells[0], f"{stem}-{slug(cells[1])}" if len(cells) > 4 + wide else ""
    if not item or not re.fullmatch(r"D\d+", rank) or item == stem + "-" or item in made:
      continue
    ranks[rank], waits[item] = item, re.findall(r"D\d+", cells[4]) if wide else []
    if item in held_keys():
      heard.append(f"The row {rank} of the register of {key} is {item}, which the round holds already, so it queued nothing for it.")
      continue
    model, area, brief = cells[2], cells[3], "|".join(cells[4 + wide :]).strip()
    full = model if ":" in model else f"claude-cli:{model}"
    if full in runs():
      efforts[item] = full
    if "/" in area:
      areas[item] = [area.strip("`/ ")]
    kinds[item] = "edit"
    queue.append((item, f"{TRUTH}{brief}\n\nThis item is row {rank} of the register scratch://audit/{key}.md.", item))
    made.append(item)
  for item in made:
    if deps := [ranks[d] for d in waits[item] if d in ranks]:
      after[item] = deps
  if made:
    heard.append(f"The register of {key} queued {', '.join(made)}.")


def kept_asks() -> None:
  quietly(write, Text(ASKS, json.dumps([{"id": i, **a} for i, a in owner_asks.items()], indent=1) + "\n"))


def left(key: str, report: str) -> None:
  ids = sorted({m for hit in ASK_LEFT.finditer(report) for m in re.findall(r"A\d+", hit[1])} & set(owner_asks))
  others = ({q[0] for q in [*queue, *backlog]} | {working[w] for w in living()}) - {key}
  for aid in ids:
    a = owner_asks[aid]
    if a.get("state") == "decision" or others & {str(i) for i in a.get("items") or []}:
      continue
    if key.startswith("ask-"):
      a.update(state="decision", gap=report[:600])
      notice(f"left twice {aid} {key}", f"The follow-up {key} of the ask {aid} of the owner left it again, so it needs your decision and no new item. Its report:\n\n{report}")
      continue
    n = 1
    while f"ask-{aid.lower()}-{n}" in held_keys():
      n += 1
    item = f"ask-{aid.lower()}-{n}"
    kinds[item], efforts[item], areas[item] = "edit", model_of(key), list(areas.get(key, []))
    a.update(state="partly", gap=f"{key} left it: {report[:400]}", items=[*(a.get("items") or []), item])
    brief = f"{TRUTH}The ask {aid} of the owner, in his words (from {a.get('prompt')}):\n\n> {a.get('quote')}\n\nThe worker of {key} left it, in its words:\n\n{report}\n\nDo what it left, and only that."
    queue.append((item, brief, item))
    heard.append(f"The worker of {key} left the ask {aid} of the owner, so the round queued {item}.")
  if ids:
    kept_asks()


def tend() -> None:
  for name in [x for x in working if over(x)]:
    key = working.pop(name)
    slots.pop(name, None)
    got = results[name] = peek(name)
    n = got[0] if isinstance(got, tuple) and got and isinstance(got[0], int) else 0
    report = got[1] if isinstance(got, tuple) and len(got) == 2 and isinstance(got[1], str) else repr(got)[:2000]
    ends.setdefault(key, []).append((name, n))
    if kind_of(key) == "audit":
      heard.append(f"The auditor {key} ({name}) ended: {report}")
      register(key)
    elif n > 0:
      heard.append(f"The worker {key} ({name}) ended with #{n}: {report}")
    else:
      notice(f"end {name}", f"The worker {key} ({name}) ended with no PR, which needs your decision:\n\n{report}")
    left(key, report)
  if idle := sorted({s for s in slots.values() if s not in seated()}):
    quietly(shell, f"sh -c {shq(FREE)} sh " + " ".join(shq(f"{cwd()}-{s}") for s in idle))


def state(pr: dict) -> str:
  checks = pr["checks"]
  if pr["mergeable"] == "CONFLICTING":
    return "conflicting"
  if pr["open"] or (any(c in FAILS for c in checks) and all(c in (*FAILS, *GREEN) for c in checks)):
    return "red"
  return "green" if GATE in pr["names"] and all(c in GREEN for c in checks) else "running"


async def mend(pr: dict, now: str) -> None:
  n, sha = pr["n"], pr["sha"]
  held = assigned.get(n) or {}
  turn = held if held.get("sha") == sha else {}
  if held.get("fixer") and not over(held["who"]) or turn and not over(turn["who"]) and clock() - turn["at"] < OWNED or taken(f"fix{n}"):
    return
  problem = "conflicts with its base" if now == "conflicting" else "has an open box of its checklist: " + "; ".join(x[6:] for x in pr["open"]) if pr["open"] else "has a failed check"
  maker = maker_of(pr)
  if maker and not turn:
    order = FIXOWN.replace("{n}", str(n)).replace("{problem}", problem).replace("{sha}", sha[:7]).replace("{minutes}", str(int(OWNED // 60)))
    quietly(prompt, None, order, on=working_at(maker))
    assigned[n] = {"sha": sha, "who": maker, "at": clock(), "fixer": False, "maker": maker, "tries": 0}
    return
  tries = turn.get("tries", 0)
  if tries >= 2:
    notice(f"spent {n} {sha}", f"#{n} {problem} at {sha[:7]}, and 2 fixers ended at this head and did not move it, so the round sends no third one.")
    return
  if not room(fixer := fit(limits["fixer"])):
    once(f"fixer {n} {sha}", f"#{n} {problem} at {sha[:7]}, and waits for a fixer on {limits['fixer']}, which has no room.")
    return
  brief = FIX.replace("{n}", str(n)).replace("{problem}", problem).replace("{sha}", sha).replace("{head}", pr["head"])
  thread = next((m[7:] for m in pr["labels"] if m.startswith("thread:")), "")
  who = await hire(f"fix{n}", "", thread, brief, fixer)
  assigned[n] = {"sha": sha, "who": who, "at": clock(), "fixer": True, "maker": turn.get("maker", ""), "tries": tries + 1}


async def sweep() -> None:
  got = await shell(PRS, timeout=300)
  bits = re.split(r"\n--- (\d+)\n", "\n" + got.stdout.content)
  if len(bits) != 11 or any(code != "0" for code in bits[1:8:2]):
    raise Refused(f"gh listed no PRs: {got.stderr.content.strip()[-300:] or 'it said nothing'}")
  mine = set(touched)
  prs.clear()
  prs.update({pr["n"]: pr for pr in json.loads(bits[0])})
  fresh = set(json.loads(bits[2])) - merged
  merged.update(fresh)
  landed.extend(sorted(fresh & mine))
  main = json.loads(bits[4])
  red = "failure" in main["runs"]
  lead["main"] = str(main["sha"])
  gated = GATE in json.loads(bits[6])
  try:
    broke = set(json.loads(bits[8]))
  except ValueError:
    broke = set()
  heads = {n: pr for n, pr in prs.items() if ours(pr) and "paused" not in pr["labels"]}
  sought = await shell(f"sh -c {shq(LAG)} sh " + " ".join(shq(f"{n}={pr['head']}") for n, pr in sorted(heads.items())), timeout=120) if heads else None
  remote = {p[0]: p[1] for p in (x.split("\t") for x in sought.stdout.lines) if len(p) == 2} if sought else {}
  if red:
    notice(f"red {main['sha']}", f"main is red at {str(main['sha'])[:7]}: find its cause first. The round sends no order and no fixer for a red PR until the gates of main pass.")
  if not gated:
    notice("ruleset", f"main has no ruleset that requires the check {GATE}, so the round turns on no auto-merge and merges no PR.")
  for gone in [k for k in [*touched, *assigned, *green_since, *lagged] if k not in prs]:
    touched.pop(gone, None)
    lagged.pop(gone, None)
    green_since.pop(gone, None)
    assigned.pop(gone, None)
  flips = []
  for n in sorted(prs):
    pr = prs[n]
    if not ours(pr) or "paused" in pr["labels"]:
      continue
    touched[n] = {"sha": pr["sha"], "paths": pr["files"]}
    now = pr["state"] = state(pr)
    ahead = remote.get(str(n), "")
    if ahead and ahead == pr["sha"]:
      lagged.pop(n, None)
    elif ahead and lagged.get(n) != ahead:
      lagged[n] = ahead
    elif ahead and f"{n} {ahead}" not in reopened:
      reopened.add(f"{n} {ahead}")
      flips.append(f"gh pr close {n} && {{ gh pr reopen {n} || gh pr reopen {n}; }}")
      once(f"lag {n} {ahead}", f"#{n} kept the head {pr['sha'][:7]} for two sweeps while its branch {pr['head']} holds {ahead[:7]} on origin, so the round closed and reopened it, once for this head.")
      continue
    failing = {name for name, end in zip(pr["names"], pr["checks"]) if end in FAILS}
    labelled = bool(set(HOLDS) & set(pr["labels"]))
    if now != "green":
      green_since.pop(n, None)
    elif clock() - green_since.setdefault(n, clock()) > STALE and not labelled:
      once(f"stale {n} {pr['sha']}", f"#{n} is green at {pr['sha'][:7]} for {int(STALE // 60)} minutes and has not merged.")
    want = gated and not (labelled or pr["draft"] or pr["open"]) and now != "conflicting"
    if want != pr["auto"]:
      flips.append(f"gh pr merge {n} --auto --squash || gh pr merge {n} --squash || gh stack merge {n} --yes --squash" if want else f"gh pr merge {n} --disable-auto")
    if now == "red" and red:
      once(f"main waits {n} {pr['sha']}", f"#{n} is red at {pr['sha'][:7]} while main is red, so it waits for main.")
    elif now == "red" and failing and not pr["open"] and not failing - broke:
      once(f"main fails {n} {pr['sha']}", f"#{n} is red at {pr['sha'][:7]} only on {', '.join(sorted(failing))}, which main fails too, so the round hires no fixer for it.")
    elif now in ("red", "conflicting") and not labelled and not (pr["draft"] and maker_of(pr)):
      await mend(pr, now)
  if flips:
    await shell("".join(f"{{ {x}; }} >/dev/null 2>&1; " for x in flips), timeout=300)


async def review() -> None:
  mine = sorted(n for n, pr in prs.items() if ours(pr))
  if not mine:
    return
  got = await shell(REVIEWS.replace("{prs}", " ".join(map(str, mine))), timeout=300)
  if got.code != 0:
    raise Refused(f"gh read no comments: {got.stderr.content.strip()[-300:]}")
  rows = [p for p in (x.split("\t", 4) for x in got.stdout.lines) if len(p) == 5 and p[1] not in comments]
  seed = not comments
  comments.update(p[1] for p in rows)
  held: set[int] = set()
  for p in [] if seed else [p for p in rows if p[2] == OWNER and not p[4].startswith("🤖") and p[0].isdigit() and int(p[0]) in prs]:
    pr = prs[int(p[0])]
    line = f"#{p[0]} {p[1]} {p[3]}: {p[4][:1500]}"
    own = owner_of(pr)
    if own:
      quietly(prompt, None, f"A new comment of the owner on your PR #{p[0]}. Do what it asks, answer it on GitHub with a reply that starts with 🤖, push the fix, and take the label hold off.\n{line}", on=working_at(own))
    elif not taken(f"fix{p[0]}"):
      answer = ANSWER.replace("{n}", p[0]).replace("{head}", pr["head"]).replace("{line}", line)
      queue.insert(0, (f"fix{p[0]}", answer, next((m[7:] for m in pr["labels"] if m.startswith("thread:")), "")))
    if "hold:owner-review" not in pr["labels"]:
      held.add(pr["n"])
  if held:
    await shell("".join(f"gh pr edit {n} --add-label hold >/dev/null 2>&1; " for n in sorted(held)), timeout=300)


def asked() -> None:
  if not owner_asks:
    return
  named = {p for a in owner_asks.values() for p in re.findall(r"prompt\d+", str(a.get("prompt", "")))}
  for x in transcript(lead.get("home") or ROOT, "prompt"):
    if operated(str(x[2])) and x[4] == "str" and str(x[1]) not in named:
      notice(f"unasked {x[1]}", f"The message {x[1]} of the owner holds no ask in owner_asks. Split it into its asks, each with his quote and the items that carry it, before anything else.")
  carrying = {q[0] for q in [*queue, *backlog]} | {working[w] for w in living()} | {maker_key(pr) for pr in prs.values()}
  for aid, a in owner_asks.items():
    items = [str(i) for i in a.get("items") or []]
    if a.get("state") in ("done", "decision") or carrying & set(items):
      continue
    why = f"no item carries it now: its items {', '.join(items)} ended" if items else "no item names it"
    notice(f"uncarried {aid} {' '.join(items)}", f"The ask {aid} of the owner ({a.get('prompt')}) is {a.get('state') or 'open'}, and {why}. Queue an item for it, or mark it done with the evidence of main, before anything else.")


def owner_words() -> None:
  rows = []
  for x in transcript(ROOT, "prompt"):
    if operated(str(x[2])) and str(x[5]).strip():
      rows.append(f"## {x[1]}, his message\n\n{x[5]}\n\n")
    elif x[6] and operated(str(x[6])) and x[4] != "None" and isinstance(said := peek(x[1]), str) and said.strip():
      rows.append(f"## {x[1]}, his answer to my question\n\n{said}\n\n")
  quietly(write, Text(WORDS, "# The words of the owner in this session, verbatim, in order\n\n" + "".join(rows)))


def align() -> None:
  if "base" not in align_state:
    align_state["base"] = str(len(merged))
    return
  count = len(merged) - int(str(align_state["base"]))
  if count < ALIGN_EVERY or any(re.fullmatch(r"align\d+", k) for k in [*(q[0] for q in queue), *(working[w] for w in living())]):
    return
  n, held = 1, held_keys() | set(kinds)
  while f"align{n}" in held:
    n += 1
  key, since, head = f"align{n}", str(align_state.get("sha", "")), lead.get("main", "")
  owner_words()
  window = f"{since}..{head}" if since and head else "-40 origin/main"
  text = ALIGN.replace("{key}", key).replace("{count}", str(count)).replace("{window}", window).replace("{words}", WORDS)
  kinds[key] = "audit"
  queue.append((key, text.replace("{asks}", ASKS).replace("{held}", ", ".join(sorted(held_keys())) or "none"), "align"))
  align_state.update(base=str(len(merged)), n=str(n), sha=head or since)
  heard.append(f"{count} PRs merged since the last audit of alignment, so the round queued {key}.")


def follow() -> str:
  here = scope(acting())
  old = lead.get("home", "")
  lead["home"] = here
  if old and old != here and isinstance(was := module(old).get("lead"), dict):
    was["home"] = here
  return here


def ended(here: str, cause: str) -> None:
  quietly(prompt, None, f"The round on {here} ended: {cause}. Until you await conduct() again, no sweep, no merge, no fixer and no hire runs." + merged_since())


async def conduct(every: float = 300) -> dict[str, object]:
  here, at = follow(), "labels"
  try:
    lead["shell"] = str(chain("commands"))
    cd(cwd(), on=lead["shell"])
    await shell(LABELS, timeout=120)
    while lead.get("home") == here:
      for at in ("tend", "sweep", "review", "staff", "asked", "align"):
        try:
          if at in ("sweep", "review", "staff"):
            await {"sweep": sweep, "review": review, "staff": staff}[at]()
          else:
            {"tend": tend, "asked": asked, "align": align}[at]()
          for done in [k for k in noticed if k.startswith(f"fault {at} ")]:
            noticed.discard(done)
        except Exception as fault:
          notice(f"fault {at} {type(fault).__name__}", f"The round on {here} met a fault in its step {at}: {type(fault).__name__}: {str(fault)[:1000]}. It goes on at the next round, and tells this fault again only after a round of this step with no fault.")
      if not (queue or backlog or living() or any(ours(pr) and "paused" not in pr["labels"] for pr in prs.values())):
        ended(here, "it holds no item and no open PR that it tends")
        return results
      at = "wait"
      await wait(every, *living())
  except CancelledError:
    ended(here, f"it was cancelled in its step {at}")
    raise
  except Exception as fault:
    ended(here, f"the fault {type(fault).__name__}: {str(fault)[:1000]}, in its step {at}")
    raise
  return results
