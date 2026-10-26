import json
from asyncio import CancelledError

from furb.engine import (
  HIDDEN,
  OPERATOR,
  ROOT,
  Exit,
  Refused,
  Text,
  acting,
  bash,
  chain,
  clock,
  close,
  cwd,
  gate,
  get,
  module,
  over,
  paused,
  peek,
  prompt,
  read,
  rung,
  scope,
  transcript,
  wait,
  wake,
  write,
)

CONDUCTING = """Conduct the work in one loop, and plan no further than the next step.
Put each item in queue as (key, brief, thread), or in backlog to wait behind queue, then await conduct(). Each round tends the workers and the open PRs, hires a worker for each item, cap() at once, and waits every seconds or until a worker ends. It hires only when no open PR of the round needs a worker that it has not got: a failed check, a conflict or an open box, on a PR with no label paused or hold. So the fixers take the free slots first, and the round merges before it starts new work. It moves the first item of backlog to queue while queue and working hold fewer than cap() items.
The owner rules, on 10-23: "this kind of item should be done by sonnet, not opus. You need to use Sonnet more, when it make sense. And opus/medium should be your default, not opus/high. opus/high is kinda the max, xhigh for the coordination and my rulings only. most of the time sonnet/{medium,high} and opus/high are perfect." So the default of a worker and of a debrief, which asks the model of its worker, is ACTOR, opus/medium. A fixer (fixN, boxN) runs on limits["fixer"], sonnet/high, by the ruling of the operator. A worker runs only a model of MODELS: sonnet/medium, sonnet/high, opus/medium or opus/high. hire() refuses xhigh and max, and the round starts no item that asks for one and tells you. You choose sonnet for an item where it makes sense, as efforts[key] = "claude-cli:sonnet/medium" before you queue it.
The owner rules again, on 10-23: "From now on: 2 opus workers, 3 sonnet ones. max." The owner rules again, on 10-26: "add one opus/high worker for the most complex stuff". So limits["room"] gives the room of each family of model: the round runs at most 1 worker on opus/medium, 3 on sonnet and 1 on opus/high at once, by the ruling of the operator, and counts only the workers whose prompt is not over. limits lives in the state of the round, so a reload of the extension keeps it: you change it there, as limits["room"]["opus"] = 2, and cap() gives the sum of the room. You give opus/high only to the most complex items, as efforts[key] = "claude-cli:opus/high". An item whose model has no room keeps its place in queue while the next item whose model has room starts. A fixer goes before a new item. A worker that ended and whose PR needs a worker goes on in its own chain on HANDOVER, sonnet, with a new prompt there, when sonnet has room; else a fixer starts on limits["fixer"] when sonnet has room.
The round runs on one chain, lead["home"]. After a compaction of your chain, await conduct() in the new chain: it names that chain, the round of the old chain ends at its next wake, and the state that the round reads is the state of the new chain, which you change there.
A fault of one step of a round, as an empty answer of gh, ends no round: the round tells you once each kind of fault, with the fault and its step, and goes on at the next round. Each json.loads of a command reads its output only when the command succeeded. When the round ends for any cause but a new home, it tells you as a decision, with the cause and what stops.
hire(key, brief, thread) starts a worker from the base chain of its actor in bases, at its actor in efforts or ACTOR, on a chain labeled thread/key, with the brief in custom or WORKER, and its PR carries the labels w:key and thread:thread.
A worker does one item in a worktree of its own, runs only the tests that its change touches, opens its PR, and closes with its URL.
The round tells you a worker that ends with no PR, with its answer, or that says nothing for 45 minutes, relays to you each question that a worker puts to the operator, and wakes a worker that paused. results keeps the answer of each worker under its prompt.
When a worker ends, the round asks it debrief() once, and adds each friction that it gives to TOOLING, the ledger of the defects of the tooling, as flaw(what) adds a defect that you find or patch live. An entry counts each time that it comes again.
A check counts by its newest run. A failed check heals with no worker first, once for each head sha: the round updates the branch of a PR on main that is behind main while main is green, else it reruns the failed jobs of the run whose failure is the one failed check of the PR. It reruns once, for each head sha, a run that was cancelled at the head of a PR, and a check cancelled again at that sha counts as failed.
Then the round sends a conflict or a failed check at a new head to the worker that owns the PR, again after 30 minutes, and to a fixer after an hour, or at once when no worker owns it. Just before it sends an order or hires a fixer, the round reads the PR again, and drops the order when the head moved or the conflict is gone; it sends a PR no second order within 10 minutes of the first, and it names in an order the base of a stacked PR with the head that the base had before. While a fixer owns a PR, the round neither updates its branch nor merges it, and it tells a worker that owns a branch when it updates or rebases it, or when a fixer starts on it. A mergeable that gh gives as UNKNOWN is probed with git merge-tree, and the round rebases a child of a PR whose head was force-pushed onto the new head of its parent.
The round merges each green PR into main from the lowest number when no file that it changes changed on main since its branch forked, after it moves the PRs that stack on it onto main and rebases each there with git rebase --onto from the head that it held of the PR that merged, in a scratch worktree, and pushes it with --force-with-lease (a rebase that conflicts is left to the fixer); else it updates the branch, and only when main is green.
The label paused takes a PR out of the round. A draft, the label hold, a review that waits for its fix, and the label owner-merge, until the owner takes it off as his approval, keep a PR from a merge.
A notice wakes you only for a decision, with the context that it needs: each event that needs none, as a worker that ends with a PR, a heal, a merge or a PR made outside the round, is one line in the next notice. The round tells you when main is red, with the lines that failed, when a PR fails to merge, with what gh said, or when a PR on main with no draft, no hold, no worker and no check that runs has no change for 45 minutes, with its checks, and each comment or issue of the owner in the private repository of the decisions, which PRIVATE names beside this word and git does not hold. await decide(title, text) opens a decision there as an issue, and news(line) adds a line at the top of its digest issue.
Each round queues each open issue that the owner opened, or that he assigned to himself, as an item issueN, once, and its PR closes it.
Each round tells you once each fork PR of UPSTREAM that holds the label upstream-ready and not upstreamed: by the ruling of the owner, you then open its PR on pydantic/monty with the body that he reviewed, and put the label upstreamed on the fork PR.
Each round sends each new comment of the owner on a PR or on his issue to the worker that owns it, puts the label hold on that PR, and tells you only the comments that no worker owns. A text on GitHub counts only when the owner wrote it with no 🤖 at its start, or when he put a 👍 on it; each worker starts what it writes there with 🤖.
Each round sets one status label on each open PR and on each issue of the owner that it took: s:queued, s:working, s:checks, s:red, s:boxes, s:held, s:conflict, s:stacked, s:ready, or s:owner (green, waits for the owner).
Each round reads the gates: when the median time of a job in its last five green runs on main passes its budget in .github/budgets.json, which script/budgets.py sets from measures, it queues an item budget-<job>, once until five new runs measure the job; and when the gates fail, then pass on the same commit, it puts each test that failed in the ledger of the flakes, flaky, and at once queues an item flake-<test>, or tells the worker that holds that item.
Every twelfth round audits the work against its measure, the open items, writes its line in the digest, and tells you the open entries of TOOLING, the most repeated first: turn the worst into the next item. The audit prunes what the work left behind, and loses no work: a worktree of the round whose item is not in working, queue or backlog and whose branch merged, closed, or holds nothing that main lacks, with its local branch, once its changes that are not committed are a patch and its commits that only this machine holds are a bundle, in the folder of the day in ARCHIVE; a remote branch whose PR closed at its head and that no item names, written first to TRASHED; a scratch directory of a life that no process and no item uses, older than 2 days; and a folder /tmp/furb-* older than 2 days. It tells you what went and the room that df saw freed.
threads() gives the tree of the threads: the brief prompt of each worker and the prompts under it, with their state, their PRs, their queued items and each question that waits.
debrief(name) asks the chain of a worker that ended for the frictions that it met in furb.
Each act of the round is made through quietly(verb, ...), so its end wakes nobody: you hear only a notice, and each merge rides on the next one. Make each act that you do not await the same way.
A review blocks only on a defect that a command reproduces, or on a false sentence of a contract."""

LIMITS = {"room": {"opus": 1, "sonnet": 3, "opus/high": 1}, "fixer": "claude-cli:sonnet/high"}
limits: dict = globals().setdefault("limits", LIMITS)
ACTOR = "claude-cli:opus/medium"
HANDOVER = "claude-cli:sonnet/medium"
MODELS = (HANDOVER, "claude-cli:sonnet/high", ACTOR, "claude-cli:opus/high")
DECISIONS = "DECISIONS.md"
NEWS = "No answer needed here. The newest line first.\n"
GATE = "the hooks"
FAILS = ("FAILURE", "ERROR", "TIMED_OUT", "STARTUP_FAILURE", "ACTION_REQUIRED")
GREEN = ("SUCCESS", "SKIPPED", "NEUTRAL")
RUNNING = ("", "PENDING", "QUEUED", "IN_PROGRESS", "WAITING", "REQUESTED", "EXPECTED")
UPSTREAM = "uael/monty"
TOOLING = ".furb/tooling.json"
ARCHIVE = ".furb/archive"
TRASHED = ".furb/trashed-branches.tsv"
queue: list[tuple[str, str, str]] = []
backlog: list[tuple[str, str, str]] = []
working: dict[str, str] = {}
results: dict[str, object] = {}
relays: dict[str, str] = {}
bases: dict[str, str] = {}
efforts: dict[str, str] = {}
homes: dict[str, str] = {}
models: dict[str, str] = {}
lead: dict[str, str] = {}
custom: dict[str, str] = {}
prs: dict[int, dict] = {}
merged: set[int] = set()
noticed: set[str] = set()
fixes: dict[str, float] = {}
silent: dict[str, tuple[int, float]] = {}
comments: set[str] = set()
measures: list[int] = []
timers: dict[str, float] = {}
noted: list[str] = []
debriefs: dict[str, str] = {}
vetoes: dict = {}
ORDERED = 600.0
ordered: dict[int, float] = {}
tips: dict[str, str] = {}
shifted: dict[int, tuple[str, str]] = {}
PACE = 3600.0
hired: dict[str, float] = {}
paced: set[str] = set()
PACED = (
  "An hour passed since you took this item, and no PR of yours is open. Land it in steps: commit now the coherent"
  " part that serves his words, open its PR with pr(), and go on with the rest in a next PR from this chain,"
  " each PR one change. A PR that waits for every detail lands nothing,"
  " and each step is judged and merged on its own."
)

BUILD_SH = """d=${FURB_BUILD_SLOTS:-/tmp/furb-build-slots}
mkdir -p "$d"
while :; do
  for i in 1 2 3; do
    if [ -f "$d/$i/pid" ] && ! kill -0 "$(cat "$d/$i/pid")" 2>/dev/null; then rm -rf "$d/$i"; fi
    if mkdir "$d/$i" 2>/dev/null; then
      echo $$ > "$d/$i/pid"
      trap 'rm -rf "$d/$i"' EXIT
      trap 'exit 130' INT TERM
      "$@"
      exit $?
    fi
  done
  sleep 2
done
"""

RECIPES = """Recipes that fixers found again and again:
- The SSH push is refused. Push over HTTPS with the credential helper of gh and a lease on the full sha of the remote branch: git -c credential.helper= -c credential.helper='!gh auth git-credential' push --force-with-lease=<branch>:<full sha> https://github.com/<owner>/<repo>.git HEAD:refs/heads/<branch>. A push to a URL moves no tracking ref, so fetch the branch after it.
- gh run view --log-failed refuses while the run still runs. Take the id of a job from gh run view <run> --json jobs, then read gh api repos/<owner>/<repo>/actions/jobs/<id>/logs | grep -E 'FAILED|panicked|error'.
- macOS has no timeout and no flock command, and BSD sed needs sed -i '' to edit in place. Edit a file with read and write, or with python3, and bound a command with the timeout of bash.
- A git commit runs the hooks, which take minutes: give it timeout=1800.
- The checkout at the root can stand behind origin/main and hold no build. Read and change code only in your worktree.
- A cause that a brief names is a hypothesis: check it in the log before you build on it."""

RULES = """Run no real life of a model. Never open a window on this machine, not even for a test: the owner works on it, and each window takes his mouse and his keyboard. Run a check of the app only on the runners of CI. Start each comment, review, issue and PR body that you write on GitHub with 🤖, since you write from the owner's account and his own words carry no mark; a title keeps its own rule. Never take off the label owner-merge: only the owner takes it off, and that is his approval to merge. The owner sleeps or is away: ask him nothing and wait for nothing of his. Take each decision yourself from the brief and the rulings, say it in your PR body, and go on. Ask the one who gave you this work on its chain only when it alone can decide: await prompt(str, question, on="{home}").""" + "\n\n" + RECIPES

WORKER = """You are a worker on the repository {repo}. Do one item, and open its PR yourself.
1. Make a worktree whose build starts warm: {repo}/script/worktree.sh {repo}-w-{key} w/{key}, then work there. Build on origin/main, and stack your branch on an open PR only when your change needs its code; say why in the body, since a stack lands one layer at a time. If that worktree stands already, work in it, rebased onto origin/main. Run each build of Rust through sh $TMPDIR/build.sh <command>, which runs three at once at most.
2. Change it as the AGENTS.md files of the repository say.
3. Run only the tests that your change touches.
4. Commit with the title of the PR as its message (a commit that the item brought keeps its own message, since the merge squashes them), and push the branch. Then open the PR, or update it when it stands, with pr(), which fills the template of the owner from what you give it and checks each of its rules: await pr(title, why, outcome, checks, breaking=[], fixes=0, where=<your worktree>, labels="{labels}"). title is type(area): What changed, 70 characters or fewer: the type is feat, fix, perf, or chore for a change that no user sees, and the area is the part that owns the change (engine, crate, tui, gui, provider, ci, conductor, or an extension by its name). why is the problem and why it matters now, from your own work and not from git log. outcome is a list: what is true now that was not, each with the evidence of what you ran, and each gap that you leave. breaking lists each change that forces a caller of a door or of the contract to change code. 300 words at most, and no how, which the diff holds. prcheck(title, why, outcome, breaking) before the commit checks the title and gives the word count. checks is six lines, one for each box of the checklist of the template in its order: verify each box one by one, then give the evidence of how you verified it, or not: and the work that it still needs, which leaves that box open. A flaw that you name in outcome ticks the box about flaws. When the item says that the owner ruled several changes one PR, that ruling is the evidence of the box about one change. A pr() that breaks a rule raises Refused and says why; never write the body of a PR by hand.
5. Open your first PR within about an hour of work, as soon as one coherent part serves the item: a large item
lands as a series of PRs, each one change, opened from this chain, and you close with the number of the last one.
6. Keep the worktree until your PR merges, since a fix comes back to it and a new worktree builds cold: the round prunes it after the merge. When the item asks for no change, answer it in a comment that starts with 🤖 on its issue or PR. Close with a tuple: the number of your PR, or 0 when the item asked for no change, and your report in 200 words at most: what you measured, what you decided, and what is left.
""" + RULES + """

The item, {key}:
{what}"""

FIX = """You fix the open PR #{n} of the repository {repo}: it {problem} at {sha}.{base} Read it with gh pr view, gh pr checks and gh run view --log-failed. Rerun once a check that fails on one system alone. Else work in a worktree of your own at the head of its branch, since its worker may hold another one (git worktree list shows it), and fetch before each push. If the head of the PR is not {sha} any more, or the PR no longer {problem}, change nothing and close with its number and a report that says so. A PR that conflicts runs no checks, so the checks that it shows may be old. Fix it: resolve a conflict with the meaning of both sides, and for a stack read skill("gh-stack") and use gh stack sync. Run the tests that it touches, push with git push --force-with-lease=refs/heads/<branch>:<the sha that you fetched> origin HEAD:refs/heads/<branch>, wait until gh pr view --json headRefOid,mergeable shows your head (GitHub shows the old head and the old mergeable for about 20 seconds after a push), keep the worktree until the PR merges, and close with a tuple: the number of the PR, and your report in 200 words at most. """ + RULES

FIXOWN = "Your open PR #{n} {problem} at {sha}. Fetch origin, rebase your branch onto its base or merge its base into it, resolve each conflict with the meaning of both sides, run the gates that the change touches, push with --force-with-lease, and keep its checks green."

DEBRIEF = """Your work is done. As a user of furb, list each friction, defect or confusion that you met in this chain, with its evidence and what would fix it. Close with a list of str, one for each, or an empty list."""

LABELS = "gh label create hold -c FBCA04 -d 'It waits for the fix of a review before a merge' 2>/dev/null; gh label create owner-merge -c 5319E7 -d 'The owner merges it or not' 2>/dev/null; gh label create paused -c BFBFBF -d 'The owner paused it: no fix and no merge' 2>/dev/null; true"

PRS = """gh pr list --state open --limit 200 --json number,isDraft,labels,mergeable,mergeStateStatus,baseRefName,headRefName,headRefOid,statusCheckRollup,updatedAt,body --jq '[.[] | ([.statusCheckRollup[]?] | group_by(.name // .context // "") | map(max_by(if (.status // "COMPLETED") == "COMPLETED" then (.startedAt // .completedAt // "") else "~" end))) as $c | {n: .number, draft: .isDraft, labels: [.labels[].name], mergeable, state: .mergeStateStatus, base: .baseRefName, head: .headRefName, sha: .headRefOid, checks: [$c[] | (.conclusion // .state // "")], names: [$c[] | (.name // .context // "")], open: [(.body // "") | splits("\\n") | select(startswith("- [ ] "))], idle: ((now - (.updatedAt | fromdateiso8601)) / 60 | floor)}]'
c=$?; echo; echo "--- $c"
gh pr list --state merged --limit 200 --json number --jq '[.[].number]'
c=$?; echo; echo "--- $c"
gh run list --branch main --workflow gates.yml --limit 20 --json headSha,conclusion --jq '.[0].headSha as $s | {sha: ($s // ""), runs: [.[] | select(.headSha == $s) | .conclusion]}'
c=$?; echo; echo "--- $c"
cat DECISIONS.md 2>/dev/null"""

REVIEWS = r"""for n in $(gh pr list --state open --json number --jq '.[].number') $(gh issue list --state open --json number --jq '.[].number'); do
gh api "repos/{owner}/{repo}/pulls/$n/comments" --jq '.[] | "\(.pull_request_url | split("/") | last)\tc\(.id)\t\(.user.login)\t\(.reactions["+1"] // 0)\t\(.path):\(.line // .original_line)\t\(.body | gsub("[\n\t]"; " "))"' 2>/dev/null
gh api "repos/{owner}/{repo}/issues/$n/comments" --jq '.[] | "\(.issue_url | split("/") | last)\ti\(.id)\t\(.user.login)\t\(.reactions["+1"] // 0)\t-\t\(.body | gsub("[\n\t]"; " "))"' 2>/dev/null
gh api "repos/{owner}/{repo}/pulls/$n/reviews" --jq '.[] | select(.body != "") | "\(.pull_request_url | split("/") | last)\tr\(.id)\t\(.user.login)\t0\t-\t\(.body | gsub("[\n\t]"; " "))"' 2>/dev/null
done"""


landed: list[int] = []
heard: list[str] = []
OWNER = "uael"
MARKS = ("🤖", "[furb")


def quietly(verb, *args, **words):
  got = []

  def ear(id):
    got.append(verb(*args, **words))
    yield "done", id, None

  act("quiet", "", ear)
  return got[0]


def merged_since() -> str:
  lines = [*heard, *(["Merged: " + " ".join(f"#{n}" for n in landed) + "."] if landed else [])]
  heard.clear()
  landed.clear()
  return ("\n\nSince the last notice:\n" + "\n".join(f"- {x}" for x in lines)) if lines else ""


PR_TITLE = r"(feat|fix|perf|chore)\([a-z0-9][a-z0-9:/_.-]*\)!?: [A-Z0-9`].*"
PR_BOXES = [
  "I am accountable for this diff, whoever or whatever wrote it.",
  "It is one change. No part belongs in another PR. No part could be its own PR.",
  "Its comments obey {rules}: none, or each one prevents a debugging session.",
  "Its prose obeys {rules}: Simplified Technical English, for a general non-technical audience.",
  "Every flaw I know of is fixed in this diff or named in `## Outcome`.",
  "I checked each statement above. I did not assume it.",
]


def pr_box(box: str, check: str) -> str:
  said = check.strip()
  if said.lower().startswith("not:"):
    return f"- [ ] {box}\n  Not yet: {said[4:].strip()}"
  return f"- [x] {box}"


def pr_body(
  why: str, outcome: list[str], breaking: list[str], fixes: int, checks: list[str], part_of: int = 0
) -> str:
  head = "🤖" + (f" Fixes #{fixes}." if fixes else "") + (f" Part of #{part_of}." if part_of else "")
  parts = [head, "## Why", why.strip(), "## Outcome", "\n".join(f"- {x.strip()}" for x in outcome)]
  if breaking:
    parts += ["## Breaking changes", "\n".join(f"- {x.strip()}" for x in breaking)]
  return "\n\n".join([*parts, "---", "\n".join(pr_box(b, c) for b, c in zip(PR_BOXES, checks))]) + "\n"


def prcheck(title: str, why: str, outcome: list[str], breaking: list[str] | None = None) -> int:
  if len(title) > 70 or not re.fullmatch(PR_TITLE, title):
    raise Refused(f"the title {title!r} is not type(area): What changed, in 70 characters or fewer")
  if not why.strip() or not outcome:
    raise Refused("a PR needs a why and at least one outcome")
  parts = [len(why.split()), len(" ".join(outcome).split()), len(" ".join(breaking or []).split())]
  if sum(parts) > 300:
    raise Refused(
      f"the body holds {sum(parts)} words, over 300 (why {parts[0]}, outcome {parts[1]}, breaking {parts[2]}):"
      " delete a whole paragraph, not a word"
    )
  return sum(parts)


PR_OPEN = (
  "{ B=$(git branch --show-current); N=$(gh pr list --state open --head \"$B\" --json number --jq '.[0].number // empty');"
  " out=$(if [ -n \"$N\" ]; then gh pr edit \"$N\" --title @NAME@ --body-file -@ADDED@ >/dev/null"
  " && gh pr view \"$N\" --json url --jq .url; else gh pr create --base @BASE@ --head \"$B\" --title @NAME@"
  " --body-file -@FLAGS@; fi); c=$?;"
  " if [ $c -ne 0 ]; then u=$(gh pr list --state open --head \"$B\" --json url --jq '.[0].url // empty' 2>/dev/null);"
  " [ -n \"$u\" ] && { echo \"$u\"; exit 0; }; fi; echo \"$out\"; exit $c; }"
)


def pr(
  title: str,
  why: str,
  outcome: list[str],
  checks: list[str],
  *,
  breaking: list[str] | None = None,
  fixes: int = 0,
  where: str = "",
  labels: str | list[str] = "",
  number: int = 0,
  base: str = "main",
  part_of: int = 0,
) -> Act[Exit]:
  prcheck(title, why, outcome, breaking)
  if len(checks) != len(PR_BOXES) or not all(c.strip() and c.strip().lower() != "not:" for c in checks):
    raise Refused(f"checks needs {len(PR_BOXES)} lines, one for each box of the checklist in its order: the evidence of how you verified that box, or not: and the work that it still needs")
  body = pr_body(why, outcome, breaking or [], fixes, checks, part_of)
  name = shq(title)
  flags = "".join(f" --label {shq(x)}" for x in labels) if isinstance(labels, list) else labels
  rules = 'R=CLAUDE.md; [ -f AGENTS.md ] && R=AGENTS.md; sed "s/{rules}/$R/g" | '
  if number:
    command = rules + f"gh pr edit {number} --title {name} --body-file - >/dev/null && gh pr view {number} --json url --jq .url"
  else:
    opening = PR_OPEN.replace("@NAME@", name).replace("@BASE@", shq(base))
    opening = opening.replace("@ADDED@", flags.replace("--label", "--add-label")).replace("@FLAGS@", flags)
    command = (f"cd {shq(where)} || exit 1; " if where else "") + rules + opening
  return bash(command, stdin=body, show=span(-2, -1), timeout=180)


PATCHER = "claude-cli:opus/medium"
CONDUCTOR = ".furb/extensions/conductor/conductor.py"
TRIES = 3
PATCH = """You change the conductor of furb, the word below, which is {path} on origin/main. The change: {ask}

Close with a list of (old, new) pairs of str, and edit no file: code applies the pairs, checks them, and opens the
PR. Each old is an exact part of the word that occurs in it once, with the lines around it that make it unique, and
its new is its text after the change. No two olds overlap. Change only what the change needs, add each name of the
engine that new code uses to the import of furb.engine at the top, and keep the style of the word: two spaces of
indentation, lines of 120 columns at most, and prose in Simplified Technical English with no em dash and no en dash.

The word:

{word}"""
CHECKED = """Code checked the pairs: each old occurs once, the gate and ruff find nothing new, and git diff holds the
pairs alone. Verify each box of the checklist of the PR template, one by one, against the diff below. Close with a
tuple of two lists of str. The first holds six str, one for each box in its order: the evidence of how you verified
it, or not: and the flaw that keeps it open. The second holds each flaw that the diff keeps and that no pair can
fix, one sentence each, which the PR names in its Outcome. Give a flaw that a pair can fix as not: in its box,
and the next prompt asks you for the pairs again.
{boxes}

{diff}"""
LOAD = (
  "The change is not in the running round: until onelife2 lands the reload of an extension, the orchestrator loads"
  " each changed definition by hand, as a rung on the chain of the round."
)
PROOF = (
  "Each old occurs once in the word of origin/main. The gate and ruff find nothing in the new word"
  " that they do not find in that word, and git diff holds the pairs alone. The commit skips the local"
  " hooks, which a worktree with no build cannot run: the gates of the PR run them."
)


def spliced(was: str, pairs: list[tuple[str, str]]) -> str:
  spots: list[tuple[int, int, str]] = []
  for old, new in pairs:
    if not old or old == new or was.count(old) != 1:
      count = was.count(old) if old else 0
      raise Refused(
        f"each old must occur once in the word and differ from its new, and this one occurs {count} times:"
        f" {old[:300]!r}"
      )
    spots.append((was.index(old), was.index(old) + len(old), new))
  out, end = "", 0
  for at, stop, new in sorted(spots):
    if at < end:
      raise Refused(f"two olds overlap at character {at} of the word")
    out, end = out + was[end:at] + new, stop
  return out + was[end:]


def changed(was: str, pairs: list[tuple[str, str]]) -> list[str]:
  heads = r"^(?:(?:async )?def |class )(\w+)|^(\w+)(?:: [^=\n]+)? = "

  def names(part: str) -> list[str]:
    return [a or b for a, b in re.findall(heads, part, re.M)]

  said: list[str] = []
  for old, new in pairs:
    at = was.index(old)
    near = names(was[:at])
    added = [x for x in names(new) if x not in names(old)]
    after = f", after {near[-1]}" if near else ""
    adds = f" and adds {', '.join(added)}" if added else ""
    line = was.count("\n", 0, at) + 1
    lines = [f"{n} line" + ("s" if n != 1 else "") for n in (len(old.splitlines()), len(new.splitlines()))]
    said.append(f"Line {line}{after}: the pair changes {lines[0]} into {lines[1]}{adds}.")
  return said


def linted(out: str) -> set[str]:
  found: set[str] = set()
  for x in out.splitlines():
    hit = re.match(r"\S+:\d+:\d+: (.+)", x)
    if hit:
      found.add(hit[1])
    elif re.match(r"[+-](?![+-]{2} )", x):
      found.add(x)
  return found


async def vetted(word: str, known: set[str], base: set[str], tree: str, lint: str) -> None:
  found = [x for x in gate(word) if re.sub(r"^line \d+: ", "", x) not in known]
  if found:
    raise Refused("the gate refuses the word:\n" + "\n".join(found))
  wrote = await bash(f"cd {tree} && cat > {CONDUCTOR} && git status --porcelain", stdin=word, show=HIDDEN, timeout=60)
  if wrote.stdout.content.strip() != f"M {CONDUCTOR}":
    raise Refused(f"git sees more than the pairs: {wrote.stdout.content[:300]}")
  fresh = sorted(linted((await bash(lint, show=HIDDEN, timeout=120)).stdout.content) - base)
  if fresh:
    raise Refused("ruff finds what it does not find in the word of origin/main:\n" + "\n".join(fresh)[:1500])


async def patch(title: str, why: str, ask: str) -> tuple[str, str, str]:
  if len(title) > 70 or not re.fullmatch(PR_TITLE, title):
    raise Refused(f"the title {title!r} is not type(area): What changed, in 70 characters or fewer")
  key = "patch-" + "-".join(re.findall(r"[a-z0-9]+", title.split(":", 1)[1].lower())[:4])
  place = f"{cwd()}-w-{key}"
  tree, ruff = shq(place), shq(f"{cwd()}/.venv/bin/ruff")
  lint = f"cd {tree} && {ruff} format --diff {CONDUCTOR} 2>/dev/null; {ruff} check --output-format concise {CONDUCTOR}"
  made = await bash(
    f"git worktree remove --force {tree} 2>/dev/null; git fetch -q origin main"
    f" && git worktree add -q {tree} -B w/{key} origin/main && cat {tree}/{CONDUCTOR}",
    show=HIDDEN,
    timeout=180,
  )
  if made.code != 0:
    raise Refused(f"no worktree {place}: {made.stderr.content[-500:]}")
  try:
    was = word = made.stdout.content
    known = {re.sub(r"^line \d+: ", "", x) for x in gate(was)}
    base = linted((await bash(lint, show=HIDDEN, timeout=120)).stdout.content)
    rules = (await bash(f"cd {tree} && ls AGENTS.md 2>/dev/null || echo CLAUDE.md", show=HIDDEN)).stdout.content
    listed = "\n".join(f"{n}. " + box.replace("{rules}", rules.strip()) for n, box in enumerate(PR_BOXES, 1))
    where = chain(f"patch/{key}")
    quietly(rung, f"actor = {PATCHER!r}", on=where)
    brief = PATCH.format(path=CONDUCTOR, ask=ask, word=was)
    pairs: list[tuple[str, str]] = []
    checks: list[str] = []
    gaps: list[str] = []
    for _ in range(TRIES + 1):
      got = await prompt(list[tuple[str, str]], brief, to=PATCHER, on=where)
      try:
        pairs = [(str(old), str(new)) for old, new in got]
        word = spliced(was, pairs)
        await vetted(word, known, base, tree, lint)
      except (Refused, TypeError, ValueError) as fault:
        brief = (
          f"Fault: {fault}\nThe pairs apply to the word of origin/main again."
          " Close again with the whole list of pairs, fixed."
        )
        continue
      diff = (await bash(f"cd {tree} && git diff", show=HIDDEN, timeout=60)).stdout.content
      answered = await prompt(
        tuple[list[str], list[str]], CHECKED.format(boxes=listed, diff=diff), to=PATCHER, on=where
      )
      checks, gaps = [str(x) for x in answered[0]], [str(x) for x in answered[1]]
      unticked = [b for b, c in zip(PR_BOXES, checks) if not c.strip() or c.strip().lower().startswith("not:")]
      if len(checks) == len(PR_BOXES) and not unticked:
        break
      brief = (
        f"Fault: give {len(PR_BOXES)} checks in the order of the boxes, and leave no box open:\n"
        + "\n".join(f"- {b}" for b in unticked)
        + "\nFix each flaw that a pair can fix, and name each other one as a gap. The pairs apply to the word of"
        " origin/main again. Close again with the whole list of pairs."
      )
    else:
      raise Refused(f"{where} gave no pairs and checks that pass in {TRIES + 1} answers. {brief}")
    pushed = await bash(
      f"cd {tree} && git commit -q --no-verify -am {shq(title)}"
      f" && (git fetch -q origin w/{key} 2>/dev/null; true) && git push -q --force-with-lease -u origin w/{key}"
      f" && (gh label create w:{key} --force >/dev/null 2>&1; true)",
      show=HIDDEN,
      timeout=180,
    )
    if pushed.code != 0:
      raise Refused(f"no push of w/{key}: {pushed.stderr.content[-500:]}")
    opened = await pr(title, why, [*changed(was, pairs), PROOF, *gaps], checks, where=place, labels=f" --label w:{key}")
    assert isinstance(opened, Exit)
    url = (opened.stdout.content.strip().splitlines() or [""])[-1]
    if opened.code != 0 or not url.startswith("https://"):
      raise Refused(f"no PR for w/{key}: {opened.stderr.content[-500:]}")
    return url, word, LOAD
  finally:
    quietly(bash, f"git worktree remove --force {tree}; git branch -D w/{key}", show=HIDDEN)


def cap() -> int:
  return sum(limits["room"].values())


def model_of(key: str) -> str:
  fixer = limits["fixer"] if re.fullmatch(r"(fix|box)\d+", key) else ACTOR
  return efforts.get(key, fixer)


def family(model: str) -> str:
  name = model.split(":")[-1]
  return name if name in limits["room"] else name.split("/")[0]


def room(model: str) -> bool:
  if model not in MODELS:
    return False
  mine = family(model)
  live = [n for n in working if not over(n) and family(models.get(n, ACTOR)) == mine]
  return len(live) < limits["room"][mine]


def can(key: str) -> bool:
  model = model_of(key)
  if model not in MODELS:
    notice(
      f"effort {key} {model}",
      f"The item {key} asks for {model}, and a worker runs only {', '.join(MODELS)}: xhigh and max stay for you and the rulings of the owner. Set efforts[{key!r}] to one of them.",
    )
    return False
  return room(model)


def handover(key: str, text: str) -> str:
  where = homes[key]
  quietly(rung, f"actor = {HANDOVER!r}", on=where)
  name = str(quietly(prompt, tuple[int, str], text, to=HANDOVER, on=where))
  working[name] = key
  models[name] = HANDOVER
  hired[name] = clock()
  return name


VIEW = """gh pr view {n} --json state,headRefOid,mergeable --jq '[.state, .headRefOid, .mergeable] | join(" ")'"""


def fixing(pr: dict) -> bool:
  return any(key in (f"fix{pr['n']}", f"box{pr['n']}") and not over(name) for name, key in working.items())


def warn(pr: dict, what: str) -> None:
  own = owner_of(pr)
  if own:
    quietly(prompt, None, f"{what} Fetch origin before you push: your worktree holds the old head.", on=working_at(own))


def crowd(pr: dict, own: str) -> None:
  if own:
    quietly(
      prompt,
      None,
      f"A fixer starts on your PR #{pr['n']}. Leave its branch alone until the fixer ends, and fetch origin before your next push.",
      on=working_at(own),
    )


def based(pr: dict) -> str:
  if pr["base"] == "main":
    return ""
  said = f" Its base is {pr['base']}"
  if pr["n"] in shifted:
    old, new = shifted[pr["n"]]
    said += f", whose head changed from {old} to {new}: git rebase --onto origin/{pr['base']} {old} {pr['head']} puts the PR on it"
  return said + "."


async def stale(pr: dict, conflict: bool = False) -> bool:
  got = await bash(VIEW.replace("{n}", str(pr["n"])), show=HIDDEN, timeout=120)
  parts = got.stdout.content.split()
  if got.code != 0 or len(parts) != 3:
    return False
  return parts[0] != "OPEN" or parts[1] != pr["sha"] or (conflict and parts[2] == "MERGEABLE")


def assist(pr: dict, key: str, order: str, fixer: str, thread: str, brief: str) -> bool:
  maker = next((m[2:] for m in pr["labels"] if m.startswith("w:")), "")
  if homes.get(maker) and maker not in working.values():
    if room(HANDOVER):
      noticed.update({key, key + " again", key + " fixer"})
      handover(
        maker,
        f"Your work on this chain goes on, now on {HANDOVER}. {order} Close with a tuple: the number of the PR, and your report in 200 words at most.",
      )
      return True
  elif can(fixer):
    noticed.update({key, key + " again", key + " fixer"})
    hire(fixer, "", thread, brief)
    return True
  return False


def follow() -> str:
  here = scope(acting())
  old = lead.get("home", "")
  lead["home"] = here
  if old and old != here:
    held = module(old).get("lead")
    if isinstance(held, dict):
      held["home"] = here
  return here


def hire(key: str, what: str, thread: str = "", brief: str = "") -> str:
  model = model_of(key)
  if model not in MODELS:
    raise Refused(
      f"{key} asks {model}, and a worker runs only {', '.join(MODELS)}: xhigh and max stay for the coordination and the rulings of the owner"
    )
  marks = [f"w:{key}", *([f"thread:{thread}"] if thread else [])]
  if not brief:
    quietly(bash, "; ".join(f"gh label create {m} --force >/dev/null 2>&1" for m in marks), show=HIDDEN)
  quietly(write, Text("scratch://build.sh", BUILD_SH))
  text = (custom.get(key) or brief or WORKER).replace("{labels}", "".join(f" --label {m}" for m in marks))
  text = text.replace("{repo}", cwd()).replace("{key}", key).replace("{home}", scope(acting())).replace("{what}", what)
  where = chain(f"{thread or 'w'}/{key}", source=bases.get(model, ""))
  quietly(rung, f"actor = {model!r}", on=where)
  name = str(quietly(prompt, tuple[int, str], text, to=model, on=where))
  working[name] = key
  models[name] = model
  hired[name] = clock()
  homes[key] = where
  return name


def debrief(name: str) -> str:
  return str(prompt(list[str], DEBRIEF, on=str(get(name)[3])))


def notice(key: str, message: str) -> None:
  if key not in noticed:
    noticed.add(key)
    quietly(prompt, None, message + merged_since())


def working_at(name: str) -> str:
  for _ in range(5):
    where = str(get(name)[3])
    facts = transcript(where)
    steps = [x[1] for x in facts if x[0] == "rung" and x[2] == name]
    awaited = [str(x[4]) for x in facts if x[0] == "wants" and steps and get(x[2])[4] == steps[-1]]
    if not awaited or over(awaited[-1]) or get(awaited[-1])[0] != "prompt" or get(awaited[-1])[3] == where:
      return where
    name = awaited[-1]
  return str(get(name)[3])


def owner_of(pr: dict) -> str:
  keys = [m[2:] for m in pr["labels"] if m.startswith("w:")] + [f"fix{pr['n']}", f"box{pr['n']}"]
  for name, key in working.items():
    if key in keys:
      return name
  for name in working:
    for where in {str(get(name)[3]), working_at(name)}:
      for x in transcript(where):
        command = str(x[4]) if x[0] == "bash" else ""
        if ("gh pr create" in command and f"pull/{pr['n']}" in str(peek(x[1]))) or (
          "push" in command and pr["head"] in command
        ):
          return name
  return ""


def ours() -> set[str]:
  return {
    line
    for x in transcript(ROOT, "write")
    if isinstance(t := peek(x[1]), Text) and t.path.endswith(DECISIONS)
    for line in t.lines
  }


def tend() -> None:
  now = clock()
  for orig, relay in list(relays.items()):
    if relay and over(relay):
      close(peek(relay), id=orig)
      relays[orig] = ""
  for name in [x for x in working if over(x)]:
    key = working.pop(name)
    got = peek(name)
    results[name] = got
    if f"debrief {name}" not in noticed and get(name):
      noticed.add(f"debrief {name}")
      debriefs[str(quietly(debrief, name))] = key
    if isinstance(got, tuple) and len(got) == 2 and isinstance(got[0], int) and got[0] > 0:
      heard.append(f"The worker {key} ({name}) ended with PR #{got[0]}, and results[{name!r}] keeps its report.")
    else:
      quietly(prompt, None, f"The worker {key} ({name}) ended with no PR:\n\n{str(got)[:2000]}" + merged_since())
  for asked, key in list(debriefs.items()):
    if over(asked):
      del debriefs[asked]
      said = peek(asked)
      if isinstance(said, list) and said:
        flaw(*[str(one) for one in said], source=f"debrief of {key}")
  for name, key in working.items():
    where = working_at(name)
    if paused(where):
      wake(where)
    facts = transcript(where)
    for x in [*facts, *(transcript(str(get(name)[3])) if where != get(name)[3] else [])]:
      if x[0] == "prompt" and x[6] == OPERATOR and not over(x[1]) and x[1] not in relays:
        relays[x[1]] = str(quietly(prompt, x[4], f"{x[3]} asks the operator:\n\n{x[5]}"))
    if silent.get(where, (-1, now))[0] != len(facts):
      silent[where] = (len(facts), now)
    elif now - silent[where][1] >= 2700:
      notice(
        f"silent {where} {len(facts)}",
        f"The worker {key} on {where} has said nothing for {int(now - silent[where][1]) // 60} minutes. Look at it.",
      )
  pace()


def pace() -> None:
  now = clock()
  owners = {owner_of(pr) for pr in prs.values()}
  for name in [x for x in working if not over(x) and x not in paced]:
    if name in owners:
      paced.add(name)
    elif now - hired.get(name, now) > PACE:
      paced.add(name)
      quietly(prompt, None, PACED, on=working_at(name))


def boxes(pr: dict, now: float) -> str:
  if not pr.get("open"):
    return ""
  n, sha = pr["n"], pr["sha"]
  key = f"#{n} {sha} open boxes"
  since = fixes.setdefault(key, now)
  own = owner_of(pr)
  if fixing(pr):
    return key
  order = (
    f"Your open PR #{n} has a box of its checklist open at {sha[:7]}:\n"
    + "\n".join(x[6:] for x in pr["open"])
    + f"\nDo the work that each open box names, verify it, then call pr(..., number={n}) again with the evidence of each box, so pr() ticks it. The round merges no PR with an open box."
  )
  if own and now - since < 3600:
    if key not in noticed:
      noticed.add(key)
      quietly(prompt, None, order, on=working_at(own))
  elif key + " fixer" not in noticed:
    thread = next((m[7:] for m in pr["labels"] if m.startswith("thread:")), "")
    brief = (
      f"You close the open boxes of the checklist of the open PR #{n} of the repository "
      + "{repo}. "
      + order.replace("Your open PR", "The PR")
      + " Read it with gh pr view, work in a worktree of its branch, push with --force-with-lease, keep the worktree until the PR merges, and close with a tuple: the number of the PR, and your report in 200 words at most. " + RULES
    )
    if assist(pr, key, order, f"box{n}", thread, brief):
      crowd(pr, own)
  return key


async def mend(pr: dict, now: float, red: bool) -> str:
  healed = await heal(pr, red)
  if healed:
    heard.append(healed)
    return ""
  n, sha = pr["n"], pr["sha"]
  problem = (
    "conflicts with its base"
    if pr["mergeable"] == "CONFLICTING"
    else "has a failed check"
    if pr["verdict"] == "fail"
    else ""
  )
  if not problem or fixing(pr):
    return ""
  key = f"#{n} {sha} {problem}"
  since = fixes.setdefault(key, now)
  own = owner_of(pr)
  owned = bool(own) and now - since < 3600
  second = owned and key in noticed and now - since >= 1800 and key + " again" not in noticed
  if (owned and not second and key in noticed) or (not owned and key + " fixer" in noticed):
    return key
  if n in ordered and now - ordered[n] < ORDERED:
    return key
  if await stale(pr, pr["mergeable"] == "CONFLICTING"):
    return ""
  note = based(pr)
  order = FIXOWN.replace("{n}", str(n)).replace("{problem}", problem).replace("{sha}", sha) + note
  if owned:
    noticed.add(key + " again" if second else key)
    quietly(prompt, None, ("Second notice, 30 minutes later: " if second else "") + order, on=working_at(own))
    ordered[n] = now
  else:
    thread = next((m[7:] for m in pr["labels"] if m.startswith("thread:")), "")
    brief = FIX.replace("{n}", str(n)).replace("{problem}", problem).replace("{sha}", sha).replace("{base}", note)
    if assist(pr, key, order, f"fix{n}", thread, brief):
      ordered[n] = now
      crowd(pr, own)
  return key


RESTACK = """pull() { git fetch -q origin "+refs/heads/$onto:refs/remotes/origin/$onto" "+refs/heads/$head:refs/remotes/origin/$head"; }
tries=0
while :; do
  pull || exit 1
  [ "$parent" = 0 ] && break
  want=$(gh pr view "$parent" --json mergeCommit --jq .mergeCommit.oid 2>/dev/null)
  [ -n "$want" ] && git merge-base --is-ancestor "$want" "origin/$onto" 2>/dev/null && break
  tries=$((tries + 1))
  [ "$tries" -ge 10 ] && { echo "$onto holds no merge of #$parent"; exit 1; }
  sleep 3
done
[ "$(git rev-parse "origin/$head")" = "$sha" ] || { echo "$head moved since the sweep"; exit 4; }
git merge-base --is-ancestor "$old" "origin/$head" || { echo "$head holds no $old"; exit 3; }
root=$PWD
dir=$(mktemp -d) || exit 1
trap 'cd "$root"; git worktree remove --force "$dir" >/dev/null 2>&1; rm -rf "$dir"' EXIT
git -c core.hooksPath=/dev/null worktree add -q --detach "$dir" "origin/$head" || exit 1
cd "$dir" || exit 1
git -c core.hooksPath=/dev/null rebase -q --onto "origin/$onto" "$old" >/dev/null 2>&1 || { git rebase --abort >/dev/null 2>&1; echo "$head conflicts with $onto"; exit 2; }
git push -q --no-verify --force-with-lease="refs/heads/$head:$sha" origin "HEAD:refs/heads/$head" || exit 5
"""


def spec(head: str) -> str:
  return shq(f"+refs/heads/{head}:refs/remotes/origin/{head}")


async def restack(old: str, onto: str, kid: dict, parent: int = 0) -> None:
  head = kid["head"]
  setup = f"onto={shq(onto)} head={shq(head)} old={shq(old)} sha={shq(kid['sha'])} parent={parent}\n"
  got = await bash(setup + RESTACK, show=HIDDEN, timeout=300)
  if got.code != 0:
    why = " ".join((got.stdout.content + got.stderr.content).split())[-300:]
    heard.append(f"#{kid['n']} was not rebased onto {onto}, exit {got.code}: {why}")
    return
  shifted.pop(kid["n"], None)
  warn(kid, f"The round rebased your PR #{kid['n']} onto {onto} and pushed it.")
  for deeper in [up for up in prs.values() if up["base"] == head]:
    await restack(kid["sha"], head, deeper)


def shift() -> None:
  seen = {pr["head"]: pr["sha"] for pr in prs.values()}
  for n, pr in prs.items():
    old, new = tips.get(pr["base"]), seen.get(pr["base"])
    if pr["base"] == "main" or not new:
      shifted.pop(n, None)
    elif old and old != new:
      shifted[n] = (shifted[n][0] if n in shifted else old, new)
  for n in [n for n in shifted if n not in prs]:
    del shifted[n]
  tips.clear()
  tips.update(seen)


async def probe(pr: dict) -> None:
  got = await bash(
    f"git fetch -q origin {spec(pr['base'])}; git fetch -q origin refs/pull/{pr['n']}/head;"
    f" git merge-tree --write-tree {shq('origin/' + pr['base'])} {shq(pr['sha'])} >/dev/null 2>&1; echo $?",
    show=HIDDEN,
    timeout=180,
  )
  if got.stdout.lines[-1:] == ["1"]:
    pr["mergeable"] = "CONFLICTING"


async def restack_forced() -> set[int]:
  moved: set[int] = set()
  for n, (old, new) in list(shifted.items()):
    kid = prs.get(n)
    parent = next((p for p in prs.values() if kid and p["head"] == kid["base"]), None)
    if not kid or not parent:
      shifted.pop(n, None)
      continue
    key = f"restack {n} {new}"
    if key in noticed or fixing(kid) or {"paused", "hold"} & set(kid["labels"]) or not kid["head"].startswith("w/"):
      continue
    noticed.add(key)
    got = await bash(
      f"git fetch -q origin {spec(parent['head'])}; git merge-base --is-ancestor {shq(old)} {shq(new)}; echo $?",
      show=HIDDEN,
      timeout=180,
    )
    if got.stdout.lines[-1:] == ["0"]:
      continue
    moved.add(n)
    await restack(old, parent["head"], kid)
  return moved


async def land(pr: dict, red: bool) -> None:
  n, sha = pr["n"], pr["sha"]
  if pr["verdict"] != "pass" or fixing(pr):
    return
  for veto in list(vetoes.values()):
    if await veto(pr):
      return
  same = await bash(
    f"git fetch -q origin main refs/pull/{n}/head 2>/dev/null; t=$(git rev-parse {sha}^{{tree}} 2>/dev/null) && [ \"$(git merge-tree --write-tree origin/main {sha} 2>/dev/null | head -1)\" = \"$t\" ] && echo same",
    show=HIDDEN,
    timeout=180,
  )
  if "same" not in same.stdout.content:
    got = await bash(
      f"gh api 'repos/{{owner}}/{{repo}}/compare/{sha}...main' --jq '.files[].filename' && echo --- && gh pr view {n} --json files --jq '.files[].path'",
      show=HIDDEN,
      timeout=120,
    )
    if got.code != 0 or "---\n" not in got.stdout.content:
      return
    moved, _, own = got.stdout.content.partition("---\n")
    if {x for x in moved.splitlines() if x and not x.endswith(".md")} & {x for x in own.splitlines() if x}:
      if not red and f"land update {sha}" not in noticed:
        noticed.add(f"land update {sha}")
        updated = await bash(f"gh pr update-branch {n}", show=HIDDEN, timeout=120)
        if updated.code == 0:
          warn(pr, f"The round updated the branch of your PR #{n} with main.")
      return
  kids = [up for up in prs.values() if up["base"] == pr["head"]]
  cuts = {}
  for up in kids:
    found = await bash(
      f"git fetch -q origin {spec(pr['head'])} {spec(up['head'])} && git merge-base {shq('origin/' + pr['head'])} {shq('origin/' + up['head'])}",
      show=HIDDEN,
      timeout=120,
    )
    cuts[up["n"]] = found.stdout.content.strip() if found.code == 0 and found.stdout.content.strip() else pr["sha"]
  moves = "".join(f"gh pr edit {up['n']} --base main >/dev/null 2>&1; " for up in kids)
  got = await bash(
    f"{moves}out=$(gh pr merge {n} --squash --delete-branch 2>&1) || out=\"$out $(gh stack merge {n} --yes --squash 2>&1)\";"
    f" sleep 5; echo \"$out\" | tail -4; gh pr view {n} --json state --jq .state",
    show=HIDDEN,
    timeout=400,
  )
  if got.stdout.lines[-1:] == ["MERGED"]:
    merged.add(n)
    landed.append(n)
    for up in kids:
      await restack(cuts[up["n"]], "main", up, n)
  else:
    said = "\n".join(got.stdout.lines[:-1])[-800:] or "nothing"
    notice(f"unmerged {sha}", f"#{n} did not merge at {sha[:7]}, and gh said:\n{said}")


async def review(seed: bool) -> None:
  got = await bash(REVIEWS, show=HIDDEN, timeout=600)
  rows = [p for p in (x.split("\t", 5) for x in got.stdout.content.splitlines()) if len(p) == 6 and p[1] not in comments]
  mine = [p for p in rows if p[2] == OWNER and not p[5].startswith(MARKS)]
  kinds = {"c": "pulls", "i": "issues"}
  others = [p for p in rows if p[2] != OWNER and p[3] not in ("0", "null", "") and p[1][0] in kinds]
  if others:
    asks = [
      "echo " + p[1] + " $(gh api repos/{owner}/{repo}/" + kinds[p[1][0]] + "/comments/" + p[1][1:]
      + "/reactions --jq '[.[] | select(.content == \"+1\") | .user.login] | index(\"" + OWNER + "\") != null' 2>/dev/null)"
      for p in others
    ]
    seen = await bash("; ".join(asks), show=HIDDEN, timeout=300)
    yes = {x.split(" ")[0] for x in seen.stdout.content.splitlines() if x.endswith(" true")}
    mine += [p for p in others if p[1] in yes]
  comments.update(p[1] for p in rows if p[2] == OWNER)
  comments.update(p[1] for p in mine)
  if seed or not mine:
    return
  told: list[str] = []
  held: set[int] = set()
  for p in mine:
    n = int(p[0]) if p[0].isdigit() else 0
    line = f"#{p[0]} {p[1]} {p[4]} by {p[2]}: {p[5][:600]}"
    if n in prs:
      held.add(n)
      own = owner_of(prs[n])
      what = "your PR, which now holds the label hold until you push its fix and take hold off"
    elif f"issue{n}" in handled:
      own = next((w for w, k in working.items() if k == f"issue{n}"), "")
      what = "your issue"
    else:
      continue
    if own:
      quietly(prompt, None, f"A new comment of the owner on {what}. Do what it asks, and answer it on GitHub with a reply that starts with 🤖.\n{line}", on=working_at(own))
    else:
      told.append(line)
  if held:
    await bash("".join(f"gh pr edit {n} --add-label hold >/dev/null 2>&1; " for n in sorted(held)), show=HIDDEN, timeout=300)
  if told:
    quietly(prompt, None, "New comments of the owner that no worker owns:\n" + "\n".join(told) + merged_since())


async def decide(title: str, text: str) -> str:
  repo = ops()[0]
  count = await bash(f"gh issue list --repo {shq(repo)} --label decision --state all --limit 500 --json number --jq length", show=HIDDEN, timeout=120)
  n = 1 + int(count.stdout.content.strip() or "0")
  body = "🤖 " + text + "\n\nKeep it with no answer, or comment here with your change or your veto."
  made = await bash(f"gh issue create --repo {shq(repo)} --label decision --title {shq(f'D{n}. {title}')} --body-file -", stdin=body, show=HIDDEN, timeout=120)
  return f"D{n} ({made.stdout.content.strip()})"


async def audit(red: bool) -> None:
  def waits(pr: dict | None) -> bool:
    for _ in range(20):
      if not pr:
        return False
      if "owner-merge" in pr["labels"] or not pr["head"].startswith("w/"):
        return True
      pr = next((p for p in prs.values() if p["head"] == pr["base"]), None)
    return False

  mine = [n for n, pr in prs.items() if not waits(pr)]
  owner = [n for n in prs if n not in mine]
  stale = [n for n in mine if prs[n]["idle"] >= 60]
  measures.append(len(queue) + len(working) + len(mine))
  def depth(pr: dict | None) -> int:
    d = 0
    for _ in range(20):
      pr = next((p for p in prs.values() if pr and p["head"] == pr["base"]), None)
      if not pr:
        break
      d += 1
    return d

  deep = sorted(n for n, pr in prs.items() if depth(pr) >= 2)
  issues = [
    text
    for text, bad in (
      ("main is red", red),
      (f"PRs that stand on two or more open PRs: {deep}", bool(deep)),
      (f"PRs with no change for an hour: {stale}", bool(stale)),
      (
        f"the measure did not go down in two audits: {measures[-3:]}",
        len(measures) >= 3 and measures[-1] >= measures[-3],
      ),
    )
    if bad
  ]
  ours = " ".join(f"#{n}" for n in mine) or "none"
  theirs = " ".join(f"#{n}" for n in owner) or "none"
  line = f"- Audit: open items {measures[-1]} (queued {len(queue)}, working {len(working)}, PRs {ours}); PRs that wait for the owner: {theirs}; main {'red' if red else 'green'}."
  news(line[2:])
  pruned = await prune()
  entries = sorted((x for x in ledger() if x.get("state") == "open"), key=lambda x: -int(x.get("count", 1)))
  flaws = "\n".join(f"- {x.get('count', 1)}x {str(x.get('what', ''))[:300]}" for x in entries[:10]) or "none"
  quietly(
    prompt,
    None,
    f"Audit of the work, with the measures {measures[-6:]}:\n{line}\nIssues: {'; '.join(issues) or 'none'}\n"
    f"The open defects of the tooling in {TOOLING}, {len(entries)} in all, the most repeated first:\n{flaws}\n"
    f"{pruned}\nCheck each worker and each PR against it, fix what is off, turn the worst defect of the tooling into"
    " the next item, and go on." + merged_since(),
  )


ISSUES = """gh issue list --state open --limit 100 --json number,title,body,author,assignees,labels,url --jq '[.[] | select(.author.login == "uael" or ([.assignees[].login] | index("uael"))) | {n: .number, title, body, url, labels: [.labels[].name]}]'"""

ISSUE = """The issue #{n} of the owner: {url}
Title: {title}

{body}

Read the issue whole with its comments first: gh issue view {n} --comments. Do what it asks. When it asks for a change, open the PR with Fixes #{n} in its body, so that the merge closes the issue. When it asks a question or a decision and no change, answer it in a comment on the issue, and close with that answer."""

handled: set[str] = set()


async def take_issues() -> None:
  got = await bash(ISSUES, show=HIDDEN, timeout=120)
  if got.code != 0 or not got.stdout.content.startswith("["):
    notice(f"issues {int(clock() // 3600)}", "The command that lists the issues failed: " + (got.stderr.content or got.stdout.content)[-500:])
    return
  for it in json.loads(got.stdout.content):
    key = f"issue{it['n']}"
    body = str(it["body"] or "")
    if key in handled or "paused" in it["labels"] or body.startswith(MARKS):
      continue
    handled.add(key)
    if key in working.values() or any(q[0] == key for q in queue) or any(f"w:{key}" in pr["labels"] for pr in prs.values()):
      continue
    brief = ISSUE.replace("{n}", str(it["n"])).replace("{url}", it["url"]).replace("{title}", it["title"])
    queue.append((key, brief.replace("{body}", body[:6000]), "issues"))
    heard.append(f"The issue #{it['n']}, {it['title']}, is queued as the item {key}.")
    quietly(bash, f"gh issue comment {it['n']} --body '🤖 Taken: a worker of furb does this as the item {key}, and its PR or its answer comes here.'", show=HIDDEN)


STATUS = {
  "s:queued": ("C5DEF5", "A worker of furb takes it when a slot frees"),
  "s:working": ("1D76DB", "A worker of furb does it now"),
  "s:checks": ("FBCA04", "Its checks run"),
  "s:red": ("D93F0B", "A check failed, and its worker fixes it"),
  "s:boxes": ("FEF2C0", "A box of its checklist is open, and its worker closes it"),
  "s:held": ("D4C5F9", "The label hold keeps it from a merge until it comes off"),
  "s:conflict": ("B60205", "It conflicts with its base, and its worker rebases it"),
  "s:stacked": ("BFD4F2", "Green, and lands after the PR under it"),
  "s:ready": ("0E8A16", "Green: the round merges it"),
  "s:owner": ("5319E7", "Green, and waits for the merge of the owner"),
}
statused: dict[int, str] = {}


async def mark() -> None:
  want: dict[int, str] = {}
  for n, pr in prs.items():
    if "paused" in pr["labels"]:
      continue
    want[n] = (
      "s:conflict"
      if pr["mergeable"] == "CONFLICTING"
      else "s:red"
      if pr["verdict"] == "fail"
      else "s:boxes"
      if pr["verdict"] == "open"
      else "s:held"
      if "hold" in pr["labels"]
      else "s:owner"
      if "owner-merge" in pr["labels"] or pr["draft"]
      else "s:stacked"
      if pr["base"] != "main"
      else "s:checks"
      if pr["verdict"] != "pass"
      else "s:ready"
    )
  for key in handled:
    n = int(key[5:])
    if n not in want:
      want[n] = "s:working" if key in working.values() else "s:queued" if any(q[0] == key for q in queue) else ""
  edits = []
  for n, s in want.items():
    if statused.get(n) != s:
      statused[n] = s
      edits.append(
        f"gh {'pr' if n in prs else 'issue'} edit {n}"
        + (f" --add-label {s}" if s else "")
        + "".join(f" --remove-label {x}" for x in STATUS if x != s)
        + " >/dev/null 2>&1"
      )
  if edits:
    await bash("; ".join(edits), show=HIDDEN, timeout=600)


PRIVATE = ".furb/extensions/conductor/private.json"
secret: dict[str, str] = {}
NEWS_PY = """import subprocess, sys
repo, digest, line = sys.argv[1:4]
got = subprocess.run(["gh", "issue", "view", digest, "--repo", repo, "--json", "body", "--jq", ".body"], capture_output=True, text=True, check=True)
head, _, rest = got.stdout.rstrip("\\n").partition("\\n\\n")
body = (head + "\\n\\n- " + line + ("\\n" + rest if rest else ""))[:60000]
subprocess.run(["gh", "issue", "edit", digest, "--repo", repo, "--body-file", "-"], input=body, text=True, check=True)
"""
OPS_WATCH = r"""gh api 'repos/{ops}/issues/comments?sort=created&direction=desc&per_page=100' --jq '.[] | "\(.issue_url | split("/") | last)\to\(.id)\t\(.user.login)\t\(.body | gsub("[\n\t]"; " "))"'
gh issue list --repo {ops} --state all --limit 100 --json number,author,title,body --jq '.[] | "\(.number)\tn\(.number)\t\(.author.login)\t\(.body | gsub("[\n\t]"; " ")) (title: \(.title))"'"""


def shq(s: str) -> str:
  return "'" + s.replace("'", "'\\''") + "'"


def ops() -> tuple[str, str]:
  if not secret:
    try:
      secret.update(json.loads(read(PRIVATE, HIDDEN).content))
    except (Refused, ValueError):
      return "", ""
  return secret.get("repo", ""), secret.get("digest", "")


def news(line: str) -> None:
  repo, digest = ops()
  if not repo:
    notice(f"private {int(clock() // 3600)}", f"{PRIVATE} names no private repository, so a line of news went nowhere: {line[:300]}")
    return
  lock = "/tmp/furb-conductor-news.lock"
  quietly(bash, f"until mkdir {lock} 2>/dev/null; do sleep 1; done; trap 'rmdir {lock}' EXIT; python3 -I - {shq(repo)} {shq(digest)} {shq(line)}", stdin=NEWS_PY, show=HIDDEN, timeout=300)


async def ops_watch() -> None:
  repo = ops()[0]
  if not repo:
    return
  got = await bash(OPS_WATCH.replace("{ops}", repo), show=HIDDEN, timeout=300)
  rows = [p for p in (x.split("\t", 3) for x in got.stdout.content.splitlines()) if len(p) == 4 and p[1] not in comments]
  comments.update(p[1] for p in rows)
  fresh = [p for p in rows if p[2] == OWNER and not p[3].startswith(MARKS)]
  if fresh and "ops" in timers:
    quietly(prompt, None, "The owner wrote in the private repository of the decisions:\n" + "\n".join(f"ops#{p[0]} {p[3][:1500]}" for p in fresh) + merged_since())
  timers["ops"] = clock()


REHEAD = """[ "$(gh pr view {n} --json headRefOid --jq .headRefOid)" = {sha} ] || exit 0
for id in $(gh run list --commit {sha} --limit 50 --json databaseId,workflowName,createdAt,conclusion --jq 'group_by(.workflowName) | map(max_by(.createdAt)) | .[] | select({which}) | .databaseId'); do gh run rerun $id{failed} >/dev/null 2>&1 && echo $id; done"""

BEHIND = """[ "$(gh api 'repos/{owner}/{repo}/compare/main...{sha}' --jq .behind_by 2>/dev/null)" -gt 0 ] 2>/dev/null && gh pr update-branch {n} >/dev/null 2>&1 && echo updated"""

REDLINES = r"""id=$(gh run list --branch main --workflow gates.yml --commit {sha} --limit 1 --json databaseId --jq '.[0].databaseId'); gh run view "$id" --log-failed 2>/dev/null | grep -E 'FAILED|panicked|Error|error\[|AssertionError|\(fail\)|timed out' | cut -f1,3 | head -30 | cut -c1-300"""

UPSTREAMING = """gh pr list --repo {upstream} --label upstream-ready --state open --json number,headRefName,title,url,labels --jq '.[] | select([.labels[].name] | index("upstreamed") | not) | "\\(.number)\\t\\(.headRefName)\\t\\(.title)\\t\\(.url)"'"""


def rehead(pr: dict, which: str, failed: str = "") -> str:
  return (
    REHEAD.replace("{n}", str(pr["n"])).replace("{sha}", pr["sha"]).replace("{which}", which).replace("{failed}", failed)
  )


async def heal(pr: dict, red: bool) -> str:
  n, sha = pr["n"], pr["sha"]
  failed = [c for c in pr["checks"] if c in FAILS]
  if pr["mergeable"] == "CONFLICTING":
    return ""
  if not failed:
    if "CANCELLED" not in pr["checks"] or f"rerun {sha}" in noticed:
      return ""
    noticed.add(f"rerun {sha}")
    got = await bash(rehead(pr, '.conclusion == "cancelled"'), show=HIDDEN, timeout=300)
    ran = got.stdout.content.strip()
    return f"#{n} had a run cancelled at its head {sha[:7]}, and the round ran it again." if ran else ""
  if (
    pr["base"] == "main"
    and not red
    and not pr["draft"]
    and not {"hold", "paused"} & set(pr["labels"])
    and not fixing(pr)
    and f"heal update {sha}" not in noticed
  ):
    noticed.add(f"heal update {sha}")
    got = await bash(BEHIND.replace("{sha}", sha).replace("{n}", str(n)), show=HIDDEN, timeout=180)
    if "updated" in got.stdout.content:
      warn(pr, f"The round updated the branch of your PR #{n} with main.")
      return f"#{n} failed at {sha[:7]} behind main, which is green, so the round updated its branch."
  if len(failed) == 1 and f"retry {sha}" not in noticed:
    noticed.add(f"retry {sha}")
    got = await bash(
      rehead(pr, '.conclusion == "failure" or .conclusion == "timed_out"', " --failed"), show=HIDDEN, timeout=300
    )
    if got.stdout.content.strip():
      return f"#{n} failed one check alone at {sha[:7]}, and the round ran its failed jobs again."
  return ""


def verdict(pr: dict) -> str:
  checks = pr["checks"]
  if any(c in FAILS for c in checks) or ("CANCELLED" in checks and f"rerun {pr['sha']}" in noticed):
    return "fail"
  if pr.get("open"):
    return "open"
  if GATE in pr.get("names", []) and all(c in GREEN for c in checks):
    return "pass"
  return "pending"


def needs(pr: dict) -> bool:
  return (
    pr["head"].startswith("w/")
    and not {"paused", "hold"} & set(pr["labels"])
    and (pr["verdict"] in ("fail", "open") or pr["mergeable"] == "CONFLICTING")
    and not owner_of(pr)
  )


def staff() -> None:
  if any(needs(pr) for pr in prs.values()):
    return
  while backlog and len(working) + len(queue) < cap():
    queue.append(backlog.pop(0))
  for item in list(queue):
    if can(item[0]):
      queue.remove(item)
      hire(*item)


async def upstream_watch() -> None:
  got = await bash(UPSTREAMING.replace("{upstream}", UPSTREAM), show=HIDDEN, timeout=120)
  for line in got.stdout.lines:
    n, head, title, url = [*line.split("\t"), "", "", ""][:4]
    if n and f"upstream-ready {n}" not in noticed:
      noticed.add(f"upstream-ready {n}")
      quietly(
        prompt,
        None,
        f"The owner put the label upstream-ready on the fork PR {url} ({head}: {title}). By his ruling, open its PR"
        f" on pydantic/monty from {UPSTREAM.split('/')[0]}:{head}, with the title and the body that he reviewed less"
        " its line of draft, then put the label upstreamed on the fork PR." + merged_since(),
      )


def ledger() -> list[dict]:
  try:
    got = json.loads(read(TOOLING, HIDDEN).content)
  except (Refused, ValueError):
    return []
  return [x for x in got if isinstance(x, dict)] if isinstance(got, list) else []


def flaw(*whats: str, source: str = "live", item: str = "") -> None:
  entries = ledger()
  for what in whats:
    plain = " ".join(what.lower().split())
    same = next((x for x in entries if " ".join(str(x.get("what", "")).lower().split()) == plain), None)
    if same:
      same["count"] = int(same.get("count", 1)) + 1
      same["state"] = "open"
    else:
      entries.append(
        {"when": str(int(clock())), "source": source, "what": what[:600], "state": "open", "item": item, "count": 1}
      )
  quietly(write, Text(TOOLING, json.dumps(entries, indent=1) + "\n"))


PRUNE_PY = """import hashlib, json, os, shutil, subprocess, sys, time

spec = json.load(open(sys.argv[1], encoding='utf-8'))
scratch = os.path.realpath(sys.argv[2])
root = os.path.realpath(spec['root'])
keys, text, tmp = set(spec['keys']), spec['text'], spec['tmp']
now, day, stamp = time.time(), time.strftime('%Y-%m-%d'), time.strftime('%H%M%S')
old = now - 2 * 86400
archive = os.path.join(root, spec['archive'], day)
gone = {'worktrees': [], 'branches': [], 'scratch': [], 'tmp': []}
archived, faults = [], []


def run(*args, cwd=root):
  got = subprocess.run(args, cwd=cwd, capture_output=True)
  return got.returncode, got.stdout.decode('utf-8', 'replace'), got.stderr.decode('utf-8', 'replace')


def named(word):
  return word in keys or word in text


def room():
  seen, total = set(), 0
  for path in (root, scratch, tmp):
    lines = run('df', '-Pk', path)[1].splitlines() if os.path.exists(path) else []
    fields = lines[-1].split() if len(lines) > 1 else []
    if len(fields) > 3 and fields[0] not in seen and fields[3].isdigit():
      seen.add(fields[0])
      total += int(fields[3]) * 1024
  return total


def age(path):
  try:
    times = [os.lstat(path).st_mtime]
    if os.path.isdir(path) and not os.path.islink(path):
      times += [x.stat(follow_symlinks=False).st_mtime for x in os.scandir(path)]
    return max(times)
  except OSError:
    return now


def drop(path):
  if os.path.isdir(path) and not os.path.islink(path):
    shutil.rmtree(path, ignore_errors=True)
  else:
    try:
      os.remove(path)
    except OSError:
      pass
  return not os.path.lexists(path)


def many(n, one, more):
  return f'{n} {one if n == 1 else more}'


def kept(name):
  os.makedirs(archive, exist_ok=True)
  archived.append(name)
  return os.path.join(archive, name)


before = room()
code, out, err = run('gh', 'pr', 'list', '--state', 'all', '--limit', '3000', '--json', 'number,state,headRefName,headRefOid,isCrossRepository')
if code != 0:
  print('The prune removed nothing, since gh listed no PR: ' + err.strip()[-200:])
  sys.exit(0)
newest = {}
for pr in json.loads(out or '[]'):
  name = pr['headRefName']
  if not pr.get('isCrossRepository') and (name not in newest or pr['number'] > newest[name]['number']):
    newest[name] = pr
run('git', 'fetch', '-q', '--prune', 'origin')
run('git', 'worktree', 'prune')

prefix = os.path.basename(root) + '-w-'
for block in run('git', 'worktree', 'list', '--porcelain')[1].split(chr(10) * 2):
  fields = dict(line.split(' ', 1) for line in block.splitlines() if ' ' in line)
  path = fields.get('worktree', '')
  base = os.path.basename(path)
  if os.path.dirname(path) != os.path.dirname(root) or not base.startswith(prefix):
    continue
  key, branch = base[len(prefix):], fields.get('branch', '')[len('refs/heads/'):]
  if not branch or named(key) or named(path) or named(branch):
    continue
  state = newest.get(branch, {}).get('state', '')
  code, out, err = run('git', 'cherry', 'origin/main', branch)
  ahead = [x for x in out.splitlines() if x.startswith('+')]
  if state == 'OPEN' or code != 0 or (ahead and state not in ('MERGED', 'CLOSED')):
    continue
  if run('git', 'status', '--porcelain', cwd=path)[1].strip():
    run('git', 'add', '-A', cwd=path)
    diff = subprocess.run(['git', 'diff', '--cached', '--binary', 'HEAD'], cwd=path, capture_output=True)
    if diff.returncode != 0:
      faults.append('no patch of ' + key)
      continue
    with open(kept(f'{key}-{stamp}.patch'), 'wb') as f:
      f.write(diff.stdout)
  if run('git', 'rev-list', '-1', branch, '--not', '--remotes')[1].strip():
    if run('git', 'bundle', 'create', kept(f'{key}-{stamp}.bundle'), branch, '--not', '--remotes')[0] != 0:
      faults.append('no bundle of ' + key)
      continue
  if run('git', 'worktree', 'remove', '--force', path)[0] != 0:
    faults.append('no removal of ' + key)
    continue
  run('git', 'branch', '-D', branch)
  gone['worktrees'].append(key)

default = run('git', 'symbolic-ref', '--short', 'refs/remotes/origin/HEAD')[1].strip().split('/', 1)[-1]
code, out, err = run('git', 'ls-remote', '--heads', 'origin')
for line in out.splitlines() if code == 0 else []:
  sha, _, ref = line.partition(chr(9))
  name = ref[len('refs/heads/'):]
  pr = newest.get(name)
  if not pr or name in ('main', default) or pr['state'] == 'OPEN' or pr['headRefOid'] != sha:
    continue
  if named(name) or (name.startswith('w/') and name[2:] in keys):
    continue
  try:
    with open(os.path.join(root, spec['trashed']), 'a', encoding='utf-8') as f:
      f.write(chr(9).join([day, name, sha, str(pr['number']), pr['state']]) + chr(10))
  except OSError:
    faults.append('no line in ' + spec['trashed'])
    break
  if run('git', 'push', '-q', 'origin', '--delete', name)[0] == 0:
    gone['branches'].append(name)

held = [line[1:] for line in run('lsof', '-n', '-F', 'n')[1].splitlines() if line.startswith('n' + os.sep)]
held += [x for line in run('ps', '-A', '-o', 'command=')[1].splitlines() for x in line.split() if x.startswith(os.sep)]
used = set()
for path in held:
  for one in {path, os.path.realpath(path)}:
    used.add(hashlib.sha256(one.encode()).hexdigest()[:16])
    if one.startswith(scratch + os.sep):
      used.add(one[len(scratch) + 1:].split(os.sep)[0])
if not held:
  faults.append('lsof saw no open file, so no scratch directory and no folder of ' + tmp + ' went')
else:
  for entry in os.scandir(scratch) if os.path.isdir(scratch) else []:
    if entry.is_dir(follow_symlinks=False) and entry.name not in used and not named(entry.name):
      if not named(entry.path) and age(entry.path) < old and drop(entry.path):
        gone['scratch'].append(entry.name)
  reals = [os.path.realpath(x) for x in held]
  for name in os.listdir(tmp) if os.path.isdir(tmp) else []:
    path = os.path.join(tmp, name)
    real = os.path.realpath(path)
    if not name.startswith('furb-') or named(path) or age(path) >= old:
      continue
    if not any(x == real or x.startswith(real + os.sep) for x in reals) and drop(path):
      gone['tmp'].append(name)

freed = max(room() - before, 0)
parts = []
if gone['worktrees']:
  parts.append('the worktrees ' + ', '.join(gone['worktrees']) + ', with their local branches')
if gone['branches']:
  parts.append(many(len(gone['branches']), 'remote branch', 'remote branches') + ', listed in ' + spec['trashed'])
if gone['scratch']:
  parts.append(many(len(gone['scratch']), 'scratch directory', 'scratch directories'))
if gone['tmp']:
  parts.append(many(len(gone['tmp']), 'folder', 'folders') + ' of ' + tmp)
said = 'The prune removed ' + '; '.join(parts) + f', and df saw {freed / 2 ** 30:.1f} GiB freed.' if parts else 'The prune found nothing to remove.'
if archived:
  said += f' {archive} keeps their work: ' + ', '.join(archived) + '.'
if faults:
  said += ' Faults: ' + '; '.join(faults) + '.'
print(said)
"""


async def prune() -> str:
  items = [*queue, *backlog]
  spec = {
    "root": cwd(),
    "keys": sorted({*working.values(), *(one[0] for one in items)}),
    "text": "\n".join([*(one[1] for one in items), *custom.values(), *(str(get(x)[5]) for x in working if get(x))]),
    "archive": ARCHIVE,
    "trashed": TRASHED,
    "tmp": "/tmp",
  }
  quietly(write, Text("scratch://prune.json", json.dumps(spec)))
  got = await bash(
    's=$(dirname "$TMPDIR"); [ "$(basename "$s")" = scratch ] || s=/nonexistent; python3 -I - "$TMPDIR/prune.json" "$s"',
    stdin=PRUNE_PY,
    show=HIDDEN,
    timeout=1800,
  )
  return (got.stdout.lines[-1:] or [f"The prune failed: {got.stderr.content[-300:]}"])[0]


MANY = 3
budgeted: dict[str, list[int]] = {}
flaky: dict[str, list[str]] = {}
runs_read: set[int] = set()
CI_PY = r'''import json, re, statistics, subprocess, sys, time
from datetime import datetime
read = set(json.loads(sys.argv[1]))
def gh(*args):
  for wait in (2, 8, 30, 0):
    got = subprocess.run(["gh", *args], capture_output=True, text=True)
    if got.returncode == 0:
      return got.stdout
    if not wait or "HTTP 5" not in got.stderr:
      sys.exit(f"gh {' '.join(args[:2])} failed: {got.stderr.strip()[-400:]}")
    time.sleep(wait)
def jobs(run, attempt=0):
  at = f"/attempts/{attempt}" if attempt else ""
  return json.loads(gh("api", f"repos/{{owner}}/{{repo}}/actions/runs/{run}{at}/jobs?per_page=100", "--jq", "[.jobs[] | {id, name, conclusion, started_at, completed_at}]"))
def spent(job):
  stamp = lambda s: datetime.fromisoformat(s.replace("Z", "+00:00")).timestamp()
  return int(stamp(job["completed_at"]) - stamp(job["started_at"]))
budgets = json.loads(subprocess.run(["git", "show", "origin/main:.github/budgets.json"], capture_output=True, text=True).stdout or "{}")
path = "repos/{owner}/{repo}/actions/workflows/gates.yml/runs"
green = json.loads(gh("api", f"{path}?branch=main&status=success&per_page=5", "--jq", "[.workflow_runs[].id]"))
measured = {}
for run in green:
  for job in jobs(run):
    if job["conclusion"] == "success":
      measured.setdefault(job["name"], []).append(spent(job))
over = [{"job": n, "median": int(statistics.median(v)), "budget": budgets[n], "runs": green} for n, v in measured.items() if n in budgets and len(v) >= 3 and statistics.median(v) > budgets[n]]
runs = json.loads(gh("api", f"{path}?per_page=30", "--jq", "[.workflow_runs[] | {id, sha: .head_sha, attempt: .run_attempt, conclusion, url: .html_url}]"))
flakes, settled = [], []
for run in runs:
  if run["attempt"] < 2 or run["conclusion"] != "success":
    continue
  settled.append(run["id"])
  if run["id"] in read:
    continue
  for attempt in range(1, run["attempt"]):
    for job in jobs(run["id"], attempt):
      if job["conclusion"] not in ("failure", "timed_out"):
        continue
      log = subprocess.run(["gh", "api", f"repos/{{owner}}/{{repo}}/actions/jobs/{job['id']}/logs"], capture_output=True, text=True).stdout
      tests = []
      for line in log.splitlines():
        line = re.sub(r"\x1b\[[0-9;]*m", "", re.sub(r"^\S+Z ", "", line))
        hit = re.match(r"\(fail\) (.+?)(?: \[[\d.]+m?s\])?$", line) or re.match(r"FAILED (\S+)", line) or re.match(r"test (\S+) \.\.\. FAILED$", line)
        if hit and hit[1] not in tests:
          tests.append(hit[1])
      flakes.append({"tests": tests, "job": job["name"], "sha": run["sha"], "url": f"{run['url']}/attempts/{attempt}"})
print(json.dumps({"over": over, "flakes": flakes, "read": settled}))
'''

BUDGET = """The job {job} of the gates took {median} s in the median of its last green runs on main, {runs}, over its budget of {budget} s in .github/budgets.json, so each loop of feedback waits longer for it. Find where the job spends its time, from the times of its steps in gh run view --json jobs and from its logs, and bring the job under its budget at the cause: a cache that misses, a build done twice, or a test that waits. Never drop or weaken a gate to save time: move it, share it or make it faster. Change a budget only with the measures of script/budgets.py, and say why in the PR."""

FLAKE = """The gates failed, then passed when they ran again, on the same commit {sha}, so a test or a job is flaky, and a flake hides a defect and slows each loop of feedback. In the job {job}, at {url}, it failed in:
{tests}
The ledger of the flakes holds every attempt that failed this way: {seen}.
Read the log of the attempt that failed: gh run view <run> --attempt <attempt> --log-failed. Find the cause, and prove it with a command that shows it, before and after your fix. {many}Fix it at its cause: never with a skip, a retry, a longer bound or a weaker assertion."""

MANY_SAID = "More than {n} tests failed in one attempt, so one cause, as a machine that is slow or a shared resource that waits, probably holds them all: find it first. "


def slug(text: str, size: int = 34) -> str:
  return "-".join(re.findall(r"[a-z0-9]+", text.lower()))[:size].strip("-")


def taken(key: str) -> str:
  return next((w for w, k in working.items() if k == key), "") or ("queued" if any(q[0] == key for q in queue) else "")


async def watch_ci() -> None:
  got = await bash(f"python3 -I - {shq(json.dumps(sorted(runs_read)))}", stdin=CI_PY, show=HIDDEN, timeout=900)
  if got.code != 0 or not got.stdout.content.startswith("{"):
    notice(f"ci {int(clock() // 3600)}", "The command that measures the gates failed: " + (got.stderr.content or got.stdout.content)[-500:])
    return
  seen = json.loads(got.stdout.content)
  runs_read.clear()
  runs_read.update(seen["read"])
  seed = "ci" not in timers
  timers["ci"] = clock()
  for one in seen["over"]:
    job, ran = str(one["job"]), [int(r) for r in one["runs"]]
    key = "budget-" + slug(job)
    if taken(key) or set(ran) & set(budgeted.get(job, [])):
      continue
    budgeted[job] = ran
    brief = BUDGET.format(job=job, median=one["median"], runs=", ".join(map(str, ran)), budget=one["budget"])
    queue.append((key, brief, "ci"))
    heard.append(f"The job {job} took {one['median']} s, over its budget of {one['budget']} s: queued as the item {key}.")
  for one in seen["flakes"]:
    tests = [str(x) for x in one["tests"]]
    many = len(tests) > MANY
    name = str(one["job"]) if many or not tests else tests[0]
    flaky.setdefault(name, []).append(str(one["url"]))
    if seed:
      continue
    key = "flake-" + slug(name)
    listed = "\n".join(f"- {x}" for x in tests) or "- the job itself, with no test named in its log"
    brief = FLAKE.format(
      sha=one["sha"], job=one["job"], url=one["url"], tests=listed, seen=", ".join(flaky[name]),
      many=MANY_SAID.format(n=MANY) if many else "",
    )
    own = taken(key)
    if own == "queued":
      continue
    if own:
      quietly(prompt, None, f"Your flake came again, at {one['url']}:\n{listed}", on=working_at(own))
      continue
    queue.append((key, brief, "ci"))
    heard.append(f"The gates flaked at {one['url']} in {name[:120]}: queued as the item {key}.")


async def sweep(every: float) -> None:
  got = await bash(PRS, show=HIDDEN, timeout=300)
  bits = re.split(r"\n--- (\d+)\n", got.stdout.content)
  steps = ("the open PRs", "the merged PRs", "the gates of main")
  whole = len(bits) == 2 * len(steps) + 1
  failed = [
    s for s, code, out in zip(steps, bits[1::2], bits[0::2], strict=False) if not whole or code != "0" or not out.strip()
  ]
  if not whole or failed:
    notice(
      f"list {int(clock() // 3600)}",
      f"The command that lists the PRs failed at {', '.join(failed) or 'its output'}: "
      + (got.stderr.content.strip()[-500:] or "gh said nothing."),
    )
    return
  parts = [bits[0], bits[2], bits[4], bits[6]]
  prs.clear()
  timers["sweep"] = clock()
  for pr in json.loads(parts[0]):
    pr["verdict"] = verdict(pr)
    prs[pr["n"]] = pr
  merged.update(json.loads(parts[1]))
  shift()
  for one in prs.values():
    if one["mergeable"] == "UNKNOWN":
      await probe(one)
  restacked = await restack_forced()
  main = json.loads(parts[2])
  red = "failure" in main["runs"]
  if red and f"red {main['sha']}" not in noticed:
    failed = await bash(REDLINES.replace("{sha}", str(main["sha"])), show=HIDDEN, timeout=300)
    notice(
      f"red {main['sha']}",
      f"main is red at {str(main['sha'])[:7]}. Find its cause first. The lines that failed:\n"
      + ("\n".join(failed.stdout.lines) or "none that the filter knows: read gh run view --log-failed"),
    )
  now = clock()
  live = set()
  for n in sorted(prs):
    pr = prs[n]
    if "paused" in pr["labels"]:
      continue
    if not pr["head"].startswith("w/"):
      if f"foreign #{n}" not in noticed:
        noticed.add(f"foreign #{n}")
        heard.append(
          f"#{n} ({pr['head']}) was not made by a worker of the round, so the round neither mends nor merges it."
        )
      continue
    if n in restacked:
      continue
    live.add(await mend(pr, now, red))
    box = f"#{n} {pr['sha']} open boxes"
    if pr["open"] and (box not in noticed or now - fixes.get(box, now) >= 3600) and await stale(pr):
      continue
    live.add(boxes(pr, now))
    if "owner-merge" in pr["labels"]:
      continue
    stuck = pr["verdict"] == "pending" and not any(c in RUNNING for c in pr["checks"])
    calm = pr["base"] == "main" and not pr["draft"] and "hold" not in pr["labels"]
    if pr["idle"] >= 45 and calm and stuck and not owner_of(pr):
      checks = ", ".join(f"{a} {b}" for a, b in zip(pr["names"], pr["checks"])) or "none"
      notice(
        f"idle {pr['sha']}",
        f"#{n} has had no change for {pr['idle']} minutes, no check of it runs and no worker owns it, so the round"
        f" cannot land it. Its checks at {pr['sha'][:7]}: {checks}. Move it on, or say why it waits.",
      )
    if not (pr["draft"] or "hold" in pr["labels"] or pr["base"] != "main"):
      await land(pr, red)
  for key in [k for k in fixes if k not in live]:
    del fixes[key]
  if parts[3] and noted and parts[3] != noted[0]:
    old = set(noted[0].splitlines()) | ours()
    added = [x for x in parts[3].splitlines() if x.strip() and x not in old]
    if added:
      quietly(prompt, None, "The owner changed DECISIONS.md:\n" + "\n".join(added[:60]))
  if parts[3]:
    noted[:] = [parts[3]]
  await take_issues()
  await upstream_watch()
  await ops_watch()
  await mark()
  await watch_ci()
  if now - timers.get("review", now - every) >= every:
    await review("review" not in timers)
    timers["review"] = now
  if now - timers.setdefault("audit", now) >= 12 * every:
    await audit(red)
    timers["audit"] = now


def faulted(here: str, step: str, fault: Exception) -> None:
  notice(
    f"fault {step} {type(fault).__name__}",
    f"The round on {here} met a fault in its step {step}: {type(fault).__name__}: {str(fault)[:1000]}. The round goes"
    " on at the next round, and tells this kind of fault once.",
  )


def ended(here: str, cause: str) -> None:
  try:
    quietly(
      prompt,
      None,
      f"The round on {here} ended: {cause}. Decide whether it goes on: until you await conduct() again, no sweep, no"
      " heal, no merge, no fixer and no hire runs, and no worker is tended." + merged_since(),
    )
  except Exception:
    return


async def conduct(every: float = 300) -> dict[str, object]:
  here, at = follow(), "labels"
  try:
    made = "".join(f"gh label create {s} --force -c {c} -d {shq(d)} >/dev/null 2>&1; " for s, (c, d) in STATUS.items())
    await bash(made + LABELS, show=HIDDEN)
    while lead.get("home") == here:
      for at in ("tend", "sweep", "staff"):
        try:
          if at == "tend":
            tend()
          elif at == "sweep":
            await sweep(every)
          else:
            staff()
        except Exception as fault:
          faulted(here, at, fault)
      if not (
        queue
        or backlog
        or working
        or any("owner-merge" not in pr["labels"] and "paused" not in pr["labels"] for pr in prs.values())
      ):
        ended(here, "it holds no item in queue, in backlog or in working, and no open PR that it tends")
        return results
      at = "wait"
      await wait(every, *working, *[x for x in relays.values() if x])
  except CancelledError:
    ended(here, f"it was cancelled in its step {at}")
    raise
  except Exception as fault:
    ended(here, f"the fault {type(fault).__name__}: {str(fault)[:1000]}, in its step {at}")
    raise
  return results


def threads() -> str:
  chains, todo = [], [ROOT]
  while todo:
    one = todo.pop()
    chains.append(one)
    todo += [str(x[1]) for x in transcript(one) if x[0] == "chain"]
  asks = [x for c in chains for x in transcript(c) if x[0] == "prompt" and x[4] != "None"]
  tops = {str(x[1]): x for x in asks if (get(x[3]) or ("", "", ""))[2] == x[2]}
  paths: dict[str, str] = {}
  kids: dict[str, list[str]] = {}
  waits: dict[str, list[str]] = {}
  out: list[str] = []

  def up(of: str) -> str:
    of = str((get(of) or ("", "", ""))[2])
    while of and of not in tops:
      of = str((get(of) or ("", "", ""))[2])
    return of

  def path(p: str) -> str:
    if p not in paths:
      label, parent = str(get(tops[p][3])[4]), up(p)
      paths[p] = label if "/" in label or not parent else path(parent) + "/" + label
    return paths[p]

  def links(p: str) -> list[str]:
    found: list[str] = []
    told_text = str(peek(p, "")) + "".join(str(x[3]) for x in transcript(str(tops[p][3])) if x[0] == "tell")
    for part in told_text.split("https://github.com/")[1:]:
      bits = part.split("/", 4)
      digits = ""
      for ch in bits[3] if len(bits) > 3 and bits[2] == "pull" else "":
        if not ch.isdigit():
          break
        digits += ch
      url = "https://github.com/" + "/".join(bits[:3]) + "/" + digits
      if digits and url not in found:
        found.append(url)
    return found[:5]

  def state_of(url: str) -> str:
    n = int(url.rsplit("/", 1)[1])
    if n in prs:
      pr = prs[n]
      return f"open {pr['verdict']}" + (" draft" if pr["draft"] else "") + "".join(" " + m for m in pr["labels"])
    return "unseen" if "sweep" not in timers else "merged" if n in merged else "closed"

  def draw(p: str, depth: int) -> None:
    pad, where = "  " * depth, str(tops[p][3])
    state = "done" if over(p) else "paused" if paused(where) else "working"
    out.append(f"{pad}{path(p)} {state} {p} on {where}")
    out.extend(f"{pad}  PR {u} {state_of(u)}" for u in links(p))
    out.extend(f"{pad}  waits {w}" for w in waits.get(p, []))
    out.extend(f"{pad}  {thread}/{key} queued" for key, _, thread in queue if thread == path(p))
    for q in kids.get(p, []):
      draw(q, depth + 1)

  for p in tops:
    hits = [q for q in tops if q != p and path(p).startswith(path(q) + "/")]
    best = max(hits, key=lambda q: len(path(q))) if hits else "" if "/" in str(get(tops[p][3])[4]) else up(p)
    kids.setdefault(best, []).append(p)
  for x in asks:
    if str(x[1]) not in tops and not over(x[1]):
      waits.setdefault(up(str(x[1])), []).append(f"{x[1]} on {x[3]} to {x[6] or 'its model'}: {str(x[5])[:200]}")
  for p in kids.get("", []):
    draw(p, 0)
  out.extend(f"{thread or 'w'}/{key} queued" for key, _, thread in queue if thread not in paths.values())
  out.extend(f"waits {w}" for w in waits.get("", []))
  return "\n".join(out)
