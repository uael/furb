"""The conductor of the converge: one home for each rule of the round that the desk of the lead ran."""

import re
from collections.abc import Callable, Generator
from typing import Any

from furb.engine import (
  HIDDEN,
  OPERATOR,
  ROOT,
  WINDOW,
  Act,
  Exit,
  Text,
  act,
  bash,
  chain,
  clock,
  close,
  controlled,
  covers,
  dump,
  filled,
  get,
  offered,
  operated,
  over,
  pause,
  paused,
  peek,
  prompt,
  question,
  scope,
  standing,
  transcript,
  under,
  wait,
  wake,
  write,
)

# The values that the lead rules: a load of the kit keeps the values bound before it.
ROOM_CAP: dict[str, int] = globals().setdefault("ROOM_CAP", {"opus/high": 3, "opus/medium": 3, "sonnet/high": 3, "sonnet/medium": 3, "gpt-6-astra/high": 2})
TOPICS: list[str] = globals().setdefault("TOPICS", ["engine", "host", "tui", "skills", "rust", "ci", "tidy", "review", "qa"])
desk_width: dict[str, int] = globals().setdefault("desk_width", dict.fromkeys(TOPICS, 1))
FIX_MODEL: str = globals().setdefault("FIX_MODEL", "claude-cli:sonnet/high")
AGG_MODEL: str = globals().setdefault("AGG_MODEL", "claude-cli:opus/medium")
ASTRA: str = globals().setdefault("ASTRA", "codex:gpt-6-astra/high")
VERIFIER: str = globals().setdefault("VERIFIER", "claude-cli:opus/medium")
SERIAL2: list[str] = globals().setdefault("SERIAL2", [])
POLISH_KEYS: list[str] = globals().setdefault("POLISH_KEYS", ['agents-arrow-gutter', 'answer-cut-no-rest', 'config-panel-look', 'hover-highlight', 'loader-like-peers', 'loading-shot-no-loader', 'markdown-links-click', 'markdown-shot-overlay', 'more-chars-trailer', 'part-error-raw', 'pinned-header-rung', 'quiet-row-rung-id', 'setup-above-placeholder', 'thinking-gutter-mark', 'tui-polish', 'two-running-marks'])
RANK: dict[str, int] = globals().setdefault("RANK", {'opus/high': 1, 'opus/medium': 2, 'gpt-6-astra/high': 2, 'sonnet/high': 3, 'sonnet/medium': 4})

AGG, QA, REVIEW = "converge", "qa", "review"
CODE = ("engine", "host", "tui", "skills", "rust", "ci")
CLASSES = {one: f"claude-cli:{one}" for one in ("opus/high", "opus/medium", "sonnet/high", "sonnet/medium")}
OPUS_M, SONNET_M = CLASSES["opus/medium"], CLASSES["sonnet/medium"]
UP = {'sonnet/medium': 'sonnet/high', 'opus/medium': 'opus/high'}
EFFORT_UP = {"medium": "high", "high": "xhigh"}
EVERY, FLOOR, CI_EVERY, DISK_EVERY, HOUR = 300.0, 900.0, 120.0, 900.0, 3600.0
STALL, STUCK, STUCK_RUNNING, ROUNDS, MERGES = 600.0, 1200.0, 2700.0, 3, 4
DISK_FLOOR = 60
ROOM_WHY, FIX_WHY = "past the room of its class", "a fix of the CI of w/converge runs first on this topic"
DF = "df -k ~ | tail -1"
MEM = "memory_pressure | tail -1; sysctl -n vm.swapusage"
GH_POLL = 'sh "$TMPDIR/converge/gh.sh"'
SHOTS = 'sh "$TMPDIR/converge/shots.sh"'
REGROUND, REGROUND_TOKENS, REGROUND_GROWTH = 0.5, 300000, 150000
NO_VERIFY = '^(audit|review|qa-run|words-audit|perf-|libs-audit|align-audit|fold)'
URGENT_NOTE = '(?m)The disk|The memory runs low|not applied|\\bP0\\b|checks of CI of w/converge pass|push of the captures'
CI_PROBE = 'gh run list --repo uael/furb --branch w/converge --workflow gates.yml --limit 1 --json headSha,status,conclusion,databaseId --jq \'.[0] | "\\(.headSha) \\(.status) \\(.conclusion) \\(.databaseId)"\''
QA_WRAP = 'The lead: a cycle of the aggregator starts now, and its merges and its build change the worktree that you read. Run no new command: close at once with the rows of what you found, and name what you did not reach.'
CLAIM = " Judge the row against main first: it is a claim of an audit."
FIX_HEAD = 'A failure of the CI of w/converge in a path that this topic owns. Follow the part A fix of CI of scratch://converge/method.md. The brief of the aggregator:\n\n'
CYCLE = 'One cycle of the aggregator. The charter of this chain, scratch://converge/converge.md, holds your rules and the paths of each topic.\n1. In your worktree: git fetch origin, then list each topic branch origin/c/<topic> that holds commits that w/converge lacks. Merge the branch that each topic pushes, since an item can push from a scratch worktree and leave the local branch c/<topic> of this clone behind.\nThen, for each topic whose local branch c/<topic> holds commits that origin/c/<topic> lacks, push it with git push origin c/<topic> when that is a fast-forward, and merge it in step 2; route it to its topic when it is not.\n2. Merge them all in one command, so that the hooks run once, on the merged diff: git merge --no-ff --no-edit origin/c/<topic> origin/c/<topic> ..., with a timeout of 2400 s in bash. Never pass --no-verify or SKIP=.\n- When git stops on a conflict, run git merge --abort, then merge the topics one by one in the order engine, host, tui, skills, rust, ci, each with its hooks. Resolve a conflict by the meaning of both sides, never by a pattern on its markers. A conflict that needs the knowledge of a topic goes to that topic as a failure: abort that merge and route it.\n- When a hook refuses the merge, run git merge --abort, route the failure to the topic that owns the failing path, and merge the other topics again in one command.\n3. When the merge changes the source of a generated file, make the file again (bun run protocol, bun run build, bun run docs) and commit it, with its hooks. Then apply each line of scratch://converge/converge-needs.md that is not marked done, as one commit of the paths that you own, and mark it done there with its sha.\n4. Push w/converge. While no PR stands for it, open one ready, never a draft, with the label hold so that it never merges by itself: await pr(title, why, outcome, checks, where=<your worktree>, labels="hold") of the skill pr, whose rules the template gives. Its body says that the branch converges the work of the owner since 11-03, topic by topic, and lists the PRs that it carries. Update the body when a cycle adds a topic or a fold, with one line "Fixes #N." for each issue that scratch://converge/issues.md says that w/converge carries, once the commits that fix it stand on the branch.\n5. Build the pushed head for the window of the owner, the QA and the benches, in one command: cargo build -p furb-cli && bun run build. Leave the worktree clean at the pushed head, with no merge in progress, until the next cycle: the QA reads that build.\n6. Close with a tuple: a dict that maps each topic to the brief of each failure of this cycle (a conflict or a hook that refused the merge, with the head, the failing test or file, and the lines that show the cause), with each failure routed once, and a summary in 100 words at most: the commits merged from each topic, the head pushed, and each topic held back.\nRun no other check, and read no check of the PR: the CI tests the pushed head, and the desk routes each of its failures to its topic as soon as the run ends.'
FINGER = 'cd ~/src/github.com/uael/furb || exit 0\nfor b in c/engine c/host c/tui c/skills c/rust c/ci; do echo "$b $(git rev-parse --short "$b" 2>/dev/null)"; done\nn=$(gh pr list --repo uael/furb --head w/converge --state open --json number --jq \'.[0].number // empty\' 2>/dev/null)\necho "pr ${n:-none}"\n[ -n "$n" ] && gh pr checks "$n" --repo uael/furb --json bucket --jq \'[.[] | .bucket] | group_by(.) | map("\\(.[0]) \\(length)") | join(", ")\' 2>/dev/null\n[ -n "$n" ] && gh pr view "$n" --repo uael/furb --json headRefOid --jq \'"head " + .headRefOid[0:9]\'\ntrue\n'
QA_BRIEF = "A QA run of the tip of w/converge, by the charter of this chain. If origin/w/converge stands at the sha of the last run in scratch://converge/qa.md and no live walk is due, close at once with: same head <sha>; when a live walk is due, run the live walk alone on that head. Else run it, and append to scratch://converge/qa.md ($TMPDIR/converge/qa.md) the sha, the minutes that the run took, the result of the scenarios, and each finding with its capture. Close with the rows of the new findings, or with clean, the sha and the minutes. Once an hour (the last run of qa.md that names a live walk is older than an hour), walk also a real life on claude-cli:haiku/low in a new session of a workspace under $TMPDIR, with a daemon of its own (XDG_CACHE_HOME under $TMPDIR): a message and its answer, a thread, a cancel and /clear, and name it a live walk in qa.md. In each run, scroll a long thread with large blocks of words from its top to its end and back, and judge whether it jumps or stalls. Each run also scrolls a long thread that holds large word blocks from its top to its end and back, and reads as an image each capture that a finding names; when you cannot read an image, write that in qa.md as a blocker. Read the build of the worktree of the aggregator, where this chain stands, at its pushed head: run no build there, no git command that moves it, and write nothing in it. A cycle of the aggregator ended just before this run, and the next one starts in about 15 minutes: when the lead tells you that it starts, close at once with your rows, since its merges change the worktree. Give each finding as a row for its topic, and leave its cause to the topic that takes it. scratch://converge/qa.md is the ledger of the QA: before you run anything, read its last entry, whose handoff tells what works, what fails and what was not reached. Check again only what the commits since its head touch (git log --stat <its head>..origin/w/converge), what it names as failing or not reached, and each row that the lead lists below as ended by a topic; never repeat a check of a path that no commit changed. End your entry with a handoff of at most 15 lines for the next run: what works and needs no check until its paths change, what still fails with its row, what you did not reach, and what to try first. Before you run anything, read the QA trailers of the commits since the head of the last entry of the ledger: git log --format='%h %s%n%(trailers:key=QA,valueonly)' <that head>..origin/w/converge. Test on this build first each QA line that the handoff of the last entry names as not reached, oldest first, then each of these, and name in your entry each one with its commit sha: passed, or the row of what failed, with that sha in the row. A QA line stays in the handoff as not reached until a run tests it. Each run also judges the screens in one batch: run bun run screenshots on this build with FURB_GALLERY_TEXT and FURB_GALLERY_CELLS to $TMPDIR/qa/<sha>, copy the PNGs there, and restore docs/screenshots of the worktree to its head. Hash each PNG, and read as an image each one whose hash changed since the last run of the ledger, and each one that a pending item names. Judge them against the pending items of tui in scratch://converge/desk.md, the QA lines of the new commits, and the open asks of scratch://converge/asks-state.md: name each pending item that a capture shows done, with the capture; give a row for each one that a capture shows still wrong; and give a row for each new flaw of design that a capture shows. Keep the table of hashes in the ledger. Test also, a few in each run, the QA lines of scratch://converge/asks-state.md that no run passed yet, oldest first, and mark each one passed or failed with its id in the ledger."
REVIEW_BRIEF = 'The review of the paths of the topic {t}, top 3 and the bar, read only. Run git fetch origin. Read each change that the converge made in the paths that scratch://converge/{t}.md gives to {t}, whichever topic made it: git log --no-merges -p <base>..origin/w/converge -- <paths>, where <base> is the sha of origin/w/converge that the last entry of scratch://converge/review-{t}.md names (an entry that names only a sha of c/{t} counts as none, so take origin/main),, the resolutions of the merges in those paths (git log -p --cc --merges <base>..origin/w/converge -- <paths>), and then the commits of the topic that w/converge does not hold yet: git log --no-merges -p origin/w/converge..origin/c/{t}. Review each change as an expert reviewer, by the charter of this chain. Append to scratch://converge/review-{t}.md ($TMPDIR/converge/review-{t}.md) the sha of origin/w/converge and of origin/c/{t} that you read, a verdict (clean, or the count of findings), and each finding with its commit, its path and lines, the rule that it breaks and its fix. Close with the rows of the findings, or with clean and the two shas.'
WORDS_BRIEF = 'The audit of the words of the owner, top 10 and the bar, read only. Read whole: scratch://converge/words-life.md (each message of the owner in this life and each of his answers, in order), ~/Library/Caches/furb/handoff/converge/plan.md, the picks (~/Library/Caches/furb/handoff/converge/decisions/picks.md, with its rulings N, N2, N3, A, O and W), scratch://converge/answers.md (the answers of the lead to earlier audits) and scratch://converge/desk.md (the code and the values that run the converge now: the desk, its queues, the method of the topics, the head of a fix and the brief of the aggregator). In the ledger ~/Library/Caches/furb/handoff/converge/asks.json, read the asks that these words name or change. For each ask in these words (an order, a rule, a number, a choice, or a question that wants a change), find what serves it: a line of the code or a value of desk.md, a queued or ended item, a commit on origin (git log origin/w/converge and origin/c/*), or a file of scratch://converge/. A later word of the owner rules over an earlier one. Write scratch://converge/words-audit.md ($TMPDIR/converge/words-audit.md): the missing and partly served asks first, then a table of each ask with its source id and its state (served, with where; queued, with the item; partly, with what lacks; missing; or overruled, with the later word). State each ask in plain English and quote no words. Close with the missing and partly served asks, one line each: the source id, the ask in one sentence, and the change that serves it (an item of a topic with its class of model and its paths, or a change of the desk for the lead). Start your close with one line: <n> asks: <a> served, <b> queued, <c> partly, <d> missing, <e> overruled.'
ALIGN_BRIEF = 'The audit of alignment, top 10 and the bar, read only. Read the ledger of the asks (~/Library/Caches/furb/handoff/converge/asks.json), the top 10 (~/Library/Caches/furb/handoff/converge/top10.md) and the picks (~/Library/Caches/furb/handoff/converge/decisions/picks.md). Then run git fetch origin and read the commits of w/converge since the last audit: git log --no-merges --stat <base>..origin/w/converge, where <base> is the sha that the last entry of scratch://converge/align.md names, else origin/main, and git show for each commit that serves an ask of the top 3. Judge whether each change goes the way of its ask or drifts from it. Name each change that serves no ask or goes against one, and name the asks of the top 3 of the top 10 that no change serves yet. Append to scratch://converge/align.md ($TMPDIR/converge/align.md) the sha that you read and these lists. Close with the rows of the drifts and of the gaps, by the charter of this chain.'
PERF_BRIEF = 'The census of speed, again, read only: by the charter of this chain, compare with the last entries of scratch://converge/perf.md and scratch://converge/ci-time.md. Read the times of the jobs and steps of the latest runs of the gates on w/converge (gh run list, then gh api repos/uael/furb/actions/runs/<id>/jobs), the numbers of bun run bench in the log of the job of the speed of the TUI, the slowest tests that the logs now print (pytest --durations and the times of the cargo binaries), and the token walls of engine.py and DOCTRINE.md. Append to scratch://converge/perf.md the sha, each measure against the last entry, each regression past a fifth and each gain. Close with a row for each regression and each new lever.'
TRIAGE_NEW = 'Each row gets an after list of the keys of the items that it waits on: an item that changes a path that another queued or live item moves or redesigns waits on that item, and an item that builds on the result of another waits on it. A wide cut, as the rename of extension to skill (skill-rename-cut), comes before each row that touches a path that it moves.\n\nNew rows for the queue. Read scratch://converge/triage/queue.md again, the queue as it stands now, and scratch://converge/words-life.md for the latest words of the owner. By the charter of this chain, give each new row below its priority, its class and its complete brief. Merge a new row into a queued item that one commit of its topic serves: keep the key of the queued item, give its whole new brief, and list the new key in absorbs; or merge two new rows. Drop only with a reason. Keep each after key valid. Close with close((items, dropped)), each new key present once; an item may carry the key of a queued item that it changes, never the key of a live one.\n\n'
DEPS_ASK = 'Rule the dependencies of the queue of the converge, which you own: the desk starts an item only once a cycle merged each item of its after list, and the lead no longer edits an after list. Read each row below (the live items, then each queued item with its priority, its after list and the head of its brief) against your charter. Close with a dict that maps the key of each queued item to the list of keys that it must wait on: an item that changes a path that a live or queued item moves or redesigns waits on it, an item that builds on the result of another waits on it, and a wide cut comes before each row that touches a path that it moves. Each key of a list is the key of a live or a queued item, or of an item that ended with a commit. No list makes a cycle. Give an empty list to an item that waits on nothing. Hard first: a P0 or P1 item waits on a lower one only when it needs its result. Why the desk asks now:\n'
VERIFY_BRIEF = 'Verify one item of the converge against the ask of the owner, as an independent judge. The item counts as done only when you answer yes.\nFirst read the part What done means of scratch://converge/method.md, and run skill("ponytail-review").\nThe item KEY of the topic TOPIC. Its brief:\nBRIEF\n\nIts report:\nREPORT\n\nCheck by reading alone: git fetch origin, git show, git log and git grep in /Users/uael/src/github.com/uael/furb-converge. Make no change, no build and no merge there.\n1. The commit that the report names stands on origin/c/TOPIC or origin/w/converge, and its diff does what the brief asks, at the cause, in the least code, with no hack, no second home of a truth, and no quote of a person.\n2. For a report of the owner, the report shows his symptom on his path before the fix and gone after it, with the command and its output. A unit test of the item\'s own reading alone proves nothing.\n3. A change that the owner can see carries a QA line with his steps.\n4. A close with no commit holds only when the report proves, with a command and its output on origin/w/converge, that the claim does not hold or that w/converge already serves the ask.\nClose with the one word yes when each check holds. Else close with the cause: each check that fails, what is missing, and the smallest work that makes it hold, in at most 120 words.'
PROOF_BRIEF = 'Prove on his path the asks of the owner that stand committed with no proof, in a batch. A later run takes the next batch.\nSetup, once for each head, in one word. The tree stands under /tmp, since a socket path under the scratch of the life passes 103 bytes and no scenario runs there; remove each older furb-proof tree of this chain with git worktree remove --force once the new one stands:\ncd /Users/uael/src/github.com/uael/furb-converge && git fetch -q origin && head=$(git rev-parse --short=9 origin/w/converge) && d=/tmp/furb-proof-$head && { [ -d "$d" ] || sh script/worktree.sh "$d" "proof/$head" "$head"; }\nscript/worktree.sh copies the build of the aggregator on write, so build nothing more there unless a test says that the build is stale. Work in that tree alone: never in furb-converge, the topic worktrees, or furb-try-930ed03c9, which runs a window of the owner. Each life that you open runs with a daemon of its own (XDG_CACHE_HOME under $TMPDIR/proof/<head>) in a workspace under $TMPDIR, on claude-cli:haiku/low.\nRead scratch://converge/asks-state.md (the table committed and the part QA lines) and your ledger scratch://converge/proof.md, if it stands. Take the 15 oldest committed asks that the ledger does not mark as passed at an earlier head of the same paths, top 10 first.\nFor each ask, run its QA line on the path of the owner: the TUI on a real life (the headless window and the keys of tui/scenarios, as the QA runs it), the real command line, or a real record. Judge the result against the ask itself, not against the QA line alone. For a screen, capture it from the real renderer and read the PNG as an image.\nWrite to scratch://converge/proof.md, for each ask: its id, the head, passed with the command and its output or the path of the capture, or failed with the evidence. Write in words that a later run reads at once.\nClose with one row for each ask that failed: reopen-<ask id> | <topic> | opus/medium | <paths> | P0, a reopen of <ask id>: <the ask>, <what failed on his path, with the evidence>, <the commit that claimed it>. The topic is one of engine, host, tui, skills, rust and ci. End with one line: proved <n> of <m> at <head>.'
COVER_ONE = 'A new message of the owner, MSG, stands below. Split it into each thing that he asks: a request, a correction, a ruling, a report of a defect. For each part, find the item that serves it whole: grep $TMPDIR/converge/items-full.md, one section per item under ## <state> <topic>:<key>, for its words. An ended item serves a part only when its outcome shows that change.\nClose with lines only: covered <the part>; amend <key> | <the part that its brief misses, with his words verbatim>; or a row <new key> | <topic> | <class> | <paths> | <the part, his words verbatim, and the proof on his path> (topic: engine, host, tui, skills, rust or ci; class: opus/high, opus/medium, sonnet/high or sonnet/medium; keys in lower case with dashes). A message with no ask, as a thanks or a question that the lead answered, closes with: no ask.\n\nThe message:\nTEXT'

ROUTE = "Route the checks of w/converge now, with no merge and no push: the run RUN of the gates at HEAD ended CONCLUSION. For each failed job, read its log (gh api repos/uael/furb/actions/jobs/<id>/logs, with grep and tail), and find each failing test or file and the topic that owns its path, as step 4 of a cycle does. Close with a tuple: a dict that maps each topic to the brief of its failures (the check, the link of the job, the head, the failing tests, and the lines of the log that show the cause), and a summary in 60 words at most. Route nothing that you routed already at this head, and say so in the summary."
DISK_BRIEF = "The disk holds FREE GB free, under the floor of FLOOR GB. Free the target of the oldest ended worktree first, then the next one in age order, until the disk holds the floor again; never the target of a worktree that a live item or fix builds in, a record, or a worktree with uncommitted work. Report each folder freed and the free disk after."
LANES = {
  "triage": "This chain is the triage of the converge. It keeps the reads that ground it: the words of the owner, his picks, the top 10, the plan, the ledger of his asks and the paths of each topic. Its charter, scratch://converge/triage.md, holds its rules.",
  "words": "This chain is the words chain of the converge: it judges each text of the owner part by part against the items of scratch://converge/items-full.md, which it reads again for each text.",
  "proof": "This chain is the proof lane of the converge: it proves on the path of the owner, in batches, the asks that stand committed with no proof, and keeps its ledger in scratch://converge/proof.md.",
}
PAGED = ["desk", "desk_beat", "admit", "pick", "room_step", "ours", "start_fix", "ends_step", "cycle_step", "merges_step", "ci_step", "disk_step", "ground_step", "chain_due", "verify_step", "blanks_and_stuck", "chain_stalls", "triage_step", "deps_step", "priority"]

# The state of the round: a load of the kit keeps the state bound before it.
topic_chain: dict[str, str] = globals().setdefault("topic_chain", {})
desk_queue: dict[str, list[dict]] = globals().setdefault("desk_queue", {})
desk_live: dict[str, dict[str, dict]] = globals().setdefault("desk_live", {})
for _t in TOPICS:
  desk_queue.setdefault(_t, [])
  desk_live.setdefault(_t, {})
desk_ended: list[tuple[str, str, str]] = globals().setdefault("desk_ended", [])
desk_end_at: dict[str, float] = globals().setdefault("desk_end_at", {})
desk_fixing: dict[str, str] = globals().setdefault("desk_fixing", {})
desk_paused: dict[str, list[str]] = globals().setdefault("desk_paused", {})
desk_ground: dict[str, str] = globals().setdefault("desk_ground", {})
desk_text: dict[str, str] = globals().setdefault("desk_text", {"id": "", "cycle": "", "print": ""})
desk_time: dict[str, float] = globals().setdefault("desk_time", {})
desk_notes: list[str] = globals().setdefault("desk_notes", [])
desk_prio: dict[str, int] = globals().setdefault("desk_prio", {})
desk_cleared: list[str] = globals().setdefault("desk_cleared", [])
desk_dropped: dict[str, str] = globals().setdefault("desk_dropped", {})
desk_round: dict[str, int] = globals().setdefault("desk_round", {})
desk_verify: dict[str, tuple] = globals().setdefault("desk_verify", {})
desk_verified: list[tuple] = globals().setdefault("desk_verified", [])
desk_flagged: set[str] = globals().setdefault("desk_flagged", set())
desk_seen_len: dict[str, tuple] = globals().setdefault("desk_seen_len", {})
desk_item_seen: dict[str, tuple] = globals().setdefault("desk_item_seen", {})
triage_inbox: list[dict] = globals().setdefault("triage_inbox", [])
triage_batch: list[dict] = globals().setdefault("triage_batch", [])
qa_handed: set[str] = globals().setdefault("qa_handed", set())
qa_told: set[str] = globals().setdefault("qa_told", set())
qa_count: dict[str, int] = globals().setdefault("qa_count", {"n": 0})
review_round: dict[str, int] = globals().setdefault("review_round", {"n": 0})
gh_seen: set[str] = globals().setdefault("gh_seen", set())
owner_seen: set[str] = globals().setdefault("owner_seen", set())
words_live: dict[str, str] = globals().setdefault("words_live", {})
for _lane in ("triage", "proof", "words"):
  if desk_text.get(f"{_lane}_chain"):
    topic_chain.setdefault(_lane, desk_text.pop(f"{_lane}_chain"))
desk_time.setdefault("merges", desk_time.get("align_mark", 0.0))


def chain_of(name: str) -> str:
  if name not in topic_chain:
    topic_chain[name] = str(chain(name))
  return topic_chain[name]


def kind(model: str) -> str:
  """The class of the room of a model: a rerun at xhigh takes a seat of its high class."""
  return model.split(":")[-1].replace("/xhigh", "/high")


def up_effort(model: str) -> str:
  head, _, effort = model.rpartition("/")
  return f"{head}/{EFFORT_UP.get(effort, effort)}"


def afters_of(x: dict) -> list[str]:
  key = str(x["key"])
  serial = SERIAL2[SERIAL2.index(key) - 1 : SERIAL2.index(key)] if key in SERIAL2 else []
  return sorted({*map(str, x.get("after") or []), *serial})


def priority(x: dict) -> int:
  key = str(x["key"])
  if key in desk_prio:
    return desk_prio[key]
  if tag := re.search(r"\bP([0-6])\.", str(x.get("brief", ""))[:400]):
    return int(tag.group(1))
  return 6 if key in POLISH_KEYS else 3 + max(0, RANK.get(kind(str(x["model"])), 3) - 2)


def hard_rank(x: dict) -> tuple[int, int]:
  return (priority(x), RANK.get(kind(str(x["model"])), 3))


def ended_whole(p: str) -> bool:
  got = peek(p)
  head = str(got)[:160].lower()
  return not isinstance(got, BaseException) and "blocked" not in head and "partly" not in head and "not performed" not in head


def merged_keys() -> set[str]:
  upto = desk_time.get("merged_upto", 0.0)
  return {str(k) for _, k, p in desk_ended if (k in desk_cleared or ended_whole(p)) and desk_end_at.get(str(k), 0.0) <= upto}


def cycle_live() -> bool:
  return bool(desk_text["cycle"]) and not over(desk_text["cycle"])


def qa_live() -> bool:
  return any(not over(p) for p in desk_live[QA])


def seats_used() -> dict[str, int]:
  models = [str(v["model"]) for t in TOPICS for p, v in desk_live[t].items() if not over(p) and not paused(p)]
  models += [FIX_MODEL for f in desk_fixing.values() if not over(f)]
  models += [AGG_MODEL] if cycle_live() else []
  used: dict[str, int] = {}
  for m in models:
    used[kind(m)] = used.get(kind(m), 0) + 1
  return used


def item_text(t: str, item: dict) -> str:
  text = f"Item {item['key']} of the topic {t}. Follow scratch://converge/method.md and the charter of this chain, scratch://converge/{t}.md, and read both again when they changed since you read them.\n\n{item['brief']}"
  return text + qa_handoff() if t == QA else text


def review_cut(acts: list[tuple]) -> list[tuple]:
  live = [a[1] for a in acts if a[0] == "prompt" and not over(a[1])]
  return [a for a in acts if any(under(a[1], p) for p in live)]


def pick() -> tuple[str, dict] | None:
  """The hardest ready item of a topic with room, on a seat of its class, or of the class above when its class is full and no ready item of that class waits."""
  done, used = merged_keys(), seats_used()
  ready = [(t, x) for t in TOPICS if t != QA and not desk_fixing.get(t) and len(desk_live[t]) < desk_width.get(t, 0) for x in desk_queue[t] if all(a in done for a in afters_of(x))]
  kinds = {kind(str(x["model"])) for _, x in ready}
  for t, x in sorted(ready, key=lambda one: hard_rank(one[1])):
    k = kind(str(x["model"]))
    if used.get(k, 0) < ROOM_CAP.get(k, 0):
      chosen = x
    elif (up := UP.get(k, "")) and up not in kinds and used.get(up, 0) < ROOM_CAP.get(up, 0):
      chosen = {**x, "model": str(x["model"]).replace(k, up)}
    else:
      continue
    desk_queue[t].remove(x)
    return t, chosen
  return None


def ours(p: str, why: str) -> bool:
  """Whether the last pause that covers p is the one that the desk made on p for why, so that no pause of the operator ends by a wake of the desk."""
  facts = [x for x in transcript(p, "pause") if covers(x, p)]
  return bool(facts) and facts[-1][:2] == ("pause", p) and str(facts[-1][3:4]).find(why) >= 0


def number(p: str) -> int:
  return int(re.sub(r"\D", "", p) or 0)


def holds() -> dict[str, str]:
  """For each class with a free seat after admission, the cause that keeps each of its queued items from it."""
  used, done, out = seats_used(), merged_keys(), {}
  for k, room in ROOM_CAP.items():
    if used.get(k, 0) >= room:
      continue
    why: list[str] = []
    for t in TOPICS:
      for x in desk_queue[t]:
        if kind(str(x["model"])) == k:
          wait = [a for a in afters_of(x) if a not in done]
          busy = "fixes its CI" if desk_fixing.get(t) else f"runs {len(desk_live[t])} of its width {desk_width.get(t, 0)}"
          why.append(f"{x['key']} waits on {', '.join(wait)}" if wait else f"{x['key']}: {t} {busy}")
    if why:
      out[k] = f"{room - used.get(k, 0)} free: " + "; ".join(why)
  return out


def idle_step() -> list[str]:
  """Each seat that stays free has its cause on the record, told once for each change."""
  if (now := repr(holds())) == desk_text.get("holds", ""):
    return []
  desk_text["holds"] = now
  return ["Free seats and their causes: " + "; ".join(f"{k} {why}" for k, why in holds().items())] if holds() else []


def admit() -> None:
  while got := pick():
    t, item = got
    if t == REVIEW and not desk_live[REVIEW]:
      compact([review_cut], summary="This chain is the review of the converge. It is cut before each item to its charter, which its setup shows, and its open items: each item reads again the files that its brief names. Its charter, scratch://converge/review.md, holds its rules.", on=chain_of(REVIEW))
    desk_live[t][str(prompt(str, item_text(t, item), to=item["model"], on=chain_of(t)))] = item


def room_step() -> list[str]:
  """Pause the newest items past the room of their class, and wake the oldest of them once a seat of their class is free."""
  notes, used, held = [], seats_used(), desk_paused.setdefault("room", [])
  newest = sorted(((p, t, x) for t in TOPICS for p, x in desk_live[t].items() if not over(p) and not paused(p)), key=lambda one: -number(one[0]))
  for p, t, x in newest:
    if used.get(k := kind(str(x["model"])), 0) > ROOM_CAP.get(k, 0):
      pause(p, ROOM_WHY)
      held.append(p)
      used[k] -= 1
      notes.append(f"{t}: {p} ({x['key']}) paused, past the room of {k}; it resumes when a seat is free.")
  for p in list(held):
    t, x = next(((t, desk_live[t][p]) for t in TOPICS if p in desk_live[t]), ("", None))
    if x is None or over(p) or not ours(p, ROOM_WHY):
      held.remove(p)
    elif not desk_fixing.get(t) and used.get(k := kind(str(x["model"])), 0) < ROOM_CAP.get(k, 0):
      wake(p)
      held.remove(p)
      used[k] = used.get(k, 0) + 1
  return notes


def start_fix(t: str, why: str) -> str:
  """A failure of CI goes before the items of its topic: to its live fix, or to a new fix that pauses the live items of the topic until it ends."""
  if (live := desk_fixing.get(t, "")) and not over(live):
    prompt(None, f"A new failure of the CI of w/converge for this fix:\n\n{why}", on=live)
    return live
  for p in desk_live[t]:
    if not over(p) and not paused(p):
      pause(p, FIX_WHY)
      desk_paused.setdefault(t, []).append(p)
  desk_fixing[t] = str(prompt(str, FIX_HEAD + why, to=FIX_MODEL, on=chain_of(t)))
  return desk_fixing[t]


def route(got: object) -> list[str]:
  if not (isinstance(got, tuple) and len(got) == 2 and isinstance(got[0], dict)):
    return []
  sent = [str(t) for t, why in got[0].items() if t in CODE and why]
  for t in sent:
    start_fix(t, str(got[0][t]))
  return sent


def skip_note(p: str) -> str:
  cmds = [str(x[4])[:200] for x in transcript(p, "bash") if len(x) > 4 and under(str(x[1]), p) and re.search(r"--no-verify|\bSKIP=", str(x[4]))]
  return f"\nIt skipped hooks, against its charter, in: {cmds}" if cmds else ""


def route_needs(t: str, key: str, p: str) -> str:
  said, rows = str(peek(p)), []
  for i, m in enumerate(re.finditer(r"(?m)(?:^|(?<=\. ))\s*Needs:\s*([a-z]+)\b[:,]?\s*(.+?)(?=\s+Needs:|$)", said)):
    rows.append(f"needs-{key}-{i + 1} | {m.group(1)} | opus/medium | | {m.group(2).replace('|', '/')}")
  listed = "\n".join(m.group(1) for m in re.finditer(r"(?m)Needs:\s*\n((?:- .+\n?)+)", said))
  for j, b in enumerate(re.finditer(r"(?m)^- ([a-z]+)\b[:,]?\s*(.+)$", listed)):
    rows.append(f"needs-{key}-b{j + 1} | {b.group(1)} | opus/medium | | {b.group(2).replace('|', '/')}")
  put = queue_found("\n".join(rows), f"the Needs of {key} ({t}, {p})")
  return f"\nIts Needs went to triage as {', '.join(put)}." if put else ""


def found_rows(t: str, p: str) -> str:
  put = queue_found(str(peek(p)), f"the {t} run {p}") if t in (QA, REVIEW) else []
  return f"\nIts rows went to triage as {', '.join(put)}." if put else ""


def ends_step() -> None:
  for t in TOPICS:
    if (fix := desk_fixing.get(t, "")) and over(fix):
      desk_fixing.pop(t)
      for p in desk_paused.pop(t, []):
        over(p) or not ours(p, FIX_WHY) or wake(p)
      desk_notes.append(f"{t} ended the fix {fix} of CI: {str(peek(fix))[:2000]}" + skip_note(fix) + route_needs(t, fix, fix))
    for p in [p for p in desk_live[t] if over(p)]:
      item = desk_live[t].pop(p)
      key = str(item["key"])
      desk_ended.append((t, key, p))
      desk_end_at[key] = clock()
      if key.startswith("words-audit") and (wc := re.search(r"\d+ (?:asks|request rows)[^.\n]*", str(peek(p)))):
        desk_text["words_counts"] = wc.group(0)
      desk_notes.append(f"{t}: the item {key} ({p}, {item['model']}) ended: {str(peek(p))[:3000]}" + skip_note(p) + route_needs(t, key, p) + found_rows(t, p))
      if t in CODE and not re.match(NO_VERIFY, key):
        verify_start(t, key, p, str(item["model"]), str(item["brief"]))


def merges_step(seen: str) -> None:
  """Count each new head of w/converge as one integration, and queue the audit of alignment after each MERGES of them, by turns on ASTRA and opus/high."""
  head = next((x[5:] for x in seen.splitlines() if x.startswith("head ")), "")
  if head and head != desk_text.get("head", ""):
    if desk_text.get("head"):
      desk_time["merges"] = desk_time.get("merges", 0.0) + 1
    desk_text["head"] = head
  n = int(desk_time.get("merges", 0.0))
  if n - desk_time.get("align_mark", 0.0) >= MERGES and not any(str(x["key"]).startswith("align-audit") for x in [*desk_queue[REVIEW], *desk_live[REVIEW].values()]):
    desk_time["align_mark"] = float(n)
    desk_time["audits"] = desk_time.get("audits", 0.0) + 1
    model = ASTRA if desk_time["audits"] % 2 else CLASSES["opus/high"]
    desk_queue[REVIEW].insert(0, {"key": f"align-audit-m{n}", "model": model, "brief": "P0. " + ALIGN_BRIEF})


def cycle_step(every: float, floor: float) -> Generator[set[str], object, None]:
  """One cycle of the aggregator at a time, a floor after the start of the last, on a new print of the branches and checks; one QA run after each."""
  if (cycle := desk_text["cycle"]) and over(cycle):
    got = peek(cycle)
    desk_text["cycle"] = ""
    desk_time["cycle_end"] = clock()
    desk_time["merged_upto"] = desk_time.get("cycle_at", 0.0)
    if not desk_queue[QA]:
      qa_count["n"] += 1
      desk_queue[QA].append({"key": f"qa-run-{qa_count['n']}", "model": OPUS_M, "brief": QA_BRIEF})
    sent = route(got)
    told = f"{got[1]}" if isinstance(got, tuple) and len(got) == 2 and isinstance(got[0], dict) else f"ended with {str(got)[:1500]}"
    desk_notes.append(f"cycle {cycle}: {told}" + (f" Sent to {', '.join(sent)}." if sent else ""))
  if not cycle_live() and desk_queue[QA] and not qa_live():
    item = desk_queue[QA].pop(0)
    desk_live[QA][str(prompt(str, item_text(QA, item), to=item["model"], on=chain_of(QA)))] = item
  if cycle_live() or clock() - desk_time.get("last", 0.0) < every or clock() - desk_time.get("cycle_at", 0.0) < floor:
    return
  ran = bash(FINGER, show=HIDDEN, ok=True, timeout=120)
  while not over(ran):
    yield {ran}
  out = peek(ran)
  seen = out.stdout.content if isinstance(out, Exit) else ""
  desk_time["last"] = clock()
  merges_step(seen)
  if seen != desk_text["print"]:
    for qp in [p for p in desk_live[QA] if not over(p) and p not in qa_told]:
      prompt(None, QA_WRAP, on=qp)
      qa_told.add(qp)
    desk_text["print"] = seen
    desk_time["cycle_at"] = clock()
    desk_text["cycle"] = str(prompt(tuple[dict[str, str], str], CYCLE, to=AGG_MODEL, on=chain_of(AGG)))


def ci_step() -> Generator[set[str], object, None]:
  """Route each failed run of the gates of w/converge to the topics that own its failures, once for each run."""
  if (rp := desk_text.get("route_live", "")) and over(rp):
    desk_text["route_live"] = ""
    got = peek(rp)
    sent = route(got)
    desk_notes.append(f"{rp} routed the failures of CI of w/converge: {got[1] if isinstance(got, tuple) and len(got) == 2 else str(got)[:800]}" + (f" Sent to {', '.join(sent)}." if sent else ""))
  if desk_text.get("route_live") or clock() - desk_time.get("ci_at", 0.0) < CI_EVERY:
    return
  desk_time["ci_at"] = clock()
  cr = bash(CI_PROBE, show=HIDDEN, ok=True, timeout=60)
  while not over(cr):
    yield {cr}
  out = peek(cr)
  cols = out.stdout.content.split() if isinstance(out, Exit) else []
  if len(cols) == 4 and cols[1] == "completed" and cols[3] != desk_text.get("routed_run", ""):
    desk_text["routed_run"] = cols[3]
    if cols[2] == "success":
      desk_notes.append(f"The checks of CI of w/converge pass at {cols[0][:9]} (run {cols[3]}).")
    else:
      ask = ROUTE.replace("RUN", cols[3]).replace("HEAD", cols[0]).replace("CONCLUSION", cols[2])
      desk_text["route_live"] = str(prompt(tuple[dict[str, str], str], ask, to=AGG_MODEL, on=chain_of(AGG)))


def disk_step() -> Generator[set[str], object, None]:
  """Keep DISK_FLOOR GB free: under it, tidy frees the targets of the ended worktrees, oldest first."""
  if clock() - desk_time.get("disk_at", 0.0) < DISK_EVERY:
    return
  desk_time["disk_at"] = clock()
  dk = bash(DF, show=HIDDEN, ok=True, timeout=30)
  while not over(dk):
    yield {dk}
  out = peek(dk)
  cols = out.stdout.content.split() if isinstance(out, Exit) else []
  free = int(cols[3]) // 1048576 if len(cols) > 3 and cols[3].isdigit() else -1
  if 0 <= free < DISK_FLOOR and not any(str(x["key"]) == "free-disk" for x in [*desk_queue["tidy"], *desk_live["tidy"].values()]):
    keep = sorted({t for t in TOPICS if any(not over(p) for p in desk_live[t]) or (desk_fixing.get(t) and not over(desk_fixing[t]))})
    brief = DISK_BRIEF.replace("FREE", str(free)).replace("FLOOR", str(DISK_FLOOR)) + f" The topics that build now, whose worktrees you keep: {', '.join(keep) or 'none'}."
    desk_queue["tidy"].insert(0, {"key": "free-disk", "model": SONNET_M, "brief": "P0. " + brief})
    desk_notes.append(f"The disk holds {free} GB free, under the floor of {DISK_FLOOR} GB, so tidy frees the targets of the ended worktrees.")


def read_tokens(r: str) -> int:
  v = peek(r)
  return v[2][0] if isinstance(v, tuple) and len(v) > 2 and isinstance(v[2], tuple) and v[2] and isinstance(v[2][0], int) else 0


def chain_due(c: str) -> bool:
  """Past REGROUND_TOKENS or REGROUND of its window, and grown past a guard over the size after its last compaction, so a large reground never compacts again at once."""
  replies = [str(x[1]) for x in transcript(c, "reply") if over(x[1])]
  if not replies or (since := desk_text.get("compacted_" + c, "")) == replies[-1]:
    return False
  last = replies[-1]
  base = read_tokens(replies[replies.index(since) + 1]) if since in replies[:-1] else 0
  tokens, made = read_tokens(last), get(last)
  roster = standing()[0]
  window = (offered(roster, str(made[4])) if made and len(made) > 4 and isinstance(roster, list) else None) or WINDOW  # ty: ignore[invalid-argument-type]
  return tokens > base + min(REGROUND_GROWTH, window // 4) and (tokens > REGROUND_TOKENS or (filled(last) or 0.0) > REGROUND)


def grounding(acts: list[tuple]) -> list[tuple]:
  asked = [a[1] for a in acts if a[0] == "prompt"]
  live = [a[1] for a in acts if a[0] == "prompt" and not over(a[1])]
  return [a for a in acts if (a[0] == "read" and not any(under(a[1], p) for p in asked)) or any(under(a[1], p) for p in live)]


def kept_reads(ids: set[str]) -> Callable[[list[tuple]], list[tuple]]:
  def keep(acts: list[tuple]) -> list[tuple]:
    live = [a[1] for a in acts if a[0] == "prompt" and not over(a[1])]
    return [a for a in grounding(acts) if (a[0] == "read" and a[1] in ids) or any(under(a[1], p) for p in live)]

  return keep


def ground_note(name: str) -> str:
  return LANES.get(name) or f"This chain is the topic {name} of the converge. It was regrounded past {REGROUND:.0%} of its window: it keeps the reads that grounded it, read again to show what changed, and its open items. The work of each ended item stands in the commits of its branch. Its charter, scratch://converge/{name}.md, holds its rules, its paths and the asks of the owner that it carries."


def ground_step(name: str) -> str:
  """The one home of the compaction of each chain of the round: a reground on its reads, then, when it still reads past REGROUND, a cheap choice of the reads to keep."""
  c = topic_chain[name]
  replies = [str(x[1]) for x in transcript(c, "reply") if over(x[1])]
  last = replies[-1] if replies else ""
  share = (filled(last) or 0.0) if last else 0.0
  mark = desk_ground.get(name, "")
  if mark.startswith("c:"):
    if not over(chose := mark[2:]):
      return ""
    got = peek(chose)
    desk_ground[name] = ""
    if isinstance(got, tuple) and len(got) == 2 and isinstance(got[0], list):
      ids = {str(i) for i in got[0]}
      desk_text["compacted_" + c] = last
      compact([kept_reads(ids), latest()], summary=ground_note(name) + "\n" + str(got[1]).strip(), on=c)
      return f"{name} ({c}) still read {share:.0%} of its window after its reground, so a cheap model chose the acts to keep ({len(ids)})."
    return f"{name} ({c}): the cheap choice of its reads gave {str(got)[:300]}, so the chain stays as it is."
  if mark.startswith("g:") and mark != "g:" + last:
    desk_ground[name] = ""
    if share > REGROUND:
      desk_ground[name] = "c:" + str(choice(on=c))
      return ""
  if not chain_due(c):
    return ""
  desk_text["compacted_" + c] = last
  desk_ground[name] = "g:" + last
  compact([grounding, latest()], summary=ground_note(name), on=c)
  return f"{name} ({c}) read {read_tokens(last) // 1000}k tokens, {share:.0%} of its window, so it was regrounded on its reads."


def verify_start(t: str, key: str, p: str, model: str, brief: str) -> str:
  text = VERIFY_BRIEF.replace("KEY", key).replace("TOPIC", t).replace("BRIEF", brief[:5000]).replace("REPORT", str(peek(p))[:6000])
  vp = str(prompt(str, text, to=VERIFIER, on=chain(f"verify {key}")))
  desk_verify[vp] = (t, key, p, model, brief)
  return vp


def verify_step() -> list[str]:
  """An item is done when its judge says yes; else it opens again at P0, one effort higher, at most ROUNDS times."""
  notes = []
  for vp in [v for v in desk_verify if over(v)]:
    t, key, p, model, brief = desk_verify.pop(vp)
    got = peek(vp)
    verdict, base = str(got).strip(), re.sub(r"-r\d+$", "", key)
    if isinstance(got, BaseException):
      notes.append(f"The verification {vp} of {t}:{key} ended with {verdict[:300]}, so {key} stays unverified.")
    elif verdict.lower().rstrip(".") == "yes":
      desk_verified.append((t, key, p, vp))
    elif (n := desk_round.get(base, 0) + 1) > ROUNDS:
      notes.append(f"{t}:{key} failed its verification {n - 1} times. The last cause, from {vp}: {verdict[:800]}")
    else:
      desk_round[base] = n
      desk_queue[t].insert(0, {"key": f"{base}-r{n}", "model": up_effort(model), "brief": f"P0. A reopen of {key} ({p}) by its judge ({vp}). Fix what it names, and push: {verdict[:1500]}\n\nThe brief of {key}:\n{brief}"})
  return notes


def proof_step() -> list[str]:
  last = desk_text.get("proof_live", "")
  if last and not over(last):
    return []
  got = str(peek(last)) if last else ""
  m = re.search(r"proved (\d+) of (\d+) at (\w+)", got)
  if last:
    desk_text["proof_last"] = m.group(0) if m else got[-300:]
  if m and m.group(2) == "0" and desk_text.get("proof_idle_at", "") == desk_text.get("asks_state_at", ""):
    return []
  desk_text["proof_idle_at"] = desk_text.get("asks_state_at", "")
  n = int(desk_text.get("proof_n", "0")) + 1
  desk_text["proof_n"] = str(n)
  p = str(prompt(str, PROOF_BRIEF, to=VERIFIER, on=chain_of("proof")))
  desk_live[REVIEW][p] = {"key": f"proof-{n}", "model": VERIFIER, "brief": PROOF_BRIEF}
  desk_text["proof_live"] = p
  return [f"The proof lane ended {last}: {desk_text.get('proof_last', '')}. It started proof-{n} ({p})."] if last else []


def printed_by_life(url: str) -> bool:
  n = 1
  while get(f"chain{n}"):
    if any(url in str(x[3:]) for x in transcript(f"chain{n}", "out")):
      return True
    n += 1
  return False


def gh_step() -> list[str]:
  """Each word of the owner on GitHub reaches the root as a message of his."""
  live = desk_text.get("gh_live", "")
  if live and not over(live):
    return []
  if live:
    desk_text["gh_live"] = ""
    got = peek(live)
    for line in (got.stdout.content if isinstance(got, Exit) else "").splitlines():
      parts = line.split("\t")
      if parts[0] == "now" and len(parts) > 1:
        desk_text["gh_since"] = parts[1]
      elif parts[0] in ("issue", "comment") and len(parts) >= 4 and (key := parts[0] + parts[1]) not in gh_seen and not parts[3].lstrip().startswith("🤖"):
        gh_seen.add(key)
        if not printed_by_life(parts[2]):
          prompt(None, f"The owner wrote on GitHub, {parts[0]} {parts[2]}: {parts[3]}\nRead it whole with gh, answer it there with a body that starts with 🤖, and act on it as a message of his.", on=ROOT)
  if clock() - desk_time.get("gh_at", 0.0) >= CI_EVERY:
    desk_time["gh_at"] = clock()
    desk_text["gh_live"] = str(bash(f"{GH_POLL} {desk_text.get('gh_since', '')}", show=HIDDEN, ok=True, timeout=60))
  return []


def shots_step() -> list[str]:
  live = desk_text.get("shots_live", "")
  if live and not over(live):
    return []
  notes = []
  if live:
    desk_text["shots_live"] = ""
    got = peek(live)
    out = got.stdout.content.strip() if isinstance(got, Exit) else str(got)
    if not isinstance(got, Exit) or got.code != 0:
      notes.append(f"The push of the captures to w/shots failed: {out[-300:]} {got.stderr.content[-300:] if isinstance(got, Exit) else ''}")
    elif "published" in out:
      desk_text["shots_last"] = out.splitlines()[-1]
  if clock() - desk_time.get("shots_at", 0.0) >= EVERY:
    desk_time["shots_at"] = clock()
    desk_text["shots_live"] = str(bash(SHOTS, show=HIDDEN, ok=True, timeout=300))
  return notes


def mem_step() -> list[str]:
  live = desk_text.get("mem_live", "")
  notes = []
  if live and over(live):
    desk_text["mem_live"] = ""
    got = peek(live)
    out = got.stdout.content if isinstance(got, Exit) else ""
    m_free, m_swap = re.search(r"free percentage: (\d+)%", out), re.search(r"free = ([\d.]+)M", out)
    free = int(m_free.group(1)) if m_free else 100
    swap = float(m_swap.group(1)) if m_swap else 1e9
    desk_text["mem_last"] = f"memory {free}% free, swap {swap:.0f} MB free"
    if free < 15 or (swap < 512 and free < 30):
      notes.append(f"The memory runs low: {desk_text['mem_last']}. Find the processes that hold it (ps by rss) and tell the owner.")
  elif not live and clock() - desk_time.get("mem_at", 0.0) >= CI_EVERY:
    desk_time["mem_at"] = clock()
    desk_text["mem_live"] = str(bash(MEM, show=HIDDEN, ok=True, timeout=30))
  return notes


def queue_found(text: str, source: str) -> list[str]:
  """The one way in of a new row: each line key | topic | class | paths | finding goes to the triage once."""
  taken = {str(x["key"]) for t in TOPICS for x in [*desk_queue[t], *desk_live[t].values()]} | {str(k) for _, k, _ in desk_ended} | {str(x["key"]) for x in [*triage_inbox, *triage_batch]}
  put = []
  for line in text.splitlines():
    parts = [p.strip() for p in line.split("|")]
    if len(parts) < 5:
      continue
    key, t, k = parts[0].removeprefix("- ").strip().rsplit(": ", 1)[-1].strip(), parts[1], kind(parts[2])
    if t in TOPICS and t not in (REVIEW, QA) and k in CLASSES and key not in taken and re.fullmatch(r"[A-Za-z0-9][\w-]*", key):
      brief = f"A finding of {source}: {' | '.join(parts[4:])} Paths: {parts[3]}. Judge it against w/converge first, then fix it at its cause as one commit, with a test or a scenario that fails before the fix." + CLAIM
      triage_inbox.append({"key": key, "model": CLASSES[k], "brief": brief, "topic": t})
      taken.add(key)
      put.append(f"{t}:{key}")
  return put


def drop_row(key: str, why: str) -> None:
  desk_dropped[key] = why
  triage_inbox[:] = [x for x in triage_inbox if str(x["key"]) != key]
  triage_batch[:] = [x for x in triage_batch if str(x["key"]) != key]
  desk_text["triage_given"] = " ".join(k for k in desk_text.get("triage_given", "").split() if k != key)
  for t in TOPICS:
    desk_queue[t] = [x for x in desk_queue[t] if str(x["key"]) != key]


def write_triage_queue() -> str:
  rows = [f"## {q['key']} | {t} | P{priority(q)} | {kind(str(q['model']))} | after {afters_of(q)}\n{q['brief']}" for t in TOPICS if t not in (REVIEW, QA) for q in desk_queue[t]]
  live = [f"{i['key']} ({t})" for t in TOPICS for i in desk_live[t].values()]
  write(Text("scratch://converge/triage/queue.md", f"# The queued items of the topics, whole, for the triage\n\nEach item: key | topic | priority | class | the keys that it waits for, then its whole brief.\n\nLive now, not yours to change, but an item may wait for one: {', '.join(live)}.\nThe changes of src/furb/engine.py run one at a time in this order (SERIAL2): {SERIAL2}.\n\n" + "\n\n".join(rows) + "\n"))
  return f"{len(rows)} queued"


def triage_apply(got: object, given: set[str]) -> str:
  """The triage owns the priority, the class, the brief and the after list of each new row; the desk checks its answer whole before it applies it."""
  if not (isinstance(got, tuple) and len(got) == 2 and isinstance(got[0], list) and isinstance(got[1], dict)):
    return f"the triage was not applied: no tuple of items and drops, {str(got)[:300]}"
  items = [x for x in got[0] if isinstance(x, dict) and str(x.get("key")) not in desk_dropped]
  dropped = {str(k): str(v) for k, v in got[1].items()}
  queued = {str(q["key"]) for t in TOPICS for q in desk_queue[t]}
  live = {str(i["key"]) for t in TOPICS for i in desk_live[t].values()}

  def absorbs(x: dict) -> list[str]:
    return [str(a) for a in x.get("absorbs") or []] if isinstance(x.get("absorbs") or [], list) else []

  covered = [str(x.get("key")) for x in items] + [a for x in items for a in absorbs(x)] + list(dropped)
  missing, twice = sorted(given - set(covered)), sorted({k for k in covered if covered.count(k) > 1})
  extra = sorted(set(covered) - given - queued - set(desk_dropped))
  bad = [str(x.get("key")) for x in items if str(x.get("topic")) not in TOPICS or str(x.get("topic")) in (REVIEW, QA) or str(x.get("class")) not in CLASSES or not isinstance(x.get("priority"), int) or not str(x.get("brief", "")).strip() or str(x.get("key")) in live]
  if missing or twice or extra or bad:
    return f"the triage was not applied: missing {missing}, twice {twice}, not queued {extra}, bad {bad}"
  keys = {str(x["key"]) for x in items}
  gone = set(covered) - keys
  for x in items:
    merged = {str(x["key"]), *absorbs(x)}
    after = {*map(str, x.get("after") or []), *[a for r in triage_batch if str(r["key"]) in merged for a in afters_of(r)]}
    desk_prio[str(x["key"])] = int(x["priority"])
    for t in TOPICS:
      desk_queue[t] = [q for q in desk_queue[t] if str(q["key"]) not in merged]
    desk_queue[str(x["topic"])].append({"key": str(x["key"]), "model": CLASSES[str(x["class"])], "brief": str(x["brief"]), "after": sorted(after)})
  for t in TOPICS:
    desk_queue[t] = [q for q in desk_queue[t] if str(q["key"]) not in gone]
  desk_dropped.update(dropped)
  return f"applied: {len(items)} items, {len(gone) - len(dropped)} merged into others, {len(dropped)} dropped"


def triage_step() -> str:
  note = ""
  if (tri := desk_text.get("triage_live", "")) and over(tri):
    desk_text["triage_live"] = ""
    res = triage_apply(peek(tri), set(desk_text.get("triage_given", "").split()))
    if res.startswith("the triage was not applied"):
      for r in triage_batch:
        desk_queue[str(r["topic"])].append({k: v for k, v in r.items() if k != "topic"})
      res += "; the rows went to their queues as they came"
    note = "" if re.fullmatch(r"applied: \d+ items, 0 merged into others, 0 dropped", res) else f"triage {tri}: {res}"
    triage_batch.clear()
  if not desk_text.get("triage_live") and not desk_text.get("deps_live") and triage_inbox:
    triage_batch.extend(triage_inbox)
    triage_inbox.clear()
    write_triage_queue()
    rows = "\n\n".join(f"## {r['key']} | {r['topic']} | {kind(str(r['model']))}\n{r['brief']}" for r in triage_batch)
    desk_text["triage_given"] = " ".join(str(r["key"]) for r in triage_batch)
    desk_text["triage_live"] = str(prompt(tuple[list[dict[str, object]], dict[str, str]], TRIAGE_NEW + rows, to=OPUS_M, on=chain_of("triage")))
  return note


def ask_deps(why: str) -> None:
  desk_text["deps_due"] = (desk_text.get("deps_due", "") + "\n" + why).strip()


def deps_rows() -> str:
  rows = [f"{x['key']} | {t} | live" for t in TOPICS for x in desk_live[t].values()]
  rows += [f"{x['key']} | {t} | P{priority(x)} | after: {', '.join(afters_of(x)) or 'none'} | {str(x['brief'])[:400]}" for t in TOPICS if t not in (REVIEW, QA) for x in desk_queue[t]]
  return "\n".join(rows)


def deps_apply(got: object) -> str:
  """Apply the after lists of the triage only when each key is known and no list makes a cycle."""
  if not isinstance(got, dict):
    return f"the deps of the triage were not applied: no dict, {str(got)[:300]}"
  whole = {str(k) for _, k, p in desk_ended if k in desk_cleared or ended_whole(p)}
  queued = {str(x["key"]) for t in TOPICS for x in desk_queue[t]}
  known = queued | {str(i["key"]) for t in TOPICS for i in desk_live[t].values()} | whole
  wants = {str(k): sorted({str(a) for a in v}) for k, v in got.items() if str(k) in queued and isinstance(v, list)}
  if bad := sorted(f"{k}:{a}" for k, v in wants.items() for a in v if a not in known or a == k):
    return f"the deps of the triage were not applied: unknown or self keys {bad[:10]}"
  graph = {str(x["key"]): afters_of(x) for t in TOPICS for x in desk_queue[t]} | wants
  left = set(graph)
  while free := {k for k in left if not any(a in left for a in graph[k])}:
    left -= free
  if left:
    return f"the deps of the triage were not applied: a cycle holds {sorted(left)[:10]}"
  changed = 0
  for t in TOPICS:
    for n, x in enumerate(desk_queue[t]):
      if (k := str(x["key"])) in wants and wants[k] != afters_of(x):
        desk_queue[t][n] = {**x, "after": wants[k]}
        changed += 1
  return f"the triage set the deps: {changed} items changed"


def deps_step() -> str:
  if (live := desk_text.get("deps_live", "")) and over(live):
    desk_text["deps_live"] = ""
    return deps_apply(peek(live))
  if not live and not desk_text.get("triage_live") and (why := desk_text.get("deps_due", "")):
    desk_text["deps_due"] = ""
    desk_text["deps_live"] = str(prompt(dict[str, list[str]], DEPS_ASK + why + "\n\n" + deps_rows(), to=OPUS_M, on=chain_of("triage")))
  return ""


def blanks_and_stuck() -> list[str]:
  """A wait on a key that nothing will end goes to the triage; a topic that runs nothing says why; an item with no step for STUCK is stepped again."""
  notes, now = [], clock()
  whole = {str(k) for _, k, p in desk_ended if k in desk_cleared or ended_whole(p)}
  merged = merged_keys()
  pending = {str(x["key"]) for t in TOPICS for x in [*desk_queue[t], *desk_live[t].values()]} | {str(x["key"]) for x in [*triage_inbox, *triage_batch]}
  dead = []
  for t in TOPICS:
    for x in desk_queue[t]:
      for a in afters_of(x):
        if a not in whole and a not in pending and (mark := f"dead:{x['key']}:{a}") not in desk_flagged:
          desk_flagged.add(mark)
          dead.append(f"{t}:{x['key']} waits on {a}")
  if dead:
    ask_deps("These items wait on a key that no queue holds and that never ended with a commit, so they never start: " + "; ".join(dead) + ".")
    notes.append("Dead waits sent to the triage: " + "; ".join(dead[:15]) + ".")
  for t in TOPICS:
    if t in (QA, REVIEW) or desk_width.get(t, 0) == 0:
      continue
    if desk_live[t] or desk_fixing.get(t):
      desk_flagged.discard(f"blank:{t}")
      continue
    if f"blank:{t}" in desk_flagged:
      continue
    desk_flagged.add(f"blank:{t}")
    heads = [f"{x['key']} (waits on {', '.join(a for a in afters_of(x) if a not in merged) or 'a seat of ' + kind(str(x['model']))})" for x in desk_queue[t][:4]]
    notes.append(f"{t} runs nothing, with {len(desk_queue[t])} queued: {'; '.join(heads) or 'an empty queue'}.")
    if any(a not in merged for x in desk_queue[t][:4] for a in afters_of(x)):
      ask_deps(f"{t} runs nothing while the heads of its queue wait on items: {'; '.join(heads)}.")
  for t in TOPICS:
    for p in [*desk_live[t], *([desk_fixing[t]] if desk_fixing.get(t) else [])]:
      if over(p) or paused(p):
        continue
      by_p = transcript(p, p)
      seen = desk_item_seen.get(p)
      if seen is None or seen[0] != len(by_p):
        desk_item_seen[p] = (len(by_p), now)
        desk_flagged.discard(f"stuck:{p}")
        continue
      idle = now - seen[1]
      runs = any(question(x) and not over(x[1]) for x in by_p[-3:])
      if idle >= (STUCK_RUNNING if runs else STUCK) and f"stuck:{p}" not in desk_flagged:
        desk_flagged.add(f"stuck:{p}")
        if not runs:
          prompt(None, "The lead: no step of this item ran for 20 minutes, so this message steps it again.", on=p)
        notes.append(f"{t}: {p} made no step for {idle / 60:.0f} minutes, " + ("and its last act still runs past the timeout of a commit." if runs else "with no act of it open; the lead nudged it."))
  return notes


def chain_stalls() -> list[str]:
  """A topic chain with turns that wait and no run or reply open for STALL is stepped again through its live item."""
  notes = []
  for t in [t for t in TOPICS if t in topic_chain]:
    c = topic_chain[t]
    n = len(transcript(c))
    seen = desk_seen_len.get(c, (-1, clock()))
    if seen[0] != n:
      desk_seen_len[c] = (n, clock())
      continue
    waiting = [x for x in transcript(c, "rung") if len(x) > 4 and not x[4] and not over(x[1])]
    busy = [x for x in transcript(c) if x[0] in ("run", "reply") and question(x) and not over(x[1])]
    live = [p for p in [*([desk_fixing[t]] if desk_fixing.get(t) else []), *desk_live[t]] if not over(p)]
    if waiting and not busy and live and clock() - seen[1] >= STALL:
      prompt(None, "The lead: this chain made no fact for 10 minutes while turns wait on it, with no run and no reply open, so this message steps it again.", on=live[0])
      desk_seen_len[c] = (n + 1, clock())
      notes.append(f"{t} ({c}) stalled 10 min with {len(waiting)} waiting turns and no run or reply open; nudged {live[0]}.")
  return notes


def write_items_full() -> None:
  verified = {k for _, k, _, _ in desk_verified}
  ix = [f"## queued {t}:{x['key']} ({x['model']})\n{x['brief']}\n" for t in TOPICS for x in desk_queue[t]]
  ix += [f"## live {t}:{x['key']} ({x['model']}, {p})\n{x['brief']}\n" for t in TOPICS for p, x in desk_live[t].items() if not over(p)]
  ix += [f"## queued {x.get('topic', '')}:{x['key']} ({x['model']})\n{x['brief']}\n" for x in [*triage_inbox, *triage_batch]]
  ix += [f"## ended{' verified' if k in verified else ''} {t}:{k} ({p})\nOutcome:\n{peek(p)!s}\n" for t, k, p in desk_ended]
  write(Text("scratch://converge/items-full.md", "\n".join(ix)))


def amend_item(key: str, text: str, src: str) -> bool:
  add = f" Added by {src}, a part of the ask of the owner that this item missed: {text}"
  for x in [*triage_inbox, *triage_batch, *[y for t in TOPICS for y in desk_queue[t]]]:
    if str(x["key"]) == key:
      x["brief"] = str(x["brief"]) + add
      return True
  for t in TOPICS:
    for p, x in desk_live[t].items():
      if str(x["key"]) == key and not over(p):
        prompt(None, f"An amendment of your brief, from {src}: {text}", on=p)
        return True
  return any(str(k) == key and bool(queue_found(f"{key}-more | {t} | opus/medium | | {text.replace('|', '/')}", src)) for t, k, _ in desk_ended)


def apply_cover(out: str, src: str) -> tuple[int, list[str], list[str], list[str]]:
  covered, amended, missed, rows = 0, [], [], []
  for line in out.splitlines():
    s = line.strip().strip("`").removeprefix("- ").strip()
    if s.startswith("covered"):
      covered += 1
    elif s.startswith("amend "):
      k, _, txt = s[6:].partition("|")
      (amended if txt.strip() and amend_item(k.strip(), txt.strip(), src) else missed).append(k.strip() if txt.strip() else s[:200])
    elif s.count("|") >= 4:
      rows.append(s)
  return covered, amended, queue_found("\n".join(rows), src), missed


def cover(text: str, label: str) -> str:
  """The words chain judges a text of the owner (a new message, or a part of the ledger of his asks) part by part against the items."""
  text = re.sub(r"furb-image://\S+", "[an image of the owner]", text)
  p = str(prompt(str, COVER_ONE.replace("MSG", label).replace("TEXT", text), to=VERIFIER, on=chain_of("words")))
  words_live[p] = label
  return p


def words_step() -> list[str]:
  notes = []
  for wp in [w for w in words_live if over(w)]:
    label = words_live.pop(wp)
    c, a, q, m = apply_cover(str(peek(wp)), f"the words chain, on {label}")
    if a or q or m:
      notes.append(f"The words chain judged {label} ({wp}): {c} parts covered, amended {a}, queued {q}" + (f", not applied: {m}" if m else ""))
  new = [x for x in transcript(ROOT, "prompt") if operated(str(x[2])) and str(x[1]) not in owner_seen]
  if new:
    write_items_full()
  for x in new:
    owner_seen.add(str(x[1]))
    cover(str(x[5]), str(x[1]))
  return notes


def qa_handoff() -> str:
  rows = []
  for t, k, p in desk_ended:
    if k in qa_handed or t == QA or not (made := get(p)) or "A finding of the QA run" not in str(made[5]):
      continue
    qa_handed.add(k)
    rows.append(f"- {t}:{k} ({'ended whole' if ended_whole(p) else 'ended blocked or partly'}): {' '.join(str(peek(p)).split())[:240]}")
  head = "\n\nThe rows that the QA found and that a topic ended since the last run. Check each one on this build, and name in your entry each one that still fails:\n"
  return head + "\n".join(rows[:30]) if rows else ""


def answers_file() -> str:
  rows = []
  for x in transcript(ROOT, "prompt"):
    if (made := get(str(x[2]))) and (under(str(x[2]), chain_of(REVIEW)) or str(made[3]) == chain_of(REVIEW)):
      answer = peek(x[1]) if over(x[1]) and str(x[4]) != "None" else "(a note, answered by a change of the desk)"
      rows.append(f"## {x[1]}, asked by the review chain\n{str(x[5])[:1500]}\n\nThe answer of the lead:\n{str(answer)[:3000]}")
  write(Text("scratch://converge/answers.md", "# The questions of the audits to the lead, and his answers\n\n" + "\n\n".join(rows) + "\n"))
  return f"{len(rows)} answers"


def words_files() -> str:
  said = []
  for x in transcript(ROOT, "prompt"):
    if operated(str(x[2])):
      said.append(f"## {x[1]}, a message of the owner\n{x[5]}")
    elif str(x[6]) == OPERATOR and str(x[4]) != "None" and over(x[1]) and not isinstance(peek(x[1]), BaseException):
      said.append(f"## {x[1]}, his answer to a question of the lead: {str(x[5])[:400]}\n{peek(x[1])}")
    elif str(x[6]) == OPERATOR and str(x[4]) == "None":
      said.append(f"## {x[1]}, a message of the lead to the owner, made by {x[2]}\n{str(x[5])[:1500]}")
  write(Text("scratch://converge/words-life.md", "# The words of the owner in this life, and the messages of the lead to him, in order\n\n" + "\n\n".join(said) + "\n"))
  done = {str(k) for _, k, _ in desk_ended}
  queues = "\n".join(f"- {t}: width {desk_width.get(t, 0)}; live {[(str(i['key']), kind(str(i['model']))) for i in desk_live[t].values()]}; queued {[(str(x['key']), f'P{priority(x)}', kind(str(x['model'])), [a for a in afters_of(x) if a not in done]) for x in desk_queue[t]]}" for t in TOPICS)
  names = ["ROOM_CAP", "TOPICS", "topic_chain", "desk_width", "desk_fixing", "desk_paused", "desk_ground", "SERIAL2", "FIX_MODEL", "AGG_MODEL", "ASTRA", "VERIFIER", "RANK", "desk_prio", "desk_time", "DISK_FLOOR"]
  values = "\n".join(f"{n} = {globals()[n]!r}" for n in names) + f"\nseats_used() = {seats_used()!r}"
  page = f"# The converge as it runs\n\nThe kit conductor runs the converge; each definition below is the one bound now.\n\nTopics, widths and queues (key, priority, class, the keys that it still waits for):\n{queues}\n\n## Values bound now\n{values}\n\n## FIX_HEAD\n{FIX_HEAD}\n\n## CYCLE\n{CYCLE}\n\n## FINGER\n{FINGER}\n\n## QA_BRIEF\n{QA_BRIEF}\n\n## REVIEW_BRIEF\n{REVIEW_BRIEF}\n\n## WORDS_BRIEF\n{WORDS_BRIEF}\n\n## ALIGN_BRIEF\n{ALIGN_BRIEF}\n\n## PERF_BRIEF\n{PERF_BRIEF}\n\n## The definitions bound now\n{dump(*PAGED)}\n"
  write(Text("scratch://converge/desk.md", page))
  return f"{len(said)} entries, desk.md {len(page)} chars"


def review_rows(n: int) -> list[dict[str, str]]:
  rows = [{"key": f"words-audit-{n}", "model": ASTRA, "brief": WORDS_BRIEF}, {"key": f"perf-{n}", "model": ASTRA, "brief": PERF_BRIEF}]
  return rows + [{"key": f"review-{t}-{n}", "model": ASTRA, "brief": REVIEW_BRIEF.replace("{t}", t)} for t in CODE if t in TOPICS]


def queue_reviews() -> list[str]:
  """Each hour, one audit of the words, one census of speed and one review of each topic, with no second of a kind that waits."""
  review_round["n"] += 1
  words_files()
  answers_file()
  pending = {str(x["key"]).rsplit("-", 1)[0] for x in [*desk_queue[REVIEW], *desk_live[REVIEW].values()]}
  rows = [x for x in review_rows(review_round["n"]) if x["key"].rsplit("-", 1)[0] not in pending]
  desk_queue[REVIEW].extend(rows)
  return [x["key"] for x in rows]


def desk_beat(every: float, floor: float) -> Generator[set[str], object, None]:
  ends_step()
  desk_notes.extend(room_step())
  admit()
  for name in list(topic_chain):
    if said := ground_step(name):
      desk_notes.append(said)
  yield from cycle_step(every, floor)
  yield from ci_step()
  yield from disk_step()
  if clock() - desk_time.get("review_at", 0.0) >= HOUR:
    desk_time["review_at"] = clock()
    queue_reviews()
  for step in SIDE:
    desk_notes.extend(step())
  if said := deps_step():
    desk_notes.append(said)
  if said := triage_step():
    desk_text["triage_last"] = said
    "not applied" in said and desk_notes.append(said)
  admit()
  desk_notes.extend(idle_step())
  if desk_notes and (clock() - desk_time.get("notes_at", 0.0) >= HOUR or any(re.search(URGENT_NOTE, n) for n in desk_notes)):
    prompt(None, "Desk:\n" + "\n\n".join(desk_notes), on=ROOT)
    desk_notes.clear()
    desk_time["notes_at"] = clock()


def watched() -> list[str]:
  keys = ("cycle", "triage_live", "deps_live", "route_live")
  return [*[p for t in TOPICS for p in desk_live[t]], *desk_fixing.values(), *[desk_text[k] for k in keys if desk_text.get(k)], *desk_verify, *words_live]


SIDE = [verify_step, proof_step, gh_step, shots_step, mem_step, words_step, chain_stalls, blanks_and_stuck]


def desk(every: float = EVERY, floor: float = FLOOR, on: str = "") -> Act:
  """The round: one beat at each end of a watched act, or each CI_EVERY seconds; a new desk ends the old one."""

  def ear(id: str) -> Generator[Any, object, None]:
    yield "started", id
    if (old := desk_text.get("id", "")) and get(old) and not over(old):
      close(None, old)
    desk_text["id"] = id
    while desk_text["id"] == id:
      yield from desk_beat(every, floor)
      beat = wait(min(every, CI_EVERY), id, *watched())
      while not over(beat):
        yield {beat}
    yield "done", id, None

  return act("desk", on, controlled(ear), every, floor)


def news(seconds: float = 1800.0, on: str = "") -> Act:
  def ear(id: str) -> Generator[Any, object, None]:
    yield "started", id
    bound = wait(seconds, id)
    while True:
      match (yield {id, "prompt", bound}):
        case "prompt", about, *_ if scope(str(about)) == ROOT:
          yield "done", id, str(about)
          return
        case "done", about, *_ if about == bound:
          yield "done", id, None
          return

  return act("news", on, controlled(ear), seconds)
