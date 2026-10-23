import json

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
Put each item in queue as (key, brief, thread), then await conduct(). Each round hires a worker for each item, CAP at once, tends the workers and the open PRs, and waits every seconds or until a worker ends.
hire(key, brief, thread) starts a worker from the base chain of its actor in bases, at its actor in efforts or ACTOR, on a chain labeled thread/key, with the brief in custom or WORKER, and its PR carries the labels w:key and thread:thread.
A worker does one item in a worktree of its own, runs only the tests that its change touches, opens its PR, and closes with its URL.
The round tells you a worker that ends, with its answer in results under its prompt, or that says nothing for 45 minutes, relays to you each question that a worker puts to the operator, and wakes a worker that paused.
The round sends a conflict or a failed check at a new head to the worker that owns the PR, again after 30 minutes, and to a fixer after an hour, or at once when no worker owns it.
The round merges each green PR into main from the lowest number when no file that it changes changed on main since its branch forked, after it moves the PRs that stack on it onto main; else it updates the branch, and only when main is green.
The label paused takes a PR out of the round. A draft, the label hold, a review that waits for its fix, and the label owner-merge, until the owner takes it off as his approval, keep a PR from a merge.
The round tells you when main is red, when a PR fails to merge or has no change for 45 minutes, and each comment or issue of the owner in the private repository of the decisions, which PRIVATE names beside this word and git does not hold. await decide(title, text) opens a decision there as an issue, and news(line) adds a line at the top of its digest issue.
Each round queues each open issue that the owner opened, or that he assigned to himself, as an item issueN, once, and its PR closes it.
Each round sends each new comment of the owner on a PR or on his issue to the worker that owns it, puts the label hold on that PR, and tells you only the comments that no worker owns. A text on GitHub counts only when the owner wrote it with no 🤖 at its start, or when he put a 👍 on it; each worker starts what it writes there with 🤖.
Each round sets one status label on each open PR and on each issue of the owner that it took: s:queued, s:working, s:checks, s:red, s:conflict, s:ready, or s:owner (green, waits for the owner).
Every twelfth round audits the work against its measure, the open items, and writes its line in the digest.
threads() gives the tree of the threads: the brief prompt of each worker and the prompts under it, with their state, their PRs, their queued items and each question that waits.
debrief(name) asks the chain of a worker that ended for the frictions that it met in furb.
Each act of the round is made through quietly(verb, ...), so its end wakes nobody: you hear only a notice, and each merge rides on the next one. Make each act that you do not await the same way.
A review blocks only on a defect that a command reproduces, or on a false sentence of a contract."""

CAP = 6
ACTOR = "claude-cli:opus/medium"
DECISIONS = "DECISIONS.md"
NEWS = "No answer needed here. The newest line first.\n"
GATE = "the hooks"
FAILS = ("FAILURE", "ERROR", "TIMED_OUT", "STARTUP_FAILURE", "ACTION_REQUIRED")
GREEN = ("SUCCESS", "SKIPPED", "NEUTRAL")
queue: list[tuple[str, str, str]] = []
working: dict[str, str] = {}
results: dict[str, object] = {}
relays: dict[str, str] = {}
bases: dict[str, str] = {}
efforts: dict[str, str] = {}
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

WORKER = """You are a worker on the repository {repo}. Do one item, and open its PR yourself.
1. Make a worktree whose build starts warm: {repo}/script/worktree.sh {repo}-w-{key} w/{key}, then work there. Build on origin/main, and stack your branch on an open PR only when your change needs its code; say why in the body, since a stack lands one layer at a time. If that worktree stands already, work in it, rebased onto origin/main. Run each build of Rust through sh $TMPDIR/build.sh <command>, which runs three at once at most.
2. Change it as the AGENTS.md files of the repository say.
3. Run only the tests that your change touches.
4. Commit with the title of the PR as its message, and push the branch. Then open the PR, or update it when it stands, with pr(), which fills the template of the owner from what you give it and checks each of its rules: await pr(title, why, outcome, checks, breaking=[], fixes=0, where=<your worktree>, labels="{labels}"). title is type(area): What changed, 70 characters or fewer: the type is feat, fix, perf, or chore for a change that no user sees, and the area is the part that owns the change (engine, crate, tui, gui, provider, ci, conductor, or an extension by its name). why is the problem and why it matters now, from your own work and not from git log. outcome is a list: what is true now that was not, each with the evidence of what you ran, and each gap that you leave. breaking lists each change that forces a caller of a door or of the contract to change code. 300 words at most, and no how, which the diff holds. checks is six lines, one for each box of the checklist of the template in its order: verify each box one by one, then give the evidence of how you verified it, or not: and the work that it still needs, which leaves that box open. A pr() that breaks a rule raises Refused and says why; never write the body of a PR by hand.
5. Remove the worktree with git worktree remove --force, and close with a tuple: the number of your PR, or 0 when the item asked for no change, and your report in 200 words at most: what you measured, what you decided, and what is left.
Run no real life of a model. Start each comment, review, issue and PR body that you write on GitHub with 🤖, since you write from the owner's account and his own words carry no mark; a title keeps its own rule. Never take off the label owner-merge: only the owner takes it off, and that is his approval to merge. The owner sleeps or is away: ask him nothing and wait for nothing of his. Take each decision yourself from the brief and the rulings, say it in your PR body, and go on. Ask the one who gave you this work on its chain only when it alone can decide: await prompt(str, question, on="{home}").

The item, {key}:
{what}"""

FIX = """You fix the open PR #{n} of the repository {repo}: it {problem} at {sha}. Read it with gh pr view, gh pr checks and gh run view --log-failed. Rerun once a check that fails on one system alone. Else fix it in a worktree of its branch: resolve a conflict with the meaning of both sides, and for a stack read skill("gh-stack") and use gh stack sync. Run the tests that it touches, push with --force-with-lease, remove the worktree, and close with a tuple: the number of the PR, and your report in 200 words at most. Run no real life of a model. Start each comment, review, issue and PR body that you write on GitHub with 🤖, since you write from the owner's account and his own words carry no mark; a title keeps its own rule. Never take off the label owner-merge: only the owner takes it off, and that is his approval to merge. The owner sleeps or is away: ask him nothing and wait for nothing of his. Take each decision yourself from the brief and the rulings, say it in your PR body, and go on. Ask the one who gave you this work on its chain only when it alone can decide: await prompt(str, question, on="{home}")."""

FIXOWN = "Your open PR #{n} {problem} at {sha}. Fetch origin, rebase your branch onto its base or merge its base into it, resolve each conflict with the meaning of both sides, run the gates that the change touches, push with --force-with-lease, and keep its checks green."

DEBRIEF = """Your work is done. As a user of furb, list each friction, defect or confusion that you met in this chain, with its evidence and what would fix it. Close with a list of str, one for each, or an empty list."""

LABELS = "gh label create hold -c FBCA04 -d 'It waits for the fix of a review before a merge' 2>/dev/null; gh label create owner-merge -c 5319E7 -d 'The owner merges it or not' 2>/dev/null; gh label create paused -c BFBFBF -d 'The owner paused it: no fix and no merge' 2>/dev/null; true"

PRS = """gh pr list --state open --limit 200 --json number,isDraft,labels,mergeable,mergeStateStatus,baseRefName,headRefName,headRefOid,statusCheckRollup,updatedAt,body --jq '[.[] | {n: .number, draft: .isDraft, labels: [.labels[].name], mergeable, state: .mergeStateStatus, base: .baseRefName, head: .headRefName, sha: .headRefOid, checks: [.statusCheckRollup[]? | (.conclusion // .state // "")], names: [.statusCheckRollup[]? | (.name // .context // "")], open: [(.body // "") | splits("\\n") | select(startswith("- [ ] "))], idle: ((now - (.updatedAt | fromdateiso8601)) / 60 | floor)}]'
echo ---
gh pr list --state merged --limit 200 --json number --jq '[.[].number]'
echo ---
gh run list --branch main --workflow gates.yml --limit 20 --json headSha,conclusion --jq '.[0].headSha as $s | {sha: ($s // ""), runs: [.[] | select(.headSha == $s) | .conclusion]}'
echo ---
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


def pr_body(why: str, outcome: list[str], breaking: list[str], fixes: int, checks: list[str]) -> str:
  parts = ["🤖" + (f" Fixes #{fixes}." if fixes else ""), "## Why", why.strip(), "## Outcome", "\n".join(f"- {x.strip()}" for x in outcome)]
  if breaking:
    parts += ["## Breaking changes", "\n".join(f"- {x.strip()}" for x in breaking)]
  return "\n\n".join([*parts, "---", "\n".join(pr_box(b, c) for b, c in zip(PR_BOXES, checks))]) + "\n"


def pr(title: str, why: str, outcome: list[str], checks: list[str], breaking: list[str] | None = None, fixes: int = 0, where: str = "", labels: str = "", number: int = 0, base: str = "main") -> Act:
  if len(title) > 70 or not re.fullmatch(PR_TITLE, title):
    raise Refused(f"the title {title!r} is not type(area): What changed, in 70 characters or fewer")
  if not why.strip() or not outcome:
    raise Refused("a PR needs a why and at least one outcome")
  if len(checks) != len(PR_BOXES) or not all(c.strip() and c.strip().lower() != "not:" for c in checks):
    raise Refused(f"checks needs {len(PR_BOXES)} lines, one for each box of the checklist in its order: the evidence of how you verified that box, or not: and the work that it still needs")
  words = len(" ".join([why, *outcome, *(breaking or [])]).split())
  if words > 300:
    raise Refused(f"the body holds {words} words, over 300: delete a whole paragraph, not a word")
  body = pr_body(why, outcome, breaking or [], fixes, checks)
  name = shq(title)
  rules = 'R=CLAUDE.md; [ -f AGENTS.md ] && R=AGENTS.md; sed "s/{rules}/$R/g" | '
  if number:
    command = rules + f"gh pr edit {number} --title {name} --body-file - && gh pr view {number} --json url --jq .url"
  else:
    added = labels.replace("--label", "--add-label")
    command = (f"cd {shq(where)} && " if where else "") + rules + f"if gh pr view --json number >/dev/null 2>&1; then gh pr edit --title {name} --body-file -{added}; else gh pr create --base {base} --title {name} --body-file -{labels}; fi && gh pr view --json url --jq .url"
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


def hire(key: str, what: str, thread: str = "", brief: str = "") -> str:
  model = efforts.get(key, ACTOR)
  marks = [f"w:{key}", *([f"thread:{thread}"] if thread else [])]
  if not brief:
    quietly(bash, "; ".join(f"gh label create {m} --force >/dev/null 2>&1" for m in marks), show=HIDDEN)
  text = (custom.get(key) or brief or WORKER).replace("{labels}", "".join(f" --label {m}" for m in marks))
  text = text.replace("{repo}", cwd()).replace("{key}", key).replace("{home}", scope(acting())).replace("{what}", what)
  where = chain(f"{thread or 'w'}/{key}", source=bases.get(model, ""))
  quietly(rung, f"actor = {model!r}", on=where)
  name = str(quietly(prompt, tuple[int, str], text, to=model, on=where))
  working[name] = key
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
  keys = [m[2:] for m in pr["labels"] if m.startswith("w:")]
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
    results[name] = peek(name)
    got = results[name]
    said = f"PR #{got[0]}\n\n{got[1]}" if isinstance(got, tuple) and len(got) == 2 and got[0] else str(got)
    quietly(prompt, None, f"The worker {working.pop(name)} ({name}) ended with:\n\n{said[:2000]}" + merged_since())
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


def boxes(pr: dict, now: float) -> str:
  if not pr.get("open"):
    return ""
  n, sha = pr["n"], pr["sha"]
  key = f"#{n} {sha} open boxes"
  since = fixes.setdefault(key, now)
  own = owner_of(pr)
  order = (
    f"Your open PR #{n} has a box of its checklist open at {sha[:7]}:\n"
    + "\n".join(x[6:] for x in pr["open"])
    + f"\nDo the work that each open box names, verify it, then call pr(..., number={n}) again with the evidence of each box, so pr() ticks it. The round merges no PR with an open box."
  )
  if own and now - since < 3600:
    if key not in noticed:
      noticed.add(key)
      quietly(prompt, None, order, on=working_at(own))
  elif key + " fixer" not in noticed and len(working) < CAP:
    noticed.update({key, key + " fixer"})
    thread = next((m[7:] for m in pr["labels"] if m.startswith("thread:")), "")
    brief = (
      f"You close the open boxes of the checklist of the open PR #{n} of the repository "
      + "{repo}. "
      + order.replace("Your open PR", "The PR")
      + " Read it with gh pr view, work in a worktree of its branch, push with --force-with-lease, remove the worktree, and close with a tuple: the number of the PR, and your report in 200 words at most. Run no real life of a model. Start each comment that you write on GitHub with 🤖. Ask the one who gave you this work on its chain: await prompt(str, question, on=\"{home}\")."
    )
    hire(f"box{n}", "", thread, brief)
  return key


def mend(pr: dict, now: float) -> str:
  n, sha = pr["n"], pr["sha"]
  problem = (
    "conflicts with its base"
    if pr["mergeable"] == "CONFLICTING"
    else "has a failed check"
    if pr["verdict"] == "fail"
    else ""
  )
  if not problem:
    return ""
  key = f"#{n} {sha} {problem}"
  since = fixes.setdefault(key, now)
  own = owner_of(pr)
  order = FIXOWN.replace("{n}", str(n)).replace("{problem}", problem).replace("{sha}", sha)
  if own and now - since < 3600:
    if key not in noticed:
      noticed.add(key)
      quietly(prompt, None, order, on=working_at(own))
    elif now - since >= 1800 and key + " again" not in noticed:
      noticed.add(key + " again")
      quietly(prompt, None, "Second notice, 30 minutes later: " + order, on=working_at(own))
  elif key + " fixer" not in noticed and len(working) < CAP:
    noticed.update({key, key + " again", key + " fixer"})
    thread = next((m[7:] for m in pr["labels"] if m.startswith("thread:")), "")
    hire(f"fix{n}", "", thread, FIX.replace("{n}", str(n)).replace("{problem}", problem).replace("{sha}", sha))
  return key


async def land(pr: dict, red: bool) -> None:
  n, sha = pr["n"], pr["sha"]
  if pr["verdict"] != "pass":
    return
  same = await bash(
    f"git fetch -q origin main refs/pull/{n}/head 2>/dev/null; [ \"$(git merge-tree --write-tree origin/main {sha} 2>/dev/null | head -1)\" = \"$(git rev-parse {sha}^{{tree}})\" ] && echo same",
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
      if not red and f"update {sha}" not in noticed:
        noticed.add(f"update {sha}")
        await bash(f"gh pr update-branch {n}", show=HIDDEN, timeout=120)
      return
  moves = "".join(f"gh pr edit {d} --base main >/dev/null 2>&1; " for d, up in prs.items() if up["base"] == pr["head"])
  got = await bash(
    f"{moves}gh pr merge {n} --squash --delete-branch >/dev/null 2>&1 || gh stack merge {n} --yes --squash >/dev/null 2>&1; sleep 5; gh pr view {n} --json state --jq .state",
    show=HIDDEN,
    timeout=400,
  )
  if "MERGED" in got.stdout.content:
    merged.add(n)
    landed.append(n)
    kids = [d for d, up in prs.items() if up["base"] == pr["head"]]
    if kids:
      await bash("sleep 10; " + "; ".join(f"gh pr edit {d} --base main >/dev/null 2>&1; gh pr update-branch {d} >/dev/null 2>&1" for d in kids), show=HIDDEN, timeout=300)
  else:
    notice(f"unmerged {sha}", f"#{n} did not merge at {sha[:7]}. Look at it.")


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


def audit(red: bool) -> None:
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
  quietly(
    prompt,
    None,
    f"Audit of the work, with the measures {measures[-6:]}:\n{line}\nIssues: {'; '.join(issues) or 'none'}\nCheck each worker and each PR against it, fix what is off, and go on.",
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
    notice(f"issues {int(clock() // 3600)}", "The command that lists the issues failed: " + got.stderr.content[-500:])
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


RUNS = r"""gh api 'repos/{owner}/{repo}/actions/runs?per_page=100' --jq '.workflow_runs[] | select(.event == "pull_request" and (.status == "queued" or .status == "in_progress" or .status == "waiting")) | "\(.id) \(.head_branch) \(.head_sha)"'"""


async def prune_runs() -> None:
  heads = {pr["head"]: pr["sha"] for pr in prs.values()}
  got = await bash(RUNS, show=HIDDEN, timeout=120)
  stale = [p[0] for p in (x.split(" ") for x in got.stdout.content.splitlines()) if len(p) == 3 and p[1] in heads and p[2] != heads[p[1]]]
  if stale:
    await bash("; ".join(f"gh run cancel {r} >/dev/null 2>&1" for r in stale), show=HIDDEN, timeout=300)


async def sweep(every: float) -> None:
  got = await bash(PRS, show=HIDDEN, timeout=300)
  parts = got.stdout.content.split("\n---\n", 3)
  if len(parts) < 4 or not parts[0].startswith("["):
    notice(f"list {int(clock() // 3600)}", "The command that lists the PRs failed: " + got.stderr.content[-500:])
    return
  prs.clear()
  timers["sweep"] = clock()
  for pr in json.loads(parts[0]):
    pr["verdict"] = (
      "fail"
      if any(c in FAILS for c in pr["checks"])
      else "open"
      if pr.get("open")
      else "pass"
      if GATE in pr.get("names", []) and all(c in GREEN for c in pr["checks"])
      else "pending"
    )
    prs[pr["n"]] = pr
  merged.update(json.loads(parts[1] or "[]"))
  main = json.loads(parts[2] or '{"sha": "", "runs": []}')
  red = "failure" in main["runs"]
  if red:
    notice(f"red {main['sha']}", f"main is red at {str(main['sha'])[:7]}. Find its cause first.")
  now = clock()
  live = set()
  for n in sorted(prs):
    pr = prs[n]
    if "paused" in pr["labels"]:
      continue
    if not pr["head"].startswith("w/"):
      notice(f"foreign #{n}", f"#{n} ({pr['head']}) was not made by a worker of the round, so the round neither mends nor merges it.")
      continue
    live.add(mend(pr, now))
    live.add(boxes(pr, now))
    if "owner-merge" in pr["labels"]:
      continue
    if pr["idle"] >= 45:
      notice(
        f"idle {pr['sha']}",
        f"#{n} has had no change for {pr['idle']} minutes{' as a draft' if pr['draft'] else ''}{' with the label hold' if 'hold' in pr['labels'] else ''}. Move it on, or say why it waits.",
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
  await ops_watch()
  await mark()
  await prune_runs()
  if now - timers.get("review", now - every) >= every:
    await review("review" not in timers)
    timers["review"] = now
  if now - timers.setdefault("audit", now) >= 12 * every:
    audit(red)
    timers["audit"] = now


async def conduct(every: float = 300) -> dict[str, object]:
  await bash(LABELS, show=HIDDEN)
  while True:
    while queue and len(working) < CAP:
      hire(*queue.pop(0))
    tend()
    await sweep(every)
    if not (
      queue or working or any("owner-merge" not in pr["labels"] and "paused" not in pr["labels"] for pr in prs.values())
    ):
      return results
    await wait(every, *working, *[x for x in relays.values() if x])


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
