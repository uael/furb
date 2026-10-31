import json
import re
from asyncio import CancelledError
from typing import Any

from furb.engine import (
  HIDDEN,
  OPERATOR,
  ROOT,
  Exit,
  Refused,
  Text,
  act,
  acting,
  bash,
  cancel,
  chain,
  clock,
  close,
  cwd,
  filled,
  gate,
  get,
  module,
  over,
  pause,
  paused,
  peek,
  prompt,
  read,
  rung,
  say,
  scope,
  transcript,
  wait,
  wake,
  write,
)

CONDUCTING = """Conduct the work in one loop, and plan no further than the next step.
Put each item in queue as (key, brief, thread), or in backlog to wait behind queue, then await conduct(). Each round tends the workers and the open PRs, hires a worker for each item, cap() at once, and waits every seconds or until a worker ends. It hires no item of a family of model while an open PR in trouble has no worker and needs one of that family. So the fixers take the free slots of their family first, the round merges before it starts new work there, and each other family hires on its own room. It tells you once when such a PR waits for a fixer whose family has no room at all. It moves the first item of backlog to queue while queue and working hold fewer than cap() items.
When you find a defect, a wrong state or a gap in your own work, or the owner points at one, fix the state at once, then make its durable fix at once: a PR, through patch() for the word of the conductor, or an item at the head of the queue. Never keep it as a note for later, and never fix it only with a live patch. When a message of the owner asks whether a state is right, that is a sign that it is not: read the whole state, list each thing that is off, fix all of them in one go, read the state again to verify it, and do this again until nothing is off. Only then report, with what you verified.
The owner rules, on 10-23: "this kind of item should be done by sonnet, not opus. You need to use Sonnet more, when it make sense. And opus/medium should be your default, not opus/high. opus/high is kinda the max, xhigh for the coordination and my rulings only. most of the time sonnet/{medium,high} and opus/high are perfect." So a debrief asks the model of its worker, and a worker runs efforts[key], or the model of its kind. An item has a kind, kinds[key], by the ruling of the owner on 10-28 on dedicated audit and edit workers: audit reads and writes a register in scratch, with no worktree and no PR, on ACTOR, opus/medium, with the brief AUDIT; edit makes a worktree and a PR, on EDITOR, sonnet/high, with the brief WORKER; fix mends one red PR on limits["fixer"]. An item with no kind is a fix when its key is fixN or boxN, else an edit. An auditor that ends with 0 and its report is a line in the next notice. A fixer (fixN, boxN) runs on limits["fixer"], sonnet/high, by the ruling of the operator. A worker runs only a model of MODELS: sonnet/medium, sonnet/high, opus/medium or opus/high. hire() refuses xhigh and max, and the round starts no item that asks for one and tells you. You choose sonnet for an item where it makes sense, as efforts[key] = "claude-cli:sonnet/medium" before you queue it.
The owner rules again, on 10-23: "From now on: 2 opus workers, 3 sonnet ones. max." The owner rules again, on 10-26: "add one opus/high worker for the most complex stuff". So limits["room"] gives the room of each family of model: the round runs at most 1 worker on opus/medium, 3 on sonnet and 1 on opus/high at once, by the ruling of the operator, and counts only the workers whose prompt is not over. limits lives in the state of the round, so a reload of the extension keeps it: you change it there, as limits["room"]["opus"] = 2, and cap() gives the sum of the room. You give opus/high only to the most complex items, as efforts[key] = "claude-cli:opus/high". An item whose model has no room keeps its place in queue while the next item whose model has room starts. A fixer goes before a new item. A worker that ended and whose PR needs a fixer goes on in its own chain on HANDOVER, sonnet, with a new prompt there, when sonnet has room, else on the model of its last prompt when that has room; else a fixer starts on limits["fixer"].
The round runs on one chain, lead["home"]. After a compaction of your chain, await conduct() in the new chain: it names that chain, the round of the old chain ends at its next wake, and the state that the round reads is the state of the new chain, which you change there.
A fault of one step of a round, as an empty answer of gh, ends no round: the round tells you once each kind of fault, with the fault and its step, and goes on at the next round. Each json.loads of a command reads its output only when the command succeeded. When the round ends for any cause but a new home, it tells you as a decision, with the cause and what stops.
hire(key, brief, thread) gives the item to an idle worker of the pool, by the ideas of the owner on 10-28, when one fits: a worker that ended stays in pool under its chain, with its model, its thread and the top folders of the files of its PR, and it takes an item of the same model whose thread is its own, or whose areas[key] names one of its folders, while its last reply filled under half of its window. Else hire starts a worker on a new chain that forks the base chain of its kind and model, bases["kind model"], which the round makes once and which reads AGENTS.md, so the provider serves that prefix from its cache. It starts the worker at model_of(key), on a chain labeled thread/key, with the brief in custom or the brief of its kind, and the PR of an edit carries the labels w:key and thread:thread.
A worker does one item in a workspace {repo}-wN, which the round gives at its hire to it alone and again to a later worker once it ends, and which it leaves clean with its branch pushed (a chain goes on in the workspace that it held last when no live worker holds it, else it gets a new one and its brief says that the old one now belongs to another worker, and the file furb-holder in the git dir of a workspace names its chain, which each worker reads before its first command of a prompt there), runs only the tests that its change touches, opens its PR, and closes with its URL.
A prompt of a worker that ends with CancelledError while the round made no cancel over it, as when the life restarts, goes on: the round asks it again on its chain with a new prompt, at most 3 times for an item, and tells nobody.
The round tells you a worker that ends with no PR, with its answer, or that says nothing for 45 minutes, relays to you each question that a worker puts to the operator, and wakes a worker that paused. results keeps the answer of each worker under its prompt.
When a worker ends, the round asks it debrief() once, and adds each friction that it gives to TOOLING, the ledger of the defects of the tooling, as flaw(what) adds a defect that you find or patch live. An entry counts each time that it comes again. settle_flaw(part, pr) closes each open entry that holds part with the PR that fixed it, once you checked on main that the PR did; the audit lists the open entries alone, and an entry that comes again opens again.
A check counts by its newest run, and the round acts on a failed check once no check of the PR runs, or at once when two failed. A failed check heals with no worker first, once for each head sha: the round reruns the failed jobs of the run whose failure is the one failed check of the PR, which costs the runners of CI the least, else it updates the branch of a PR on main that is behind main while main is green, at most once in UPDATED seconds for a PR, since a PR that still fails after an update fails on its own. It gives a PR on main whose head has no run of the gates one, once for each head sha: it updates the branch of the PR while main is green, else it reruns the workflows at the head. It reruns once, for each head sha, a run that was cancelled at the head of a PR, and a check cancelled again at that sha counts as failed.
Each sweep puts each open PR of the round in one state of a table: green, running (its checks run, or one failed check waits for the end of its run), red (two failed checks, or one once no check runs, or an open box), conflicting, main (while main is red, each line of its failed checks fails on main too), stacked, or owner (it waits on the owner). A red or conflicting PR that holds no label paused or hold, and is not a draft whose maker lives, has one worker assigned at its head, in assigned[n] with the time of the assignment. A PR in trouble with no live assignee gets one in that sweep: its maker, the live worker of its label w:key or of owns, once for each head; else a fixer. A maker whose head did not move in OWNED seconds, 15 minutes, gives the PR to a fixer. A fixer that ends with no new head is replaced at once, twice at most for a head. A sweep that ends with a PR in trouble and no assignee tells you once, as a defect of the round, with its cause. A PR in the state main gets no order, no handover and no fixer, since no change of it can fix what main fails: the notice of a red main names such PRs, and once main is green the round updates their branches. A draft whose worker ended is tended as any PR in trouble, and only its merge waits. The order and the brief of a fixer are built from a fresh read at the moment of the order: the failed checks by name with their first failed lines, the commits of main that changed a file of the PR (with the files that they delete or move), and for a stacked PR the merge ref of its checks and the rebase command that fits the base now (a plain rebase when the base only advanced, rebase --onto from the commit that the branch shares with the old base when it was rewritten), with the commits that the base gained. A conflict of w/gallery gets no order for GALLERY_WAIT seconds, since the gallery workflow builds the branch again from main. Just before it gives an order or hires a fixer, the round reads the PR again and gives none when the head moved or the conflict is gone, and it names in an order the base of a stacked PR with the head that the base had before. While a fixer is assigned to a PR, the round neither updates its branch nor merges it, and it tells the maker when it updates or rebases the branch, or when a fixer starts on it. The round closes a PR that changes no file. A mergeable that gh gives as UNKNOWN is probed with git merge-tree. The round rebases a child of a PR whose head was force-pushed onto the new head of its parent, and a child whose parent merged outside the round onto main, from the fork point when the child holds no head of the parent that the round saw.
The round merges each green PR into main from the lowest number when git merges it with main clean and no file that a gate sums (SERIAL: the engine under its wall of tokens, the budgets) changed both in it and on main since its branch forked, after it moves the PRs that stack on it onto main and rebases each there with git rebase --onto from the head that it held of the PR that merged, in a scratch worktree, and pushes it with --force-with-lease (a rebase that conflicts is left to the fixer); else it updates the branch, and only when main is green.
The label paused takes a PR out of the round. A draft, the label hold, a review that waits for its fix, and the label owner-merge, until the owner takes it off as his approval, keep a PR from a merge.
A notice wakes you only for a decision, with the context that it needs: each event that needs none, as a worker that ends with a PR, a heal, a merge or a PR made outside the round, is one line in the next notice. The round tells you when main is red, with the lines that failed, when a PR fails to merge, with what gh said, or when a PR on main with no draft, no hold, no worker and no check that runs has no change for 45 minutes, with its checks, and each comment or issue of the owner in the private repository of the decisions, which PRIVATE names beside this word and git does not hold. await decide(title, text) opens a decision there as an issue, and news(line) adds a line at the top of its digest issue.
Each round queues each open issue that the owner opened, or that he assigned to himself, as an item issueN, once, and its PR closes it. It queues an open issue again, once for each worker that ended on it with a PR, when no item of it works or waits and no open PR with the label w:issueN holds it, with the PRs that ended it in its brief.
Each round tells you once each fork PR of UPSTREAM that holds the label upstream-ready and not upstreamed: by the ruling of the owner, you then open its PR on pydantic/monty with the body that he reviewed, and put the label upstreamed on the fork PR.
Each round sends each new comment of the owner on a PR or on his issue to the worker that owns it, puts the label hold on that PR, and tells you only the comments that no worker owns. A text on GitHub counts only when the owner wrote it with no 🤖 at its start, or when he put a 👍 on it; each worker starts what it writes there with 🤖.
Each round sets one status label on each open PR and on each issue of the owner that it took: s:queued, s:working, s:checks, s:red, s:boxes, s:held, s:conflict, s:stacked, s:ready, or s:owner (green, waits for the owner).
Before it hires, each round folds into one item each queued item that is like an earlier one, by the ruling of the owner on 10-29: the same thread, kind and model, and the same PR or a brief whose long lines are half the same. The earlier item keeps its key and its place, takes the lines of the later one that it lacks, and covers its key, so a flake, an order or a step of converge for it goes to its worker. Each round reads the gates: when the median time of a job in its last five green runs on main passes its budget in .github/budgets.json, which script/budgets.py sets from measures, it queues an item budget-<job>, once until five new runs measure the job; and when the gates fail, then pass on the same commit, it puts each test that failed in the ledger of the flakes, flaky, and at once queues an item flake-<test>, or tells the worker that holds that item. An item names in covering[key] the tests, the measures and the keys of the items of flakes or budgets that it covers, as covering["tui-bench-new-act"] = ["flake-bench-a-new-act-growth"]: taken() gives its worker for each of them, so a flake or a budget that it covers goes to that worker and queues no item.
Every twelfth round audits the work against its measure, the open items, writes its line in the digest, and tells you the open entries of TOOLING, the most repeated first: turn the worst into the next item. The audit prunes what the work left behind, and loses no work: a worktree of the round whose item is not in working, queue or backlog and whose branch merged, closed, or holds nothing that main lacks, or whose work is all committed and pushed, with its local branch, once its changes that are not committed are a patch and its commits that only this machine holds are a bundle, in the folder of the day in ARCHIVE; a remote branch whose PR closed at its head and that no item names, written first to TRASHED; a scratch directory of a life that no process and no item uses, older than 2 days; and a folder /tmp/furb-* older than 2 days. It tells you what went and the room that df saw freed.
threads() gives the tree of the threads: the brief prompt of each worker and the prompts under it, with their state, their PRs, their queued items and each question that waits.
debrief(name) asks the chain of a worker that ended for the frictions that it met in furb.
Each act of the round is made through quietly(verb, ...), so its end wakes nobody: you hear only a notice, and each merge rides on the next one. Make each act that you do not await the same way.
An item that takes a PR by its brief, and not by a label or a push, names it in owns[key] = [n, ...], so that the round sees its worker as the maker of that PR: the maker gets the order first, and a fixer takes the PR after OWNED seconds.
At each round a watchdog reads each open command of each worker, with no model: a wait on CI (gh pr checks --watch, gh run watch, or a loop of sleep around gh) that runs over 10 minutes is cancelled, and its reason tells the worker to stay on its PR, wait for the checks again with await checked(number) and close only when the PR merged. Any other command whose output did not grow for 15 minutes is a line in the next notice, and at 30 minutes it is cancelled with its reason and told to you. It ends a wait or a command, never a task. It also reads every chain of the life, not the workers alone: when a wait or a run of a word has held a chain for HELD seconds while a prompt of the operator on it is not over and no reply of a rung of it was said, the round cancels that wait (else that run, and says done with CancelledError when the cancel left it open), tells the chain once to await nothing over 60 s and to peek at the next advance, and names the chain in the next notice. It leaves the home of the round alone.
DEBRIEFS is on by default and lives in the state of the round: DEBRIEFS = False in a word turns off the debrief of each worker that ends, which saves tokens.
An item waits for others with after[key] = [keys], set before you queue it: staff() hires it only once each PR that those keys ended with merged, an item of backlog whose keys merged goes to queue, and the round lists in a notice each key that no item holds. The entries of held_items in the module of the root move into backlog at each round, so a dict of your own chain is not needed.
Method, at each notice: read the whole state, fix every thing that is off in one go, read the state again to verify, and go on until nothing is off. Only then report. Each round also checks the state in invariants(): it gives each open PR that the round tends a home, from the chain that pushed its head, wakes no chain, and lists in the next notice what code cannot fix.
A PR waits on a decision of the owner while awaited[decision] names one of its labels, or its number as #N, and the decision is an act that is not over, as the prompt that asked him: a failed check of that PR gets no order, no handover and no fixer, a conflict of it still does, and the round names the decision once. When the act ends, the round drops it and tends the PR again.
The round holds at most cap() chains that hold an open prompt with a shape, and counts the chains of workers, of sub-chains and of judges alike: room() counts them, and a resume of a worker waits for a slot. When more chains stand, the round pauses the newest overflow with a note and wakes it in order when a slot frees, and it never ends its task.
The owner rules, on 10-29: "we should use more gh stacks in conductor, because they can be merged at once (all PRs of stack as once)". So a PR on main at the bottom of a stack whose layers are all green, free of hold, paused and owner-merge, and passed by the vetoes, merges with its whole stack at once through gh stack merge, and while a layer above it still runs its checks, the bottom waits rather than merge alone; a stack with a red layer merges its green layers below it at once; a stack that gh stack does not hold lands one layer at a time, and the round tells it once. Each sweep links each chain of stacked PRs on GitHub with gh stack link, once for each chain, so that it shows as one stack. GitHub gives no priority to a queued run, so each sweep cancels each queued run of the gates that can serve no merge: a run of a closed PR, of an old head, of a draft, a held or a conflicting PR, and it defers a run of a layer above a layer that is not green, which it runs again once that layer is green and fewer than THROTTLE runs wait. It never touches a run of main or of a PR that can merge.
reload(path, on) loads the definitions of the file of an extension into the module of a chain: it runs, in the order of the file, each top-level definition whose name the module lacks or whose text differs from its text in the last run of a word that ended with no fault (a load that was cancelled or failed counts for nothing, so the next reload loads it again), and gives (rung, names): the rung that runs the load, which quietly starts so that no end of the caller cancels it, and the names. The load is done only when that rung is over and peek(rung) is no exception: await it. Use it after a PR that changed the conductor merged.
A review blocks only on a defect that a command reproduces, or on a false sentence of a contract."""

LIMITS = {"room": {"opus": 1, "sonnet": 3, "opus/high": 1}, "fixer": "claude-cli:sonnet/high"}
limits: dict[str, Any] = globals().setdefault("limits", LIMITS)
ACTOR = "claude-cli:opus/medium"
HANDOVER = "claude-cli:sonnet/medium"
EDITOR = "claude-cli:sonnet/high"
MODELS = (HANDOVER, EDITOR, ACTOR, "claude-cli:opus/high")
KINDS = {"audit": ACTOR, "edit": EDITOR}
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
# The module of a chain may hold these names already, with values: a name binds its default only when it is not there.
queue: list[tuple[str, str, str]] = globals().setdefault("queue", [])
backlog: list[tuple[str, str, str]] = globals().setdefault("backlog", [])
working: dict[str, str] = globals().setdefault("working", {})
results: dict[str, object] = globals().setdefault("results", {})
relays: dict[str, str] = globals().setdefault("relays", {})
bases: dict[str, str] = globals().setdefault("bases", {})
efforts: dict[str, str] = globals().setdefault("efforts", {})
homes: dict[str, str] = globals().setdefault("homes", {})
models: dict[str, str] = globals().setdefault("models", {})
lead: dict[str, str] = globals().setdefault("lead", {})
custom: dict[str, str] = globals().setdefault("custom", {})
prs: dict[int, dict] = globals().setdefault("prs", {})
merged: set[int] = globals().setdefault("merged", set())
noticed: set[str] = globals().setdefault("noticed", set())
assigned: dict[int, dict] = globals().setdefault("assigned", {})
silent: dict[str, tuple[int, float]] = globals().setdefault("silent", {})
comments: set[str] = globals().setdefault("comments", set())
measures: list[int] = globals().setdefault("measures", [])
timers: dict[str, float] = globals().setdefault("timers", {})
noted: list[str] = globals().setdefault("noted", [])
debriefs: dict[str, str] = globals().setdefault("debriefs", {})
briefed: dict[str, str] = globals().setdefault("briefed", {})
owns: dict[str, list[int]] = globals().setdefault("owns", {})
watched: dict[str, list[float]] = globals().setdefault("watched", {})
after: dict[str, list[str]] = globals().setdefault("after", {})
parked: dict[str, dict] = globals().setdefault("parked", {})
awaited: dict[str, list[str]] = globals().setdefault("awaited", {})
covering: dict[str, list[str]] = globals().setdefault("covering", {})
kinds: dict[str, str] = globals().setdefault("kinds", {})
pool: dict[str, dict] = globals().setdefault("pool", {})
slots: dict[str, str] = globals().setdefault("slots", {})
areas: dict[str, list[str]] = globals().setdefault("areas", {})
DEBRIEFS = globals().get("DEBRIEFS", True)
vetoes: dict = globals().setdefault("vetoes", {})
OWNED = 900.0
HELD = 120.0
GALLERY = "w/gallery"
GALLERY_WAIT = 6 * 3600.0
tips: dict[str, str] = globals().setdefault("tips", {})
shifted: dict[int, tuple[str, str]] = globals().setdefault("shifted", {})
parents: dict[int, str] = globals().setdefault("parents", {})
# The lines that fail on a red main, by its sha, and for each PR the head and the main that it was read against and
# whether each line of its failed checks fails on that main too.
redmain: dict[str, list[str]] = globals().setdefault("redmain", {})
onmain: dict[int, tuple[str, str, bool]] = globals().setdefault("onmain", {})
hired: dict[str, float] = globals().setdefault("hired", {})
paced: set[str] = globals().setdefault("paced", set())

BUILD_SH = """d=${FURB_BUILD_SLOTS:-/tmp/furb-build-slots}
note=${FURB_BUILD_NOTE:-60}
mkdir -p "$d"
start=$(date +%s)
last=$start
while :; do
  for i in 1 2 3; do
    if [ -f "$d/$i/pid" ] && ! kill -0 "$(cat "$d/$i/pid")" 2>/dev/null; then rm -rf "$d/$i"; fi
    if mkdir "$d/$i" 2>/dev/null; then
      echo $$ > "$d/$i/pid"
      date +%s > "$d/$i/since"
      trap 'rm -rf "$d/$i"' EXIT
      trap 'exit 130' INT TERM
      "$@"
      exit $?
    fi
  done
  now=$(date +%s)
  if [ $((now - last)) -ge "$note" ]; then
    last=$now
    echo "build.sh: waited $((now - start)) s for a slot; the slots are held by:"
    for i in 1 2 3; do
      p=$(cat "$d/$i/pid" 2>/dev/null) || continue
      s=$(cat "$d/$i/since" 2>/dev/null) || s=$now
      echo "  slot $i: pid $p, held $((now - s)) s: $(ps -o command= -p "$p" 2>/dev/null | cut -c1-200)"
    done
  fi
  sleep 2
done
"""

RECIPES = """Recipes that fixers found again and again:
- The SSH push is refused. Push over HTTPS with the credential helper of gh and a lease on the full sha of the remote branch: git -c credential.helper= -c credential.helper='!gh auth git-credential' push --force-with-lease=<branch>:<full sha> https://github.com/<owner>/<repo>.git HEAD:refs/heads/<branch>. A push to a URL moves no tracking ref, so fetch the branch after it.
- gh run view --log-failed refuses while the run still runs. Take the id of a job from gh run view <run> --json jobs, then read gh api repos/<owner>/<repo>/actions/jobs/<id>/logs | grep -E 'FAILED|panicked|error'.
- macOS has no timeout and no flock command, and BSD sed needs sed -i '' to edit in place. Edit a file with read and write, or with python3, and bound a command with the timeout of bash.
- A git commit runs the hooks, which take minutes: give it timeout=1800.
- The checkout at the root can stand behind origin/main and hold no build. Read and change code only in your worktree.
- A cause that a brief names is a hypothesis: check it in the log before you build on it.
- python script/tokens.py prints the tokens of the engine and of the doctrine against their walls in seconds, and counts no stale copy: after a change of engine.py, run cargo check first when it asks."""

RULES = """Run no real life of a model. Never open a window on this machine, not even for a test: the owner works on it, and each window takes his mouse and his keyboard. Run a check of the app only on the runners of CI. Start each comment, review, issue and PR body that you write on GitHub with 🤖, since you write from the owner's account and his own words carry no mark; a title keeps its own rule. Never take off the label owner-merge: only the owner takes it off, and that is his approval to merge. The owner sleeps or is away: ask him nothing and wait for nothing of his. Take each decision yourself from the brief and the rulings, say it in your PR body, and go on. Ask the one who gave you this work on its chain only when it alone can decide: await prompt(str, question, on="{home}"). Set each timeout and each wait to about twice the time that the step should take, never a round bound in case, and make a long step print its progress, so that a hang shows at once. Never wait on CI in a bash for more than 10 minutes: not gh pr checks --watch, not gh run watch, not a loop of sleep around gh. To wait for the checks of your PR, await checked(number) and never write a loop of your own: it reads them once a minute for 9 minutes at most, and gives pass, fail or pending with the failed checks and their first failed lines. Rerun a check only when its line says [machine], a fault of the runner or the network, or [cancelled], once for each head, with gh run rerun <run> --failed, and then await checked(number) again; a [defect] is yours to fix. Once your push is done, read the checks once and close with the PR and its checks as they stand, since the round tends them.""" + "\n\n" + RECIPES

WORKER = """You are a worker on the repository {repo}. Do one item, and open its PR yourself.
1. Your branch is w/{key}. {workspace} Build on origin/main, and stack your branch on an open PR only when your change needs its code, and say why in the body. Make a stack with gh stack, and run skill("gh-stack") in a word of your own before you make or touch one. Link its PRs on GitHub with gh stack submit --auto or gh stack link: the round merges a green gh stack whole, all its PRs at once. Run each build of Rust through sh $TMPDIR/build.sh <command>, which runs three at once at most.
2. Change it as the AGENTS.md files of the repository say.
3. Run only the tests that your change touches.
4. Commit with the title of the PR as its message (a commit that the item brought keeps its own message, since the merge squashes them), and push the branch. Then open the PR, or update it when it stands, with pr(), which fills the template of the owner from what you give it and checks each of its rules: await pr(title, why, outcome, checks, breaking=[], fixes=0, where=<your worktree>, labels="{labels}"). title is type(area): What changed, 70 characters or fewer: the type is feat, fix, perf, or chore for a change that no user sees, and the area is the part that owns the change (engine, crate, tui, gui, provider, ci, conductor, or an extension by its name). why is the problem and why it matters now, from your own work and not from git log. outcome is a list: what is true now that was not, each with the evidence of what you ran, and each gap that you leave. breaking lists each change that forces a caller of a door or of the contract to change code. 300 words at most, and no how, which the diff holds. prcheck(title, why, outcome, breaking) before the commit checks the title and gives the word count. checks is six lines, one for each box of the checklist of the template in its order: verify each box one by one, then give the evidence of how you verified it, or not: and the work that it still needs, which leaves that box open. A flaw that you name in outcome ticks the box about flaws. When the item says that the owner ruled several changes one PR, that ruling is the evidence of the box about one change. A pr() that breaks a rule, or whose gh fails, raises Refused and says why; never write the body of a PR by hand.
5. Open your first PR within about an hour of work, as soon as one coherent part serves the item: a large item
lands as a gh stack of PRs, each layer one change, opened from this chain, and you close with the number of the last one.
6. A fix comes back to your branch in any workspace, and the audit prunes nothing that a branch on origin holds. When the item asks for no change, answer it in a comment that starts with 🤖 on its issue or PR. Close with a tuple: the number of your PR, or 0 when the item asked for no change, and your report in 200 words at most: what you measured, what you decided, and what is left.
""" + RULES + """

The item, {key}:
{what}"""

WORKSPACE = """Work in the workspace {repo}-{slot}, one of the workspaces w1, w2 and on that the round gives again from item to item, so that each build starts warm. If it does not stand, make it: {repo}/script/worktree.sh {repo}-{slot} <your branch>. If it stands, it holds the branch of an earlier item: run git status there, commit what no commit holds as a WIP commit on that branch and push it, then git fetch origin and git switch -C <your branch> onto origin/<your branch> when that branch is on origin, else onto origin/main. Leave the workspace clean when you close, each change committed and pushed, since your branch is on origin and the round gives the workspace to the next item. Your chain stands in the directory of the main checkout until a cd moves it, and a relative path of read, write or bash acts there: once the workspace stands, give cd({repo}-{slot}) in a word of your own, and use no relative path before it. The git dir of a workspace holds a file furb-holder that names the chain that holds it. Once the workspace stands, write your chain there: echo {chain} > "$(git -C {repo}-{slot} rev-parse --absolute-git-dir)/furb-holder". Before your first command of each prompt in a workspace, read that file: if it names another chain than {chain}, run nothing in that workspace, since the round gave it to another worker, and use the workspace that your brief names. A holder that names a chain with no live prompt of a worker, which threads() shows, is stale: the round writes your chain there itself when it gives you the workspace, and if a stale name stays, write your chain over it and go on."""

MOVED = "Your earlier workspace {repo}-{moved} now belongs to another worker: touch nothing in it and run no command there."

AUDIT = """You are an auditor on the repository {repo}. Do one item: read the code, the records and the logs that it names, find each fact that it asks for, and write each in a register, scratch://audit/{key}.md ($TMPDIR/audit/{key}.md), with its evidence: the file and the line, or the command and the lines that it printed. Check each claim of the item in the code, since it is a claim. Change no file of the repository, make no worktree, push nothing and open no PR. Read a file with grep(pattern) or span(lo, hi), never whole when a part serves. Close with a tuple: 0, and your report in 200 words at most: the path of the register, what you found, and what is left.
""" + RULES + """

The item, {key}:
{what}"""

FIX = """You fix the open PR #{n} of the repository {repo}: it {problem} at {sha}.{base} Read it with gh pr view, gh pr checks and gh run view --log-failed. Read the failed log before any rerun, since a rerun of a whole system job takes 5 to 6 minutes: rerun once only a check whose log shows a fault of the machine and not of a test. Else work at the head of its branch in the workspace that the end of this brief names, and fetch before each push. If the head of the PR is not {sha} any more, or the PR no longer {problem}, change nothing and close with its number and a report that says so. A PR that conflicts runs no checks, so the checks that it shows may be old. When the commits of main that this brief names show a revert, a deleted file or a moved file, and a file that your hunks change is gone from main, take the file of main and apply your change again in its new place. Fix it: resolve a conflict with the meaning of both sides, and for a stack read skill("gh-stack") and use gh stack sync. Run the tests that it touches, push with git push --force-with-lease=refs/heads/<branch>:<the sha that you fetched> origin HEAD:refs/heads/<branch>, wait until gh pr view --json headRefOid,mergeable shows your head (GitHub shows the old head and the old mergeable for about 20 seconds after a push), keep the worktree until the PR merges, and close with a tuple: the number of the PR, and your report in 200 words at most. """ + RULES

FIXOWN = "Your open PR #{n} {problem} at {sha}. Fetch origin, run git merge --ff-only origin/<your branch> when your local branch lags behind its remote, rebase your branch onto its base or merge its base into it, resolve each conflict with the meaning of both sides, run the gates that the change touches, push with --force-with-lease, and keep its checks green. Never wait on CI in a bash for more than 10 minutes: once you pushed, read its checks once and close with the PR and its checks as they stand, since the round tends them."

RESUME = """Your task did not end: your prompt ended with CancelledError, and nobody cancelled it. Go on with the same task on this chain: fetch your branch and read your PR, run again each command that ended as interrupted, and go on from where you stood. Wait on CI only with await checked(number), again after a pending, never inside a bash. Close with a tuple: the number of your last PR, and your report in 200 words at most."""

PARKED = """The round pauses this chain to hold its cap of chains. Your task did not end: when a slot frees, the round wakes this chain, and you go on from where you stood."""

DEBRIEF = """Your work is done. As a user of furb, list each friction, defect or confusion that you met in this chain, with its evidence and what would fix it. Close with a list of str, one for each, or an empty list."""

LABELS = "gh label create hold -c FBCA04 -d 'It waits for the fix of a review before a merge' 2>/dev/null; gh label create owner-merge -c 5319E7 -d 'The owner merges it or not' 2>/dev/null; gh label create paused -c BFBFBF -d 'The owner paused it: no fix and no merge' 2>/dev/null; true"

PRS = """gh pr list --state open --limit 200 --json number,isDraft,changedFiles,labels,mergeable,mergeStateStatus,baseRefName,headRefName,headRefOid,statusCheckRollup,updatedAt,body --jq '[.[] | ([.statusCheckRollup[]?] | group_by(.name // .context // "") | map(max_by(if (.status // "COMPLETED") == "COMPLETED" then (.startedAt // .completedAt // "") else "~" end))) as $c | {n: .number, draft: .isDraft, files: .changedFiles, labels: [.labels[].name], mergeable, state: .mergeStateStatus, base: .baseRefName, head: .headRefName, sha: .headRefOid, checks: [$c[] | (.conclusion // .state // "")], names: [$c[] | (.name // .context // "")], open: [(.body // "") | splits("\\n") | select(startswith("- [ ] "))], idle: ((now - (.updatedAt | fromdateiso8601)) / 60 | floor)}]'
c=$?; echo; echo "--- $c"
gh pr list --state merged --limit 200 --json number --jq '[.[].number]'
c=$?; echo; echo "--- $c"
gh run list --branch main --workflow gates.yml --limit 20 --json headSha,conclusion --jq '.[0].headSha as $s | {sha: ($s // ""), runs: [.[] | select(.headSha == $s) | .conclusion]}'
c=$?; echo; echo "--- $c"
cat DECISIONS.md 2>/dev/null"""

REVIEWS = r"""prs=$(gh pr list --state open --json number --jq '.[].number')
for n in $prs; do
{ gh api "repos/{owner}/{repo}/pulls/$n/comments" --jq '.[] | "\(.pull_request_url | split("/") | last)\tc\(.id)\t\(.user.login)\t\(.reactions["+1"] // 0)\t\(.path):\(.line // .original_line)\t\(.body | gsub("[\r\n\t]"; " "))"' 2>/dev/null; echo; }
{ gh api "repos/{owner}/{repo}/pulls/$n/reviews" --jq '.[] | select(.body != "") | "\(.pull_request_url | split("/") | last)\tr\(.id)\t\(.user.login)\t0\t-\t\(.body | gsub("[\r\n\t]"; " "))"' 2>/dev/null; echo; }
done
for n in $prs $(gh issue list --state open --json number --jq '.[].number'); do
{ gh api "repos/{owner}/{repo}/issues/$n/comments" --jq '.[] | "\(.issue_url | split("/") | last)\ti\(.id)\t\(.user.login)\t\(.reactions["+1"] // 0)\t-\t\(.body | gsub("[\r\n\t]"; " "))"' 2>/dev/null; echo; }
done"""


landed: list[int] = globals().setdefault("landed", [])
heard: list[str] = globals().setdefault("heard", [])
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


def linted(out: str) -> set[str]:
  found: set[str] = set()
  for x in out.splitlines():
    hit = re.match(r"\S+:\d+:\d+: (.+)", x)
    if hit:
      found.add(hit[1])
    elif re.match(r"[+-](?![+-]{2} )", x):
      found.add(x)
  return found


async def patch(title: str, why: str, ask: str) -> tuple[str, str, str]:
  def spliced(text: str, edits: list[tuple[str, str]]) -> str:
    spots: list[tuple[int, int, str]] = []
    for old, new in edits:
      if not old or old == new or text.count(old) != 1:
        count = text.count(old) if old else 0
        raise Refused(
          f"each old must occur once in the word and differ from its new, and this one occurs {count} times:"
          f" {old[:300]!r}"
        )
      spots.append((text.index(old), text.index(old) + len(old), new))
    out, end = "", 0
    for at, stop, new in sorted(spots):
      if at < end:
        raise Refused(f"two olds overlap at character {at} of the word")
      out, end = out + text[end:at] + new, stop
    return out + text[end:]

  def changed(text: str, edits: list[tuple[str, str]]) -> list[str]:
    heads = r"^(?:(?:async )?def |class )(\w+)|^(\w+)(?:: [^=\n]+)? = "

    def names(part: str) -> list[str]:
      return [a or b for a, b in re.findall(heads, part, re.M)]

    said: list[str] = []
    for old, new in edits:
      at = text.index(old)
      near = names(text[:at])
      added = [x for x in names(new) if x not in names(old)]
      after = f", after {near[-1]}" if near else ""
      adds = f" and adds {', '.join(added)}" if added else ""
      line = text.count("\n", 0, at) + 1
      lines = [f"{n} line" + ("s" if n != 1 else "") for n in (len(old.splitlines()), len(new.splitlines()))]
      said.append(f"Line {line}{after}: the pair changes {lines[0]} into {lines[1]}{adds}.")
    return said

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
        found = [x for x in gate(word) if re.sub(r"^line \d+: ", "", x) not in known]
        if found:
          raise Refused("the gate refuses the word:\n" + "\n".join(found))
        wrote = await bash(
          f"cd {tree} && cat > {CONDUCTOR} && git status --porcelain", stdin=word, show=HIDDEN, timeout=60
        )
        if wrote.stdout.content.strip() != f"M {CONDUCTOR}":
          raise Refused(f"git sees more than the pairs: {wrote.stdout.content[:300]}")
        fresh = sorted(linted((await bash(lint, show=HIDDEN, timeout=120)).stdout.content) - base)
        if fresh:
          raise Refused("ruff finds what it does not find in the word of origin/main:\n" + "\n".join(fresh)[:1500])
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
  kind = kind_of(key)
  return efforts.get(key) or (limits["fixer"] if kind == "fix" else KINDS[kind])


def kind_of(key: str) -> str:
  kind = kinds.get(key) or ("fix" if re.fullmatch(r"(fix|box)\d+", key) else "edit")
  return kind if kind in (*KINDS, "fix") else "edit"


def family(model: str) -> str:
  name = model.split(":")[-1]
  return name if name in limits["room"] else name.split("/")[0]


def open_chains() -> list[str]:
  found: list[str] = []
  seen, todo = {ROOT}, [ROOT]
  while todo:
    for x in transcript(todo.pop()):
      c = str(x[1])
      if x[0] == "chain" and c not in seen:
        seen.add(c)
        todo.append(c)
        if c not in parked and any(y[0] == "prompt" and not over(y[1]) and y[4] != "None" for y in transcript(c)):
          found.append(c)
  return sorted(found, key=lambda c: int(re.sub(r"\D", "", c) or 0))


def room(model: str) -> bool:
  if model not in MODELS:
    return False
  mine = family(model)
  live = [n for n in working if not over(n) and family(models.get(n, ACTOR)) == mine]
  return len(live) < limits["room"][mine] and len(open_chains()) < cap()


def resume(key: str, where: str, model: str, text: str) -> str:
  if not room(model):
    parked[where] = {"name": "", "key": key, "model": model, "text": text}
    heard.append(f"{key} waits on {where} for a free slot of the cap of {cap()} chains.")
    return ""
  quietly(rung, f"actor = {model!r}", on=where)
  if kind_of(key) != "audit" and "{workspace}" not in text:
    text += "\n\n{workspace}"
  text, slot = placed(key, text, where)
  name = str(quietly(prompt, tuple[int, str], text, to=model, on=where))
  working[name], models[name], hired[name] = key, model, clock()
  if slot:
    slots[name] = slots[where] = slot
  return name


def can(key: str) -> bool:
  model = model_of(key)
  if model not in MODELS:
    notice(
      f"effort {key} {model}",
      f"The item {key} asks for {model}, and a worker runs only {', '.join(MODELS)}: xhigh and max stay for you and the rulings of the owner. Set efforts[{key!r}] to one of them.",
    )
    return False
  return room(model)


VIEW = """gh pr view {n} --json state,headRefOid,mergeable --jq '[.state, .headRefOid, .mergeable] | join(" ")'"""


def fixing(pr: dict) -> bool:
  held = assigned.get(pr["n"])
  return bool(held and held["fixer"] and not over(held["who"]))


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


# The order of a fixer reads git when it is given, since the head of a base moves between two orders.
REBASED = """git fetch -q origin "+refs/heads/$base:refs/remotes/origin/$base" "+refs/heads/$head:refs/remotes/origin/$head" || exit 1
git cat-file -e "$old^{commit}" 2>/dev/null || exit 1
if git merge-base --is-ancestor "$old" "origin/$base"; then
  echo plain
  git merge-base "origin/$head" "origin/$base" || exit 1
  git rev-list --count "origin/$base..origin/$head" || exit 1
  git log --format='%h %s' "$old..origin/$base" | head -5
else
  c=$(git merge-base "origin/$head" "$old") || exit 1
  echo onto
  echo "$c"
  git rev-list --count "$c..origin/$head" || exit 1
fi"""

CHANGED = """git fetch -q origin main "refs/pull/$n/head" || exit 1
cut=$(git merge-base origin/main "$sha") || exit 1
[ -n "$(git diff --name-only "$cut" "$sha")" ] || exit 0
git diff --name-only -z "$cut" "$sha" | xargs -0 git log --format='C%x09%h%x09%s' --name-status "$cut..origin/main" -- | head -80"""


async def based_now(pr: dict) -> str:
  said = based(pr)
  if not said:
    return ""
  n, base, head = pr["n"], pr["base"], pr["head"]
  ref = f" Its checks run on refs/pull/{n}/merge, the merge of its head into {base}: fetch that ref to see what they saw."
  if n not in shifted:
    return said + ref
  old, new = shifted[n]
  setup = f"base={shq(base)} head={shq(head)} old={shq(old)}\n"
  got = await bash(setup + REBASED, show=HIDDEN, timeout=120)
  if got.code != 0 or len(got.stdout.lines) < 3:
    return said + ref
  how, cut, count, *gained = got.stdout.lines
  commits = f"{count} commit" + ("" if count == "1" else "s")
  if how == "plain":
    move = f"git rebase origin/{base} puts its {commits} on a base that only advanced"
  else:
    move = f"the base was rewritten and the branch stands on {cut[:12]} of it: git rebase --onto origin/{base} {cut} {head} replays its {commits}"
  lost = "\nThe base gained:\n" + "\n".join("  " + x for x in gained) + "\n" if gained else ""
  return f" Its base is {base}, whose head changed from {old} to {new}: {move}.{ref}{lost}"


SQUASHED = """git fetch -q origin main "refs/pull/$n/head" || exit 1
git cherry -v origin/main "$sha" | cut -c1-120"""


async def squashed(pr: dict) -> str:
  got = await bash(f"n={pr['n']} sha={shq(pr['sha'])}\n" + SQUASHED, show=HIDDEN, timeout=120)
  rows = [x.split(" ", 2) for x in got.stdout.lines if x[:2] in ("+ ", "- ")] if got.code == 0 else []
  held = [(x[1], x[2] if len(x) > 2 else "") for x in rows if x[0] == "-"]
  if not held:
    return ""
  last = max(i for i, x in enumerate(rows) if x[0] == "-")
  if all(x[0] == "-" for x in rows[: last + 1]):
    fix = f"git rebase --onto origin/main {held[-1][0][:12]} {pr['head']} replays only the commits after them"
  else:
    fix = "git rebase origin/main drops them as it replays your other commits"
  lines = "\n".join(f"  {one[:7]} {title}" for one, title in held[:8])
  return (
    f"\nThis is no stack. The branch of this PR holds {len(held)} commit{'' if len(held) == 1 else 's'} whose patch"
    " main holds already, the commits of a PR that main took as one squash commit, so a merge of main conflicts on"
    f" them and gh stack sync does not help:\n{lines}\nFix: {fix}.\n"
  )


async def main_changes(pr: dict) -> str:
  held = await squashed(pr)
  got = await bash(f"n={pr['n']} sha={shq(pr['sha'])}\n" + CHANGED, show=HIDDEN, timeout=120)
  if got.code != 0:
    return held
  commits: list[list] = []
  for line in got.stdout.lines:
    parts = line.split("\t")
    if parts[0] == "C" and len(parts) >= 3:
      commits.append([f"{parts[1]} {parts[2]}", [], [], 0])
    elif commits and len(parts) >= 2:
      if parts[0] == "D":
        commits[-1][1].append(parts[1])
      elif parts[0].startswith("R") and len(parts) >= 3:
        commits[-1][2].append(f"{parts[1]} to {parts[2]}")
      commits[-1][3] += 1
  if not commits:
    return held
  commits.sort(key=lambda c: not (c[1] or c[2] or " Revert" in " " + c[0].split(" ", 1)[-1]))
  rows = []
  for title, gone, moved, files in commits[:8]:
    rows.append(
      f"  {title} ({files} of your files)"
      + (f"; it deletes {', '.join(gone[:4])}" if gone else "")
      + (f"; it moves {', '.join(moved[:4])}" if moved else "")
    )
  return held + (
    "\nThe commits of main since the fork that changed a file of this PR (a revert, a deleted file or a moved file"
    " means that your hunks belong in another place):\n" + "\n".join(rows) + "\n"
  )


async def stale(pr: dict, conflict: bool = False) -> bool:
  got = await bash(VIEW.replace("{n}", str(pr["n"])), show=HIDDEN, timeout=120)
  parts = got.stdout.content.split()
  if got.code != 0 or len(parts) != 3:
    return False
  return parts[0] != "OPEN" or parts[1] != pr["sha"] or (conflict and parts[2] == "MERGEABLE")


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
  briefed[key] = custom.get(key) or what or brief
  if not brief and kind_of(key) != "audit":
    quietly(bash, "; ".join(f"gh label create {m} --force >/dev/null 2>&1" for m in marks), show=HIDDEN)
  quietly(write, Text("scratch://build.sh", BUILD_SH))
  text = (custom.get(key) or brief or (AUDIT if kind_of(key) == "audit" else WORKER)).replace("{labels}", "".join(f" --label {m}" for m in marks))
  text = text.replace("{repo}", cwd()).replace("{key}", key).replace("{home}", scope(acting())).replace("{what}", what)
  where = reuse(key, model, thread) or str(chain(f"{thread or 'w'}/{key}", source=base_of(kind_of(key), model)))
  quietly(rung, f"actor = {model!r}", on=where)
  if kind_of(key) != "audit" and "{workspace}" not in text:
    text += "\n\n{workspace}"
  text, slot = placed(key, text, where)
  name = str(quietly(prompt, tuple[int, str], text, to=model, on=where))
  working[name] = key
  models[name] = model
  hired[name] = clock()
  homes[key] = where
  if slot:
    slots[name] = slots[where] = slot
  return name


def placed(key: str, text: str, where: str = "") -> tuple[str, str]:
  if kind_of(key) == "audit":
    return text, ""
  held = {s for n, s in slots.items() if (n in working and not over(n)) or n == "coordinator"}
  # A chain goes on in the workspace that it held last: its history names that path in its commands.
  last = slots.get(where, "")
  free = next(f"w{i}" for i in range(1, cap() + len(held) + 2) if f"w{i}" not in held)
  slot = last if last and last not in held else free
  moved = " " + MOVED.replace("{moved}", last) if last and last != slot else ""
  text = text.replace("{workspace}", WORKSPACE + moved).replace("{repo}", cwd()).replace("{slot}", slot)
  if where:
    path = f"{cwd()}-{slot}"
    home = shq(path)
    quietly(bash, f'[ -d {home} ] && echo {shq(where)} > "$(git -C {home} rev-parse --absolute-git-dir)/furb-holder"', show=HIDDEN)
    quietly(rung, f"try:\n  cd({path!r})\nexcept Refused:\n  pass", on=where)
  return text.replace("{chain}", where), slot


def base_of(kind: str, model: str) -> str:
  name = f"{kind} {model}"
  if name not in bases:
    where = str(quietly(chain, f"base/{kind}/{model.split(':')[-1]}"))
    quietly(rung, f"actor = {model!r}\nread({cwd() + '/AGENTS.md'!r})", on=where)
    bases[name] = where
  return bases[name]


def share_of(where: str) -> float:
  replies = transcript(where, "reply")
  return float(filled(replies[-1][1]) or 0) if replies else 0.0


def reuse(key: str, model: str, thread: str) -> str:
  near = set(areas.get(key, []))
  for where, one in list(pool.items()):
    if one["model"] != model or not get(where) or paused(where) or where in parked:
      continue
    if (thread and one["thread"] == thread or near & set(one["folders"] or [])) and share_of(where) < 0.5:
      del pool[where]
      heard.append(f"{key} goes on {where}, the chain of {one['key']}, which worked on the same area.")
      return where
  return ""


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


def maker_of(pr: dict) -> str:
  keys = [m[2:] for m in pr["labels"] if m.startswith("w:")]
  return next(
    (w for w, key in working.items() if not over(w) and (key in keys or pr["n"] in owns.get(key, []))), ""
  )


def assignee(pr: dict) -> str:
  held = assigned.get(pr["n"])
  live = held and not over(held["who"]) and (held["fixer"] or held["sha"] == pr["sha"])
  return held["who"] if held and live else ""


def owner_of(pr: dict) -> str:
  return assignee(pr) or maker_of(pr)


def ours() -> set[str]:
  return {
    line
    for x in transcript(ROOT, "write")
    if isinstance(t := peek(x[1]), Text) and t.path.endswith(DECISIONS)
    for line in t.lines
  }


# The delimiters of a long string, built apart so that no line of this reader opens one.
TRIPLES = ('"' * 3, "'" * 3)
DEFINED = re.compile(r"(?:async def |def |class )(\w+)|(\w+)(?:\s*:\s*[^=\n]+)?\s*=(?!=)")


def parts_of(word: str) -> dict[str, str]:
  """The text of each top-level definition of a word by its name, with the comments and decorators above it."""
  stmts: list[list[str]] = []
  quote = ""
  for line in word.split("\n"):
    if not stmts or (not quote and line[:1] not in ("", " ", "\t", ")", "]", "}")):
      stmts.append([])
    stmts[-1].append(line)
    at = 0
    while True:
      if quote:
        hit = line.find(quote, at)
        if hit < 0:
          break
        quote, at = "", hit + 3
      else:
        hits = sorted((line.find(d, at), d) for d in TRIPLES if line.find(d, at) >= 0)
        if not hits:
          break
        quote, at = hits[0][1], hits[0][0] + 3
  found: dict[str, str] = {}
  above: list[str] = []
  for lines in stmts:
    code = [x for x in lines if x.strip() and not x.startswith(("#", "@"))]
    if not code:
      above += lines
      continue
    named = DEFINED.match(code[0])
    if named:
      found[named[1] or named[2]] = "\n".join([*above, *lines]).strip("\n").rstrip()
    above = []
  return found


def reload(path: str = CONDUCTOR, on: str = "") -> tuple[str, list[str]]:
  here = on or scope(acting())
  lived: dict[str, str] = {}
  for x in transcript(here):
    if x[0] == "module":
      lived = {}
    elif x[0] == "run" and over(x[1]) and not isinstance(peek(x[1]), BaseException):
      lived.update(parts_of(str(x[5])))
  held = module(here)
  fresh = {n: t for n, t in parts_of(read(path, HIDDEN).content).items() if n not in held or lived.get(n) != t}
  if not fresh:
    return "", []
  # The close of the word that calls reload cancels each rung that the word made, and the load stops between two
  # definitions: quietly makes the rung for the round, so no end of the caller reaches it.
  return str(quietly(rung, "\n\n".join(fresh.values()), on=here)), list(fresh)


def tend() -> None:
  now = clock()
  for orig, relay in list(relays.items()):
    if relay and over(relay):
      close(peek(relay), id=orig)
      relays[orig] = ""
  for name in [x for x in working if over(x)]:
    key = working.pop(name)
    slots.pop(name, "")
    got = peek(name)
    if isinstance(got, CancelledError) and get(name):
      kin, up = {name}, name
      while (a := get(up)) and a[2] and a[2] not in kin:
        up = a[2]
        kin.add(up)
      where = str(get(name)[3])
      tries = len([k for k in noticed if k.startswith(f"resume {key} ")])
      if tries < 3 and not [x for x in transcript(where, "cancel") if x[1] in kin]:
        noticed.add(f"resume {key} {name}")
        resume(key, where, models.get(name, ACTOR), RESUME)
        continue
    results[name] = got
    if get(name):
      home = str(get(name)[3])
      label = str((get(home) or ('', '', '', '', ''))[4])
      pr_n = got[0] if isinstance(got, tuple) and got and isinstance(got[0], int) else 0
      thread = label.split('/')[0] if '/' in label else ''
      pool[home] = {'key': key, 'model': models.get(name, ACTOR), 'thread': thread, 'pr': pr_n, 'folders': None}
    ends.setdefault(key, []).append((name, got[0] if isinstance(got, tuple) and got and isinstance(got[0], int) else 0))
    if DEBRIEFS and f"debrief {name}" not in noticed and get(name):
      noticed.add(f"debrief {name}")
      debriefs[str(quietly(debrief, name))] = key
    if isinstance(got, tuple) and len(got) == 2 and isinstance(got[0], int) and got[0] > 0:
      heard.append(f"The worker {key} ({name}) ended with PR #{got[0]}, and results[{name!r}] keeps its report.")
    elif kind_of(key) == "audit" and isinstance(got, tuple) and len(got) == 2:
      heard.append(f"The auditor {key} ({name}) ended, and results[{name!r}] keeps its report.")
    else:
      quietly(prompt, None, f"The worker {key} ({name}) ended with no PR:\n\n{str(got)[:2000]}" + merged_since())
  for asked, key in list(debriefs.items()):
    if over(asked):
      del debriefs[asked]
      said = peek(asked)
      if isinstance(said, list) and said:
        flaw(*[str(one) for one in said], source=f"debrief of {key}")
  live: set[str] = set()

  def cut(act: str, home: str, why: str) -> None:
    quietly(prompt, None, why, on=home)
    cancel(act)

  def watchdog(worker: str, item: str) -> None:
    for home in sorted({str(get(worker)[3]), working_at(worker)}):
      for x in transcript(home):
        if x[0] != "bash" or over(x[1]):
          continue
        act, command = str(x[1]), str(x[4])
        live.add(act)
        try:
          out = read(f"{act}/stdout", HIDDEN).content + read(f"{act}/stderr", HIDDEN).content
        except Refused:
          out = ""
        seen = watched.setdefault(act, [now, len(out), now])
        if len(out) != seen[1]:
          seen[1], seen[2] = len(out), now
        age, quiet = int((now - seen[0]) // 60), int((now - seen[2]) // 60)
        ci = re.search(r"gh +(?:pr +checks\b.*--watch|run +watch)", command, re.S) or (
          re.search(r"\b(?:while|until|for)\b", command)
          and re.search(r"\bsleep\b", command)
          and re.search(r"\bgh\b", command)
        )
        tail = "\n".join(out.splitlines()[-5:])[-300:]
        if ci and now - seen[0] > 600:
          cut(
            act,
            home,
            f"The round cut your wait on CI ({command[:100]}) after {age} minutes. It ends a wait, never your task:"
            " stay on your PR, wait for its checks with await checked(number) (never --watch, never a loop of your own),"
            " and close only when your PR has merged.",
          )
          heard.append(f"The round cut {act} of {item} on {home}: a wait on CI of {age} minutes.")
        elif now - seen[2] >= 1800:
          cut(
            act,
            home,
            f"The round cut your command {act} ({command[:100]}) after {quiet} minutes with no new output. It ends a"
            " command, never your task: find what it waited for, run it again in a bound that fits, and go on.",
          )
          notice(
            f"watchdog {act}",
            f"The round cut {act} of {item} on {home} after {quiet} minutes with no output: {command[:100]!r}."
            f" Its last output:\n{tail}",
          )
        elif now - seen[2] >= 900 and f"silent {act}" not in noticed:
          noticed.add(f"silent {act}")
          heard.append(
            f"{act} of {item} on {home} has printed nothing for {quiet} minutes: {command[:100]!r}; its last output: {tail!r}"
          )

  def held_chains() -> None:
    # A word that awaits holds its chain, so a prompt of the owner on it is never asked. The round cannot tell the
    # wait that it makes itself in conduct() from another, so it leaves the home of the round alone.
    chains, todo = {ROOT}, [ROOT]
    while todo:
      for x in transcript(todo.pop()):
        if x[0] == "chain" and str(x[1]) not in chains:
          chains.add(str(x[1]))
          todo.append(str(x[1]))
    for home in sorted(chains - {lead.get("home", "")}):
      facts = transcript(home)
      # A prompt makes its rung at once, before the chain asks it: it is asked when a reply is said by a rung of it.
      replied = {y[2] for y in facts if y[0] == "reply"}
      asked = {y[2] for y in facts if y[0] == "rung" and y[1] in replied}
      unasked = [
        str(y[1]) for y in facts if y[0] == "prompt" and y[2] == OPERATOR and not over(y[1]) and y[1] not in asked
      ]
      holding = [x for x in facts if x[0] in ("wait", "run") and not over(x[1])]
      if not (unasked and holding):
        continue
      held = []
      for x in holding:
        live.add(f"held {x[1]}")
        if now - watched.setdefault(f"held {x[1]}", [now, 0, now])[0] >= HELD:
          held.append(x)
      cuts = [str(x[1]) for x in held if x[0] == "wait"] or [str(x[1]) for x in held if x[0] == "run"]
      if not cuts:
        continue
      quietly(
        prompt,
        None,
        f"The round cut {', '.join(cuts)}, which held this chain for {int(HELD // 60)} minutes while a prompt of the"
        f" owner ({', '.join(unasked)}) waited and was never asked. A word that awaits holds the chain: await nothing"
        " over 60 s, end your word, and peek at the next advance.",
        on=home,
      )
      for cut_act in cuts:
        cancel(cut_act)
        if not over(cut_act):
          say("done", cut_act, CancelledError())
      heard.append(
        f"The round cut {', '.join(cuts)} on {home}: it held the chain while the prompt {', '.join(unasked)} of the"
        " owner waited unasked."
      )

  for name, key in working.items():
    where = working_at(name)
    if paused(where) and where not in parked:
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
    watchdog(name, key)
  held_chains()
  for gone in [a for a in watched if a not in live]:
    del watched[gone]
  late = (
    "An hour passed since you took this item, and no PR of yours is open. Land it in steps: commit now the coherent"
    " part that serves his words, open its PR with pr(), and go on with the rest in a next PR from this chain,"
    " each PR one change. A PR that waits for every detail lands nothing,"
    " and each step is judged and merged on its own."
  )
  owners = {owner_of(pr) for pr in prs.values()}
  for name in [x for x in working if not over(x) and x not in paced]:
    if name in owners:
      paced.add(name)
    elif now - hired.get(name, now) > 3600.0:
      paced.add(name)
      quietly(prompt, None, late, on=working_at(name))


TROUBLE = ("red", "conflicting")


def tended(pr: dict) -> bool:
  # A draft is the work in progress of its maker while that maker lives; once the worker ended, the round tends it.
  return (
    pr["head"].startswith("w/") and not {"paused", "hold"} & set(pr["labels"]) and not (pr["draft"] and maker_of(pr))
  )


def waits_main(pr: dict) -> bool:
  held = onmain.get(pr["n"])
  return bool(held and held[2] and held[0] == pr["sha"] and held[1] in redmain)


def table(pr: dict) -> str:
  checks = pr["checks"]
  if pr["mergeable"] == "CONFLICTING":
    return "conflicting"
  if pr["verdict"] == "fail" and waits_on(pr):
    return "owner"
  if pr["verdict"] == "fail" and waits_main(pr):
    return "main"
  running = any(c in RUNNING for c in checks)
  # A lone failed check beside a check that runs may be a flake that a rerun heals once its run ends; two are none.
  if pr["verdict"] == "fail" and not (running and sum(c in FAILS for c in checks) < 2) or pr["verdict"] == "open":
    return "red"
  if running or pr["verdict"] != "pass":
    return "running"
  if pr["base"] != "main":
    return "stacked"
  return "owner" if "owner-merge" in pr["labels"] else "green"


def spent(pr: dict) -> bool:
  held = assigned.get(pr["n"]) or {}
  return held.get("sha") == pr["sha"] and held.get("tries", 0) >= 2


def maker_key(pr: dict) -> str:
  return next((m[2:] for m in pr["labels"] if m.startswith("w:")), "")


def handing(pr: dict) -> bool:
  maker = maker_key(pr)
  return bool(homes.get(maker)) and maker not in working.values()


def mender(pr: dict) -> str:
  if not handing(pr):
    return limits["fixer"]
  last = next((name for name, _ in reversed(ends.get(maker_key(pr), []))), "")
  return next((m for m in (HANDOVER, models.get(last, "")) if room(m)), HANDOVER)


async def mend(pr: dict, now: float, red: bool) -> str:
  n, sha, state = pr["n"], pr["sha"], table(pr)
  decision = waits_on(pr) if state == "owner" else ""
  if decision and f"awaits {decision}" not in noticed:
    noticed.add(f"awaits {decision}")
    heard.append(
      f"#{n} waits on the decision {decision} of the owner, which holds {', '.join(awaited[decision])}: the"
      " round gives no order, no handover and no fixer for a failed check of them until the decision ends."
    )
  if not red and n in onmain:
    waited = onmain.pop(n)
    if waited[2] and waited[0] == sha and tended(pr) and not fixing(pr) and not pr["draft"]:
      got = await bash(BEHIND.replace("{sha}", sha).replace("{n}", str(n)), show=HIDDEN, timeout=180)
      if "updated" in got.stdout.content:
        warn(pr, f"The round updated the branch of your PR #{n} with main.")
        heard.append(
          f"#{n} waited on a red main at {sha[:7]}, and main is green again, so the round updated its branch."
        )
        return "running"
  if red and state in ("red", "main") and pr["verdict"] == "fail" and await on_main(pr):
    if f"waits {n} {sha}" not in noticed:
      noticed.add(f"waits {n} {sha}")
      if f"red {next(iter(redmain))}" in noticed:
        heard.append(
          f"#{n} fails at {sha[:7]} only the lines that main fails, so it waits on main: the round gives it no order"
          " and no fixer, and updates its branch once main is green."
        )
    return "main"
  if state == "red" or (state == "running" and not any(c in RUNNING for c in pr["checks"])):
    healed = await heal(pr, red)
    if healed:
      heard.append(healed)
      return "running"
  held = assigned.get(n) or {}
  if state not in TROUBLE or not tended(pr) or spent(pr):
    return state
  if state == "conflicting" and pr["head"] == GALLERY:
    # The gallery workflow builds this branch again from main, and a hand merge of its binary files means nothing.
    since = timers.setdefault(f"gallery {n} {sha}", now)
    if now - since < GALLERY_WAIT:
      if f"gallery {n} {sha}" not in noticed:
        noticed.add(f"gallery {n} {sha}")
        heard.append(f"#{n} ({GALLERY}) conflicts, and the gallery workflow builds it again from main, so the round orders no fixer.")
      return "running"
  if assignee(pr) and (held["fixer"] or now - held["at"] < OWNED):
    return state
  if await stale(pr, state == "conflicting"):
    return "running"
  turn = held if held.get("sha") == sha else {}
  maker, thread = maker_of(pr), next((m[7:] for m in pr["labels"] if m.startswith("thread:")), "")
  if pr["verdict"] == "open" and state == "red":
    order = (
      f"Your open PR #{n} has a box of its checklist open at {sha[:7]}:\n"
      + "\n".join(x[6:] for x in pr["open"])
      + f"\nDo the work that each open box names, verify it, then call pr(..., number={n}) again with the evidence of"
      " each box, so pr() ticks it. The round merges no PR with an open box."
    )
    key = f"box{n}"
    brief = (
      f"You close the open boxes of the checklist of the open PR #{n} of the repository {{repo}}. "
      + order.replace("Your open PR", "The PR")
      + " Read it with gh pr view, work at the head of its branch in the workspace that the end of this brief names,"
      " push with --force-with-lease, and close with a tuple: the number of the PR, and your report in 200 words at"
      " most. "
      + RULES
    )
  else:
    problem = "conflicts with its base" if state == "conflicting" else "has a failed check"
    note = await based_now(pr) + (await evidence(pr) if state == "red" else await main_changes(pr))
    order = FIXOWN.replace("{n}", str(n)).replace("{problem}", problem).replace("{sha}", sha) + note
    key = f"fix{n}"
    brief = FIX.replace("{n}", str(n)).replace("{problem}", problem).replace("{sha}", sha).replace("{base}", note)
  if maker and not turn.get("maker"):
    # The maker has OWNED seconds to move its head, then a fixer takes the PR: on 10-29 a maker busy with another
    # task left a conflict of #657 for an hour.
    late = f" A fixer takes the PR if its head does not move in {int(OWNED // 60)} minutes."
    quietly(prompt, None, order + late, on=working_at(maker))
    assigned[n] = {"sha": sha, "who": maker, "at": now, "fixer": False, "maker": maker, "tries": 0}
    return state
  label = next((m[2:] for m in pr["labels"] if m.startswith("w:")), "")
  who = ""
  if handing(pr):
    onto = mender(pr)
    if room(onto):
      said = f"Your work on this chain goes on, now on {onto}. {order} {{workspace}} Close with a tuple: the number"
      who = resume(label, homes[label], onto, said + " of the PR, and your report in 200 words at most.")
  elif can(key):
    who = hire(key, "", thread, brief)
  if who:
    tries = turn.get("tries", 0) + 1
    assigned[n] = {"sha": sha, "who": who, "at": now, "fixer": True, "maker": turn.get("maker", ""), "tries": tries}
    crowd(pr, maker)
  return state


async def failed_lines(pr: dict) -> list[str]:
  got = await bash(PRLINES.replace("{sha}", pr["sha"]), show=HIDDEN, timeout=300)
  return [x for x in got.stdout.lines if x.strip()][:30]


def alike(line: str) -> str:
  # The same failure shows in two logs with other times, durations and colors.
  return re.sub(r"\x1b\[[\d;]*m|\d{4}-\d\d-\d\dT[\d:.]+Z ?|\[[\d.]+m?s\]", "", line).strip()


async def on_main(pr: dict) -> bool:
  tip = next(iter(redmain), "")
  held = onmain.get(pr["n"])
  if held and held[:2] == (pr["sha"], tip):
    return held[2]
  mine = {alike(x) for x in await failed_lines(pr)} - {""}
  waits = bool(tip and mine and mine <= {alike(x) for x in redmain[tip]})
  onmain[pr["n"]] = (pr["sha"], tip, waits)
  return waits


async def evidence(pr: dict) -> str:
  failed = ", ".join(f"{a} {b}" for a, b in zip(pr["names"], pr["checks"]) if b in FAILS)
  head = f"\nIts failed checks: {failed}." if failed else ""
  seen = await failed_lines(pr)
  if not seen:
    return head + "\n" if head else ""
  said = head + "\nThe first failed lines of its job log:\n" + "\n".join("  " + x for x in seen) + "\n"
  files = sorted({f for x in seen for f in re.findall(r"[\w./-]+\.(?:py|rs|tsx?|json|toml|md|ya?ml|sh)\b", x)})[:8]
  if not files:
    return said
  lacks = await bash(
    f"git fetch -q origin main refs/pull/{pr['n']}/head 2>/dev/null; git log --format='%h %s' {shq(pr['sha'])}..origin/main -- "
    + " ".join(shq(f) for f in files)
    + " | head -8",
    show=HIDDEN,
    timeout=120,
  )
  if lacks.code != 0 or not lacks.stdout.content.strip():
    return said
  held = "\n".join("  " + x for x in lacks.stdout.lines)
  return said + "The commits of main that the branch lacks and that change a file of the failure:\n" + held + "\n"


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
git cat-file -e "$old^{commit}" 2>/dev/null || git fetch -q origin "$old" 2>/dev/null
git merge-base --is-ancestor "$old" "origin/$head" || old=$(git merge-base "$old" "origin/$head") || { echo "$head holds no $old"; exit 3; }
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


def shift() -> list[tuple[dict, str]]:
  seen = {pr["head"]: pr["sha"] for pr in prs.values()}
  left = [
    (pr, tips[parents[n]])
    for n, pr in prs.items()
    if pr["base"] == "main" and parents.get(n, "main") not in ("main", *seen) and parents[n] in tips
  ]
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
  parents.clear()
  parents.update({n: pr["base"] for n, pr in prs.items()})
  return left


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
    if n in {int(x) for k in noticed if k.startswith("linked ") for x in k.split()[1:]}:
      continue
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


SERIAL = re.compile(r"^(src/engine\.(py|pyi|rs)|\.github/budgets\.json)$")


async def whole(layers: list[dict]) -> str:
  """A green stack merges at once, all its PRs in one atomic merge of gh stack, by the ruling of the owner on 10-29."""
  ready: list[dict] = []
  for up in layers:
    calm = up["verdict"] == "pass" and not up["draft"] and not fixing(up)
    if not calm or {"hold", "paused", "owner-merge"} & set(up["labels"]):
      break
    if any([await veto(up) for veto in list(vetoes.values())]):
      break
    ready.append(up)
  rest = layers[len(ready) :]
  if rest and not any(up["verdict"] == "fail" for up in rest) and any(c in RUNNING for up in rest for c in up["checks"]):
    # A layer above still runs its checks: the stack waits to merge whole, and its bottom does not merge alone.
    return "wait"
  if len(ready) < 2:
    return ""
  names = " <- ".join("#" + str(up["n"]) for up in ready)
  got = await bash(
    f"out=$(gh stack merge {ready[-1]['n']} --yes --squash 2>&1); echo \"$out\" | tail -4;"
    + "".join(f" gh pr view {up['n']} --json state --jq .state;" for up in ready),
    show=HIDDEN,
    timeout=600,
  )
  if got.stdout.lines[-len(ready) :] == ["MERGED"] * len(ready):
    for up in ready:
      merged.add(up["n"])
      landed.append(up["n"])
    heard.append(f"The stack {names} merged whole.")
    return "merged"
  noticed.add(f"no stack {layers[0]['head']}")
  said = " ".join(got.stdout.lines[: -len(ready)])[-300:] or "nothing"
  heard.append(f"The stack {names} did not merge whole, so it lands one layer at a time; gh stack said: {said}")
  return ""


async def land(pr: dict, red: bool) -> None:
  n, sha = pr["n"], pr["sha"]
  if pr["verdict"] != "pass" or fixing(pr):
    return
  for veto in list(vetoes.values()):
    if await veto(pr):
      return
  layers = [pr]
  while len(ups := [up for up in prs.values() if up["base"] == layers[-1]["head"]]) == 1:
    layers.append(ups[0])
  if len(layers) > 1 and f"no stack {pr['head']}" not in noticed and await whole(layers):
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
    if {x for x in moved.splitlines() if SERIAL.search(x)} & {x for x in own.splitlines() if x}:
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
      parents[up["n"]] = "main"
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

handled: set[str] = globals().setdefault("handled", set())
ends: dict[str, list[tuple[str, int]]] = globals().setdefault("ends", {})
requeued: set[str] = globals().setdefault("requeued", set())
AGAIN = """Workers did parts of this issue before, and it is still open. Their PRs: {prs}. Read each with gh pr view, find what the issue still lacks on origin/main, and do the next part. Give pr() fixes={n} only when your change finishes the issue; else leave fixes at 0, so that the merge leaves the issue open for a next part."""


async def take_issues() -> None:
  got = await bash(ISSUES, show=HIDDEN, timeout=120)
  if got.code != 0 or not got.stdout.content.startswith("["):
    notice(f"issues {int(clock() // 3600)}", "The command that lists the issues failed: " + (got.stderr.content or got.stdout.content)[-500:])
    return
  for it in json.loads(got.stdout.content):
    key = f"issue{it['n']}"
    body = str(it["body"] or "")
    if "paused" in it["labels"]:
      continue
    fresh = [name for name, n in ends.get(key, []) if n and name not in requeued]
    if key in handled and not fresh:
      continue
    if body.startswith(MARKS):
      yes = await bash("gh api repos/{owner}/{repo}/issues/" + str(it["n"]) + "/reactions --jq '[.[] | select(.content == \"+1\") | .user.login] | index(\"" + OWNER + "\") != null'", show=HIDDEN, timeout=120)
      if yes.stdout.content.strip() != "true":
        continue
    handled.add(key)
    if taken(key) or any(q[0] == key for q in backlog) or any(f"w:{key}" in pr["labels"] for pr in prs.values()):
      continue
    requeued.update(fresh)
    brief = ISSUE.replace("{n}", str(it["n"])).replace("{url}", it["url"]).replace("{title}", it["title"])
    brief = brief.replace("{body}", body[:6000])
    if fresh:
      listed = ", ".join(f"#{n}" for _, n in ends[key] if n)
      brief += "\n\n" + AGAIN.replace("{prs}", listed).replace("{n}", str(it["n"]))
    queue.append((key, brief, "issues"))
    again = "again " if fresh else ""
    heard.append(f"The issue #{it['n']}, {it['title']}, is queued {again}as the item {key}.")
    quietly(bash, f"gh issue comment {it['n']} --body '🤖 Taken {again}: a worker of furb does this as the item {key}, and its PR or its answer comes here.'".replace(" :", ":"), show=HIDDEN)


STATUS = {
  "s:queued": ("C5DEF5", "A worker of furb takes it when a slot frees"),
  "s:working": ("1D76DB", "A worker of furb does it now"),
  "s:checks": ("FBCA04", "Its checks run"),
  "s:red": ("D93F0B", "A check failed, and its worker fixes it"),
  "s:main": ("E99695", "It fails the same lines as main, so it waits for main to be green"),
  "s:boxes": ("FEF2C0", "A box of its checklist is open, and its worker closes it"),
  "s:held": ("D4C5F9", "The label hold keeps it from a merge until it comes off"),
  "s:conflict": ("B60205", "It conflicts with its base, and its worker rebases it"),
  "s:stacked": ("BFD4F2", "Green, and lands after the PR under it"),
  "s:ready": ("0E8A16", "Green: the round merges it"),
  "s:owner": ("5319E7", "Green, and waits for the merge of the owner"),
}
statused: dict[int, str] = globals().setdefault("statused", {})


async def mark() -> None:
  want: dict[int, str] = {}
  for n, pr in prs.items():
    if "paused" in pr["labels"]:
      continue
    want[n] = (
      "s:conflict"
      if pr["mergeable"] == "CONFLICTING"
      else "s:main"
      if table(pr) == "main"
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
secret: dict[str, str] = globals().setdefault("secret", {})
NEWS_PY = """import subprocess, sys
repo, digest, line = sys.argv[1:4]
got = subprocess.run(["gh", "issue", "view", digest, "--repo", repo, "--json", "body", "--jq", ".body"], capture_output=True, text=True, check=True)
head, _, rest = got.stdout.rstrip("\\n").partition("\\n\\n")
body = (head + "\\n\\n- " + line + ("\\n" + rest if rest else ""))[:60000]
subprocess.run(["gh", "issue", "edit", digest, "--repo", repo, "--body-file", "-"], input=body, text=True, check=True)
"""
OPS_WATCH = r"""gh api 'repos/{ops}/issues/comments?sort=created&direction=desc&per_page=100' --jq '.[] | "\(.issue_url | split("/") | last)\to\(.id)\t\(.user.login)\t\(.body | gsub("[\n\t]"; " "))"'
gh issue list --repo {ops} --state all --limit 100 --json number,author,title,body --jq '.[] | "\(.number)\tn\(.number)\t\(.author.login)\t\(.body | gsub("[\n\t]"; " ")) (title: \(.title))"'"""


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


REHEAD = """[ "$(gh pr view {n} --json headRefOid --jq .headRefOid)" = {sha} ] || exit 0
for id in $(gh run list --commit {sha} --limit 50 --json databaseId,workflowName,createdAt,conclusion --jq 'group_by(.workflowName) | map(max_by(.createdAt)) | .[] | select({which}) | .databaseId'); do gh run rerun $id{failed} >/dev/null 2>&1 && echo $id; done"""

BEHIND = """[ "$(gh api 'repos/{owner}/{repo}/compare/main...{sha}' --jq .behind_by 2>/dev/null)" -gt 0 ] 2>/dev/null && gh pr update-branch {n} >/dev/null 2>&1 && echo updated"""

FAILLINES = r"""gh run view "$id" --log-failed 2>/dev/null | grep -E 'FAILED|panicked|Error|error\[|AssertionError|\(fail\)|timed out' | cut -f1,3 | head -30 | cut -c1-300"""

REDLINES = r"""id=$(gh run list --branch main --workflow gates.yml --commit {sha} --limit 1 --json databaseId --jq '.[0].databaseId'); """ + FAILLINES

PRLINES = r"""id=$(gh run list --commit {sha} --limit 30 --json databaseId,conclusion --jq '[.[] | select(.conclusion == "failure" or .conclusion == "timed_out")][0].databaseId'); [ -n "$id" ] && [ "$id" != null ] && """ + FAILLINES

UPSTREAMING = """gh pr list --repo {upstream} --label upstream-ready --state open --json number,headRefName,title,url,labels --jq '.[] | select([.labels[].name] | index("upstreamed") | not) | "\\(.number)\\t\\(.headRefName)\\t\\(.title)\\t\\(.url)"'"""


# A PR whose update with main still fails holds a failure of its own: a second update in a short time only spends
# the runners again, since the update itself makes the new head that fails.
updated: dict[int, float] = globals().setdefault("updated", {})

UPDATED = 6 * 3600.0

# Minutes that a PR stands still before the round finds that no run of the gates stands at its head.
GATELESS = 5


async def heal(pr: dict, red: bool) -> str:
  n, sha = pr["n"], pr["sha"]
  def rehead(which: str, only: str = "") -> str:
    return REHEAD.replace("{n}", str(n)).replace("{sha}", sha).replace("{which}", which).replace("{failed}", only)

  failed = [c for c in pr["checks"] if c in FAILS]
  if pr["mergeable"] == "CONFLICTING":
    return ""
  # The gates run for a PR on main only: a PR pushed while its base was another branch, then moved to main when that
  # base merged, has no run of them at its head, and no event runs them again.
  if (
    pr["base"] == "main"
    and GATE not in pr["names"]
    and pr["idle"] >= GATELESS
    and not pr["draft"]
    and not {"hold", "paused"} & set(pr["labels"])
    and not fixing(pr)
    and f"gateless {sha}" not in noticed
  ):
    noticed.add(f"gateless {sha}")
    if not red:
      got = await bash(BEHIND.replace("{sha}", sha).replace("{n}", str(n)), show=HIDDEN, timeout=180)
      if "updated" in got.stdout.content:
        warn(pr, f"The round updated the branch of your PR #{n} with main.")
        return f"#{n} had no run of the gates at {sha[:7]}, so the round updated its branch."
    got = await bash(rehead("true"), show=HIDDEN, timeout=300)
    if got.stdout.content.strip():
      return f"#{n} had no run of the gates at {sha[:7]}, and the round ran its workflows again."
    notice(
      f"gateless {sha}",
      f"#{n} has no run of the gates at {sha[:7]}, and neither an update of its branch nor a rerun gave one. Push an empty commit to it, or close and open it again.",
    )
    return ""
  if not failed:
    if "CANCELLED" not in pr["checks"] or f"rerun {sha}" in noticed or f"{n} {sha}" in deferred.values():
      return ""
    noticed.add(f"rerun {sha}")
    got = await bash(rehead('.conclusion == "cancelled"'), show=HIDDEN, timeout=300)
    ran = got.stdout.content.strip()
    return f"#{n} had a run cancelled at its head {sha[:7]}, and the round ran it again." if ran else ""
  if len(failed) == 1 and f"retry {sha}" not in noticed:
    noticed.add(f"retry {sha}")
    got = await bash(
      rehead('.conclusion == "failure" or .conclusion == "timed_out"', " --failed"), show=HIDDEN, timeout=300
    )
    if got.stdout.content.strip():
      return f"#{n} failed one check alone at {sha[:7]}, and the round ran its failed jobs again."
  if (
    pr["base"] == "main"
    and not red
    and not pr["draft"]
    and not {"hold", "paused"} & set(pr["labels"])
    and not fixing(pr)
    and f"heal update {sha}" not in noticed
    and (n not in updated or clock() - updated[n] >= UPDATED)
  ):
    noticed.add(f"heal update {sha}")
    updated[n] = clock()
    got = await bash(BEHIND.replace("{sha}", sha).replace("{n}", str(n)), show=HIDDEN, timeout=180)
    if "updated" in got.stdout.content:
      warn(pr, f"The round updated the branch of your PR #{n} with main.")
      return f"#{n} failed at {sha[:7]} behind main, which is green, so the round updated its branch."
  return ""


def verdict(pr: dict) -> str:
  checks = pr["checks"]
  if any(c in FAILS for c in checks) or ("CANCELLED" in checks and f"rerun {pr['sha']}" in noticed):
    return "fail"
  if pr.get("open"):
    return "open"
  if pr["head"] == "w/gallery" and all(c in GREEN for c in checks):
    return "pass"
  if GATE in pr.get("names", []) and all(c in GREEN for c in checks):
    return "pass"
  return "pending"


def waits_on(pr: dict) -> str:
  if pr["mergeable"] == "CONFLICTING":
    return ""
  marks = {*pr["labels"], f"#{pr['n']}"}
  return next((d for d, held in awaited.items() if get(d) and not over(d) and marks & set(held)), "")


TRIAGED = "The round folded similar items into this one item, by the ruling of the owner on 10-29: fewer items, and fewer but bigger PRs. Do them in as few PRs as the change allows, each PR one change. covering names the folded keys, so their flakes, orders and steps come to you."


def triage() -> None:
  def lines(text: str) -> set[str]:
    return {x.strip() for x in text.splitlines() if len(x.strip()) >= 40}

  def alike(a: tuple, b: tuple) -> bool:
    if a[2] != b[2] or kind_of(a[0]) != kind_of(b[0]) or model_of(a[0]) != model_of(b[0]):
      return False
    if after.get(a[0]) or after.get(b[0]) or taken(a[0]) not in ("", "queued") or taken(b[0]) not in ("", "queued"):
      return False
    near = set(re.findall(r"#(\d+)", a[1])) & set(re.findall(r"#(\d+)", b[1]))
    la, lb = lines(a[1]), lines(b[1])
    return bool(near) or bool(la and lb and len(la & lb) * 2 >= min(len(la), len(lb)))

  for b in list(queue):
    a = next((one for one in queue if one is not b and queue.index(one) < queue.index(b) and alike(one, b)), None)
    if not a:
      continue
    shared = lines(a[1])
    own = "\n".join(x for x in b[1].splitlines() if x.strip() not in shared)
    text = a[1] if a[1].startswith(TRIAGED) else TRIAGED + "\n\n" + a[1]
    queue[queue.index(a)] = (a[0], text + f"\n\n## The item {b[0]}, folded in by the triage\n" + own.strip(), a[2])
    queue.remove(b)
    covering.setdefault(a[0], []).extend([b[0], *covering.pop(b[0], [])])
    heard.append(f"The triage folded the item {b[0]} into {a[0]}, which it is like.")


def staff() -> None:
  for c in list(parked):
    entry = parked[c]
    if not get(c):
      del parked[c]
    elif len(open_chains()) >= cap():
      break
    elif entry["text"]:
      if room(entry["model"]):
        del parked[c]
        resume(entry["key"], c, entry["model"], entry["text"])
    else:
      del parked[c]
      if paused(c):
        wake(c)
      if entry["name"]:
        working[entry["name"]], models[entry["name"]], hired[entry["name"]] = entry["key"], entry["model"], clock()
      heard.append(f"{entry['key'] or c} on {c} resumed: a slot of the cap of {cap()} chains was free.")

  def met(item: str) -> bool:
    return all(dep in ends and all(n in merged for _, n in ends[dep] if n) for dep in after.get(item, []))

  blocked: set[str] = set()
  for pr in [p for p in prs.values() if tended(p) and p.get("state", table(p)) in TROUBLE and not (assignee(p) or spent(p))]:
    model = mender(pr)
    mine = family(model)
    blocked.add(mine)
    if limits["room"].get(mine, 0) <= 0:
      notice(
        f"no room #{pr['n']} {pr['sha']}",
        f"#{pr['n']} needs a fixer on {model} at {pr['sha'][:7]}, and limits['room'][{mine!r}] is"
        f" {limits['room'].get(mine, 0)}, so no fixer starts on it. Give {mine} room, or say why it waits.",
      )
  for item in [one for one in backlog if one[0] in after and met(one[0])]:
    backlog.remove(item)
    queue.append(item)
  while len(working) + len(queue) < cap():
    ready = [i for i, one in enumerate(backlog) if met(one[0])]
    if not ready:
      break
    queue.append(backlog.pop(ready[0]))
  triage()
  for item in list(queue):
    if met(item[0]) and family(model_of(item[0])) not in blocked and can(item[0]):
      queue.remove(item)
      hire(*item)


def invariants() -> None:
  for decision in [d for d in awaited if over(d) or not get(d)]:
    held = ", ".join(awaited.pop(decision))
    said = "ended" if get(decision) else "names no act"
    heard.append(f"The decision {decision} of the owner {said}, so the round tends {held} again.")
  old: Any = module(ROOT).get("pk_parked")
  for c, e in list(old.items()) if isinstance(old, dict) else []:
    del old[c]
    parked.setdefault(c, {k: str(e.get(k, "")) for k in ("name", "key", "model", "text")})
  held: Any = module(ROOT).get("held_items")
  for key, item in list(held.items()) if isinstance(held, dict) else []:
    del held[key]
    if taken(key) or any(q[0] == key for q in [*queue, *backlog]):
      continue
    if item.get("model"):
      efforts[key] = str(item["model"])
    if item.get("kind"):
      kinds[key] = str(item["kind"])
    after[key] = [str(dep) for dep in item.get("after", [])]
    ruling = str(module(ROOT).get("RULING", ""))
    backlog.append((key, ruling + str(item["brief"]), str(item.get("thread", "defects"))))
    heard.append(f"The item {key} moved from held_items to the backlog, after {', '.join(after[key]) or 'nothing'}.")
  chains = {
    **{str(x[1]): str(x[4]).split("/")[-1] for x in transcript(ROOT) if x[0] == "chain"},
    **{where: key for key, where in homes.items()},
  }
  for pr in prs.values():
    if "paused" in pr["labels"] or not pr["head"].startswith("w/") or owner_of(pr):
      continue
    key = next((m[2:] for m in pr["labels"] if m.startswith("w:")), "")
    if key and homes.get(key):
      continue
    pushed = (
      c
      for c in chains
      if any(x[0] == "bash" and "push" in str(x[4]) and pr["head"] in str(x[4]) for x in transcript(c))
    )
    where = next(pushed, "")
    if not where:
      if f"home #{pr['n']}" not in noticed:
        noticed.add(f"home #{pr['n']}")
        heard.append(f"#{pr['n']} ({pr['head']}) has no home: no worker owns it, and no chain pushed its head.")
      continue
    key = key or chains[where]
    homes[key] = where
    if f"w:{key}" not in pr["labels"]:
      pr["labels"].append(f"w:{key}")
      quietly(
        bash,
        f"gh label create w:{key} --force >/dev/null 2>&1; gh pr edit {pr['n']} --add-label w:{key} >/dev/null 2>&1",
        show=HIDDEN,
      )
    heard.append(f"#{pr['n']} had no home; the round gave it {key} on {where}, which pushed its head.")
  known = {q[0] for q in [*queue, *backlog]}
  held_keys = known | set(working.values()) | set(ends) | {m[2:] for p in prs.values() for m in p["labels"] if m[:2] == "w:"}
  for key, deps in after.items():
    for dep in deps:
      if key in known and dep not in held_keys and f"after {key} {dep}" not in noticed:
        noticed.add(f"after {key} {dep}")
        heard.append(f"The item {key} waits for {dep}, which no queue, worker, PR or end holds.")
  for worker, item in working.items():
    where = working_at(worker)
    if paused(where) and f"paused {where}" not in noticed:
      noticed.add(f"paused {where}")
      heard.append(f"The chain {where} of {item} stays paused after the round woke it.")
  chains = open_chains()
  for c in chains[cap():]:
    name = next((n for n in working if str(get(n)[3]) == c), "")
    key = working.pop(name, "")
    parked[c] = {"name": name, "key": key, "model": models.get(name, ""), "text": ""}
    if not paused(c):
      pause(c)
    quietly(prompt, None, PARKED, on=c)
    heard.append(f"{key or c} on {c} is paused: {len(chains)} chains stand over the cap of {cap()}.")


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
  entries = [x for x in got if isinstance(x, dict)] if isinstance(got, list) else []

  def day(y: int, m: int, d: int) -> int:
    y -= m <= 2
    doy = (153 * (m - 3 if m > 2 else m + 9) + 2) // 5 + d - 1
    yoe = y % 400
    return (y // 400) * 146097 + yoe * 365 + yoe // 4 - yoe // 100 + doy - 719468

  for x in entries:
    old = re.fullmatch(r"(\d\d)-(\d\d)", str(x.get("when", "")))
    if old:
      year = 1970 + int(clock() // 31556952)
      at = day(year, int(old[1]), int(old[2])) * 86400
      x["when"] = str(at if at <= clock() + 172800 else day(year - 1, int(old[1]), int(old[2])) * 86400)
  return entries


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


def settle_flaw(part: str, pr: int) -> int:
  entries = ledger()
  hit = [
    x for x in entries if x.get("state") == "open" and part.lower() in " ".join(str(x.get("what", "")).lower().split())
  ]
  for x in hit:
    x["state"], x["closed_by"] = "closed", f"#{pr}"
  if hit:
    quietly(write, Text(TOOLING, json.dumps(entries, indent=1) + "\n"))
  return len(hit)


PRUNE_PY = """import hashlib, json, os, shutil, subprocess, sys, time

spec = json.load(open(sys.argv[1], encoding='utf-8'))
scratch = os.path.realpath(sys.argv[2])
root = os.path.realpath(os.path.expanduser(spec['root']))
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
  listed = run('git', 'status', '--porcelain', cwd=path)[1]
  pushed = not listed.strip() and not run('git', 'rev-list', '-1', branch, '--not', '--remotes')[1].strip()
  if code != 0 or not (pushed or state in ('MERGED', 'CLOSED') or (not ahead and state != 'OPEN')):
    continue
  touched = [line[3:].split(' -> ')[-1].strip('"') for line in listed.splitlines() if len(line) > 3]
  files = [os.path.join(path, x) for x in touched]
  if not pushed and max([age(path), *(age(x) for x in files if os.path.lexists(x))]) >= old:
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
budgeted: dict[str, list[int]] = globals().setdefault("budgeted", {})
flaky: dict[str, list[str]] = globals().setdefault("flaky", {})
runs_read: set[int] = globals().setdefault("runs_read", set())
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
  return json.loads(gh("api", f"repos/{{owner}}/{{repo}}/actions/runs/{run}{at}/jobs?per_page=100", "--jq", "[.jobs[] | {id, name, conclusion, started_at, completed_at, steps: [(.steps // [])[] | {name, conclusion, started_at, completed_at}]}]"))
def spent(job):
  stamp = lambda s: datetime.fromisoformat(s.replace("Z", "+00:00")).timestamp()
  return int(stamp(job["completed_at"]) - stamp(job["started_at"]))
budgets = json.loads(subprocess.run(["git", "show", "origin/main:.github/budgets.json"], capture_output=True, text=True).stdout or "{}")
path = "repos/{owner}/{repo}/actions/workflows/gates.yml/runs"
green = json.loads(gh("api", f"{path}?branch=main&status=success&per_page=5", "--jq", "[.workflow_runs[] | {id, at: (.created_at | tostring), attempt: .run_attempt}]"))
measured, steps, passed = {}, {}, {}
for run in green:
  for job in jobs(run["id"]):
    if job["conclusion"] == "success":
      passed.setdefault(run["id"], set()).add(job["name"])
      measured.setdefault(job["name"], []).append(spent(job))
      for step in job["steps"]:
        if step["conclusion"] == "success" and step["completed_at"] and step["started_at"]:
          steps.setdefault(job["name"], {}).setdefault(step["name"], []).append(spent(step))
over = [{"job": n, "median": int(statistics.median(v)), "budget": budgets[n], "runs": [r["id"] for r in green], "steps": {s: int(statistics.median(x)) for s, x in steps.get(n, {}).items()}} for n, v in measured.items() if n in budgets and len(v) >= 3 and statistics.median(v) > budgets[n]]
runs = json.loads(gh("api", f"{path}?per_page=30", "--jq", "[.workflow_runs[] | {id, sha: .head_sha, at: (.created_at | tostring), attempt: .run_attempt, conclusion, url: .html_url}]"))
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
        over_row = re.match(r"(\S.*?)\s+-?[\d.]+\s+\S+.* OVER$", line)
        hit = hit or (over_row and re.match(r"(.+)", "bench: " + over_row[1]))
        if hit and hit[1] not in tests:
          tests.append(hit[1])
      newer = [g for g in green if g["at"] > run["at"] and g["attempt"] == 1 and job["name"] in passed.get(g["id"], ())]
      flakes.append({"tests": tests, "job": job["name"], "sha": run["sha"], "url": f"{run['url']}/attempts/{attempt}", "stale": len(newer) >= 5})
print(json.dumps({"over": over, "flakes": flakes, "read": settled}))
'''

BUDGET_BRIEF = """The job {job} of the gates took {median} s in the median of its last green runs on main, {runs}, over its budget of {budget} s in .github/budgets.json, so each loop of feedback waits longer for it. Find where the job spends its time, from the times of its steps in gh run view --json jobs and from its logs, and bring the job under its budget at the cause: a cache that misses, a build done twice, or a test that waits. Never drop or weaken a gate to save time: move it, share it or make it faster. Change a budget only with the measures of script/budgets.py, and say why in the PR.
The median of each step of the job over those runs, in seconds:
{steps}
A change to CI itself is proved only by the gates of its PR: after you open the PR, wait for them with await checked(number), again after a pending, before you close."""

FLAKE = """The gates failed, then passed when they ran again, on the same commit {sha}, so a test or a job is flaky, and a flake hides a defect and slows each loop of feedback. In the job {job}, at {url}, it failed in:
{tests}
The ledger of the flakes holds every attempt that failed this way: {seen}.
First check origin/main: when the same failure shows at no run newer than the commit that fixed its cause, the cause is fixed, so change nothing and close with 0.
No check of the app runs outside CI, and gh workflow run gates.yml --ref <your branch> starts the gates on your branch by hand, as often as you need, to show the cause before and after your fix.
Read the log of the attempt that failed: gh run view <run> --attempt <attempt> --log-failed. Find the cause, and prove it with a command that shows it, before and after your fix. {many}Fix it at its cause: never with a skip, a retry, a longer bound or a weaker assertion."""

MANY_SAID = "More than {n} tests failed in one attempt, so one cause, as a machine that is slow or a shared resource that waits, probably holds them all: find it first. "


def coverer(*names: str) -> str:
  return next((k for k, held in covering.items() if set(names) & set(held)), "")


def taken(key: str) -> str:
  for one in [key, coverer(key)]:
    got = next((w for w, k in working.items() if one and k == one), "")
    if got or any(one and q[0] == one for q in queue):
      return got or "queued"
  return ""


async def watch_ci() -> None:
  def slug(text: str, size: int = 34) -> str:
    return "-".join(re.findall(r"[a-z0-9]+", text.lower()))[:size].strip("-")

  def holder(names: list[str]) -> tuple[str, str]:
    k = coverer(*names)
    if k:
      return k, next((w for w, one in working.items() if one == k and not over(w)), "")
    named = [x for x in names if len(x) >= 8]
    for k, text, _ in [*queue, *backlog]:
      if any(x in text for x in named):
        return k, ""
    for worker, k in working.items():
      if not over(worker) and any(x in briefed.get(k, "") for x in named):
        return k, worker
    for up in prs.values():
      k = next((m[2:] for m in up["labels"] if m.startswith("w:")), "")
      if k and any(x in briefed.get(k, "") for x in named):
        return k, ""
    return "", ""

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
    said = "\n".join(f"- {s}: {x} s" for s, x in sorted(one["steps"].items(), key=lambda kv: -kv[1])[:15]) or "- none measured"
    brief = BUDGET_BRIEF.format(job=job, median=one["median"], runs=", ".join(map(str, ran)), budget=one["budget"], steps=said)
    queue.append((key, brief, "ci"))
    heard.append(f"The job {job} took {one['median']} s, over its budget of {one['budget']} s: queued as the item {key}.")
  for one in seen["flakes"]:
    tests = [str(x) for x in one["tests"]]
    many = len(tests) > MANY
    name = str(one["job"]) if many or not tests else tests[0]
    if str(one["url"]) not in flaky.setdefault(name, []):
      flaky[name].append(str(one["url"]))
    if seed:
      continue
    if one.get("stale"):
      heard.append(f"The gates flaked at {one['url']} in {name[:120]}, but the job passed at the first try in the 5 newest runs of main, so the round queued no item.")
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
    held, worker = holder(tests or [name])
    if held:
      if worker:
        quietly(prompt, None, f"Your item came again, at {one['url']}:\n{listed}", on=working_at(worker))
      else:
        heard.append(f"The gates flaked at {one['url']} in {name[:120]}, and the item {held} names the same test, so the round queued no new item.")
      continue
    queue.append((key, brief, "ci"))
    heard.append(f"The gates flaked at {one['url']} in {name[:120]}: queued as the item {key}.")


async def linking() -> None:
  """Each chain of stacked PRs of the round is one gh stack on GitHub, so it shows linked and merges whole."""
  for pr in sorted(prs.values(), key=lambda p: p["n"]):
    if pr["base"] != "main" or not pr["head"].startswith("w/"):
      continue
    layers = [pr]
    while len(ups := [up for up in prs.values() if up["base"] == layers[-1]["head"]]) == 1:
      layers.append(ups[0])
    numbers = " ".join(str(up["n"]) for up in layers)
    if len(layers) < 2 or f"linked {numbers}" in noticed:
      continue
    noticed.add(f"linked {numbers}")
    got = await bash(f"gh stack link {numbers} 2>&1", show=HIDDEN, timeout=120)
    said = " ".join(got.stdout.content.split())[-200:]
    names = " <- ".join("#" + str(up["n"]) for up in layers)
    linked = got.code == 0 and "✗" not in got.stdout.content
    heard.append(f"The stack {names} " + ("is linked on GitHub." if linked else f"was not linked: {said}"))


deferred: dict[str, str] = globals().setdefault("deferred", {})

THROTTLE = 6


async def throttle() -> None:
  """Each sweep cancels the queued runs of the gates that can serve no merge, so the runners take those that can."""
  got = await bash(
    "gh run list --workflow gates.yml --status queued --limit 100 --json databaseId,headBranch,headSha,event",
    show=HIDDEN,
    timeout=120,
  )
  if got.code != 0 or not got.stdout.content.strip().startswith("["):
    return
  runs = json.loads(got.stdout.content)
  heads = {pr["head"]: pr for pr in prs.values()}
  cut: list[tuple[dict, str]] = []
  for run in runs:
    if run.get("event") != "pull_request" or run["headBranch"] == "main":
      continue
    pr = heads.get(run["headBranch"])
    below = next((p for p in prs.values() if pr and p["head"] == pr["base"]), None)
    why = ""
    if not pr:
      why = "its PR is closed"
    elif run["headSha"] != pr["sha"]:
      why = "its PR has a newer head"
    elif pr["draft"] or {"hold", "paused"} & set(pr["labels"]):
      why = "its PR is a draft or held"
    elif pr["mergeable"] == "CONFLICTING":
      why = "its PR conflicts"
    elif below and below["verdict"] != "pass":
      why = "a layer below it is not green yet"
      deferred[str(run["databaseId"])] = f"{pr['n']} {pr['sha']}"
    if why:
      cut.append((run, why))
      if str(run["databaseId"]) not in deferred:
        noticed.add(f"rerun {run['headSha']}")
  if cut:
    await bash("; ".join(f"gh run cancel {run['databaseId']} >/dev/null 2>&1" for run, _ in cut), show=HIDDEN, timeout=120)
    said = ", ".join(f"#{(heads.get(run['headBranch']) or {}).get('n', run['headBranch'])} ({why})" for run, why in cut)
    heard.append(f"The round cancelled queued runs of the gates that could serve no merge: {said}.")
  if len(runs) - len(cut) >= THROTTLE:
    return
  for one, key in list(deferred.items()):
    n, sha = key.split()
    pr = prs.get(int(n))
    below = next((p for p in prs.values() if pr and p["head"] == pr["base"]), None)
    if pr and pr["sha"] == sha and below and below["verdict"] != "pass":
      continue
    del deferred[one]
    if pr and pr["sha"] == sha:
      await bash(f"gh run rerun {one}", show=HIDDEN, timeout=120)


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
  for one in [x for x in pool.values() if x["pr"] and x["folders"] is None]:
    files = await bash(f"gh pr view {one['pr']} --json files --jq '.files[].path'", show=HIDDEN, timeout=120)
    if files.code == 0:
      one["folders"] = sorted({x.split("/")[0] for x in files.stdout.lines if x.strip()})
  left = shift()
  for one in prs.values():
    if one["mergeable"] == "UNKNOWN":
      await probe(one)
  restacked = await restack_forced()
  for kid, old in [(kid, old) for kid, old in left if tended(kid) and kid["n"] not in restacked]:
    # The parent merged outside the round, so GitHub moved the base of the child to main with the commits of the
    # parent still in it.
    restacked.add(kid["n"])
    await restack(old, "main", kid)
  await linking()
  await throttle()
  main = json.loads(parts[2])
  red = "failure" in main["runs"]
  tip = str(main["sha"])
  if not red:
    redmain.clear()
  elif tip not in redmain:
    failed = await bash(REDLINES.replace("{sha}", tip), show=HIDDEN, timeout=300)
    redmain.clear()
    redmain[tip] = list(failed.stdout.lines)
  if red:
    # A PR whose failed lines all fail on main too waits on main: no order and no fixer can fix them.
    for one in [p for p in prs.values() if p["verdict"] == "fail" and p["head"].startswith("w/")]:
      if "paused" not in one["labels"] and one["mergeable"] != "CONFLICTING" and not waits_on(one):
        await on_main(one)
  if red and f"red {tip}" not in noticed:
    waiting = [n for n in sorted(prs) if table(prs[n]) == "main"]
    noticed.update(f"waits {n} {prs[n]['sha']}" for n in waiting)
    names = ", ".join(f"#{n}" for n in waiting)
    notice(
      f"red {tip}",
      f"main is red at {tip[:7]}. Find its cause first. The lines that failed:\n"
      + ("\n".join(redmain[tip]) or "none that the filter knows: read gh run view --log-failed")
      + (
        f"\nThese PRs fail only those lines, so they wait on main with no order and no fixer, and the round updates"
        f" their branches once main is green: {names}."
        if waiting
        else ""
      ),
    )
  now = clock()
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
    if pr.get("files") == 0 and not pr["draft"]:
      said = "🤖 This PR changes no file: main holds all of its change, so the round closes it."
      await bash(f"gh pr close {n} --comment {shq(said)}", show=HIDDEN, timeout=120)
      heard.append(f"#{n} ({pr['head']}) changed no file, so the round closed it.")
      continue
    pr["state"] = await mend(pr, now, red)
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
  for n in [k for k in assigned if k not in prs]:
    del assigned[n]
  for n in [k for k in onmain if k not in prs]:
    del onmain[n]
  for pr in [p for p in prs.values() if tended(p) and p.get("state") in TROUBLE]:
    if not assignee(pr):
      cause = (
        "two fixers ended at this head and did not move it"
        if spent(pr)
        else f"{mender(pr)} has no room"
        if not room(mender(pr))
        else "the round sent no order"
      )
      notice(
        f"orphan #{pr['n']} {pr['sha']}",
        f"#{pr['n']} is {pr['state']} at {pr['sha'][:7]} and no worker acts on it, a defect of the round: {cause}.",
      )
  if parts[3] and noted and parts[3] != noted[0]:
    old = set(noted[0].splitlines()) | ours()
    added = [x for x in parts[3].splitlines() if x.strip() and x not in old]
    if added:
      quietly(prompt, None, "The owner changed DECISIONS.md:\n" + "\n".join(added[:60]))
  if parts[3]:
    noted[:] = [parts[3]]
  await take_issues()
  await upstream_watch()
  repo = ops()[0]
  if repo:
    watched = await bash(OPS_WATCH.replace("{ops}", repo), show=HIDDEN, timeout=300)
    wrote = [
      p for p in (x.split("\t", 3) for x in watched.stdout.content.splitlines()) if len(p) == 4 and p[1] not in comments
    ]
    comments.update(p[1] for p in wrote)
    fresh = [p for p in wrote if p[2] == OWNER and not p[3].startswith(MARKS)]
    if fresh and "ops" in timers:
      quietly(
        prompt,
        None,
        "The owner wrote in the private repository of the decisions:\n"
        + "\n".join(f"ops#{p[0]} {p[3][:1500]}" for p in fresh)
        + merged_since(),
      )
    timers["ops"] = clock()
  await mark()
  await watch_ci()
  if now - timers.get("review", now - every) >= every:
    await review("review" not in timers)
    timers["review"] = now
  if now - timers.setdefault("audit", now) >= 12 * every:
    await audit(red)
    timers["audit"] = now


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
      for at in ("tend", "sweep", "staff", "invariants"):
        try:
          if at == "tend":
            tend()
          elif at == "sweep":
            await sweep(every)
          elif at == "staff":
            staff()
          else:
            invariants()
        except Exception as fault:
          notice(
            f"fault {at} {type(fault).__name__}",
            f"The round on {here} met a fault in its step {at}: {type(fault).__name__}: {str(fault)[:1000]}. The round goes"
            " on at the next round, and tells this kind of fault once.",
          )
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


THREADS = 3000


def threads() -> str:
  chains: list[str] = []
  asks: list[tuple] = []
  tells: dict[str, str] = {}
  todo, seen = [ROOT], set()
  while todo and len(chains) < THREADS:
    one = todo.pop()
    if one in seen:
      continue
    seen.add(one)
    chains.append(one)
    notes: list[str] = []
    for x in transcript(one):
      if x[0] == "chain":
        todo.append(str(x[1]))
      elif x[0] == "prompt" and x[4] != "None":
        asks.append(x)
      elif x[0] == "tell":
        notes.append(str(x[3]))
    tells[one] = "".join(notes)
  tops = {str(x[1]): x for x in asks if (get(str(x[3])) or ("", "", ""))[2] == x[2]}
  paths: dict[str, str] = {}
  ups: dict[str, str] = {}
  kids: dict[str, list[str]] = {}
  waits: dict[str, list[str]] = {}
  drawn: set[str] = set()
  out: list[str] = []

  def parent(of: str) -> str:
    return str((get(of) or ("", "", ""))[2])

  def up(of: str) -> str:
    if of not in ups:
      cur, hops = parent(of), 0
      while cur and cur not in tops and hops < 100:
        cur, hops = parent(cur), hops + 1
      ups[of] = cur if cur in tops else ""
    return ups[of]

  def path(p: str) -> str:
    if p not in paths:
      label, above = str(get(str(tops[p][3]))[4]), up(p)
      paths[p] = label if "/" in label or not above else path(above) + "/" + label
    return paths[p]

  def links(p: str) -> list[str]:
    found: list[str] = []
    where = str(tops[p][3])
    told_text = str(peek(p, "")) + (
      tells[where] if where in tells else "".join(str(x[3]) for x in transcript(where) if x[0] == "tell")
    )
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
    if p in drawn:
      return
    drawn.add(p)
    pad, where = "  " * depth, str(tops[p][3])
    state = "done" if over(p) else "paused" if paused(where) else "working"
    out.append(f"{pad}{path(p)} {state} {p} on {where}")
    out.extend(f"{pad}  PR {u} {state_of(u)}" for u in links(p))
    out.extend(f"{pad}  waits {w}" for w in waits.get(p, []))
    out.extend(f"{pad}  {thread}/{key} queued" for key, _, thread in queue if thread == path(p))
    for q in kids.get(p, []):
      draw(q, depth + 1)

  by_path: dict[str, list[str]] = {}
  for p in tops:
    by_path.setdefault(path(p), []).append(p)
  for p in tops:
    segs = path(p).split("/")
    best = next(("/".join(segs[:k]) for k in range(len(segs) - 1, 0, -1) if "/".join(segs[:k]) in by_path), "")
    if best:
      best = by_path[best][0]
    else:
      best = "" if "/" in str(get(str(tops[p][3]))[4]) else up(p)
    kids.setdefault(best, []).append(p)
  for x in asks:
    if str(x[1]) not in tops and not over(x[1]):
      waits.setdefault(up(str(x[1])), []).append(f"{x[1]} on {x[3]} to {x[6] or 'its model'}: {str(x[5])[:200]}")
  for p in kids.get("", []):
    draw(p, 0)
  placed = set(paths.values())
  out.extend(f"{thread or 'w'}/{key} queued" for key, _, thread in queue if thread not in placed)
  out.extend(f"waits {w}" for w in waits.get("", []))
  return "\n".join(out)
