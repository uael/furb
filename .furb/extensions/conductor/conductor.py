import json

from furb.engine import (
  HIDDEN,
  OPERATOR,
  ROOT,
  Text,
  acting,
  bash,
  chain,
  clock,
  close,
  cwd,
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
The round merges each green PR into main from the lowest number when its branch is up to date with main, after it moves the PRs that stack on it onto main, and it updates a branch that is behind only when main is green.
The label paused takes a PR out of the round. A draft, the label hold, a review that waits for its fix, and the label owner-merge, the owner merges it or not, keep a PR from a merge.
The round tells you when main is red, when a PR fails to merge or has no change for 45 minutes, and each line of DECISIONS that the owner changed. decide(title, text) adds a decision to it.
Every third round puts the label hold on each PR with a new comment, and tells you each. A comment that starts with [furb is of a worker.
Every twelfth round audits the work against its measure, the open items, and writes its line in the News of DECISIONS.
threads() gives the tree of the threads: the brief prompt of each worker and the prompts under it, with their state, their PRs, their queued items and each question that waits.
debrief(name) asks the chain of a worker that ended for the frictions that it met in furb.
A review blocks only on a defect that a command reproduces, or on a false sentence of a contract."""

CAP = 5
ACTOR = "claude-cli:opus/medium"
DECISIONS = "DECISIONS.md"
NEWS = "No answer needed here. The newest line first.\n"
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
1. Make a worktree: git -C {repo} fetch -q origin main && git -C {repo} worktree add -q {repo}-w-{key} -b w/{key} origin/main, then work there.
2. Change it as the CLAUDE.md files of the repository say.
3. Run only the tests that your change touches.
4. Commit with a message in the style of git log, push the branch, and open the PR with gh pr create --base main{labels}.
5. Remove the worktree, and close with the URL of the PR alone.
Run no real life of a model. Start each comment that you write on GitHub with [furb worker]. Ask the one who gave you this work on its chain: await prompt(str, question, on="{home}").

The item, {key}:
{what}"""

FIX = """You fix the open PR #{n} of the repository {repo}: it {problem} at {sha}. Read it with gh pr view, gh pr checks and gh run view --log-failed. Rerun once a check that fails on one system alone. Else fix it in a worktree of its branch: resolve a conflict with the meaning of both sides, and for a stack read skill("gh-stack") and use gh stack sync. Run the tests that it touches, push with --force-with-lease, remove the worktree, and close with the URL of the PR. Run no real life of a model. Start each comment that you write on GitHub with [furb worker]. Ask the one who gave you this work on its chain: await prompt(str, question, on="{home}")."""

FIXOWN = "Your open PR #{n} {problem} at {sha}. Fetch origin, rebase your branch onto its base or merge its base into it, resolve each conflict with the meaning of both sides, run the gates that the change touches, push with --force-with-lease, and keep its checks green."

DEBRIEF = """Your work is done. As a user of furb, list each friction, defect or confusion that you met in this chain, with its evidence and what would fix it. Close with a list of str, one for each, or an empty list."""

LABELS = "gh label create hold -c FBCA04 -d 'It waits for the fix of a review before a merge' 2>/dev/null; gh label create owner-merge -c 5319E7 -d 'The owner merges it or not' 2>/dev/null; gh label create paused -c BFBFBF -d 'The owner paused it: no fix and no merge' 2>/dev/null; true"

PRS = """gh pr list --state open --limit 200 --json number,isDraft,labels,mergeable,mergeStateStatus,baseRefName,headRefName,headRefOid,statusCheckRollup,updatedAt --jq '[.[] | {n: .number, draft: .isDraft, labels: [.labels[].name], mergeable, state: .mergeStateStatus, base: .baseRefName, head: .headRefName, sha: .headRefOid, checks: [.statusCheckRollup[]? | (.conclusion // .state // "")], idle: ((now - (.updatedAt | fromdateiso8601)) / 60 | floor)}]'
echo ---
gh pr list --state merged --limit 200 --json number --jq '[.[].number]'
echo ---
gh run list --branch main --limit 20 --json headSha,conclusion --jq '.[0].headSha as $s | {sha: ($s // ""), runs: [.[] | select(.headSha == $s) | .conclusion]}'
echo ---
cat DECISIONS.md 2>/dev/null"""

REVIEWS = r"""for n in $(gh pr list --state open --json number --jq '.[].number'); do
gh api "repos/{owner}/{repo}/pulls/$n/comments" --jq '.[] | select(.in_reply_to_id == null) | select(.body | startswith("[furb") | not) | "\(.pull_request_url | split("/") | last) c\(.id) \(.path):\(.line // .original_line) \(.body | gsub("\n"; " "))"'
gh api "repos/{owner}/{repo}/issues/$n/comments" --jq '.[] | select(.body | startswith("[furb") | not) | "\(.issue_url | split("/") | last) i\(.id) - \(.body | gsub("\n"; " "))"'
gh api "repos/{owner}/{repo}/pulls/$n/reviews" --jq '.[] | select(.body != "") | select(.body | startswith("[furb") | not) | "\(.pull_request_url | split("/") | last) r\(.id) - \(.body | gsub("\n"; " "))"'
done"""


def hire(key: str, what: str, thread: str = "", brief: str = "") -> str:
  model = efforts.get(key, ACTOR)
  marks = [f"w:{key}", *([f"thread:{thread}"] if thread else [])]
  if not brief:
    bash("; ".join(f"gh label create {m} --force >/dev/null 2>&1" for m in marks), show=HIDDEN)
  text = (custom.get(key) or brief or WORKER).replace("{labels}", "".join(f" --label {m}" for m in marks))
  text = text.replace("{repo}", cwd()).replace("{key}", key).replace("{home}", scope(acting())).replace("{what}", what)
  where = chain(f"{thread or 'w'}/{key}", source=bases.get(model, ""))
  rung(f"actor = {model!r}", on=where)
  name = str(prompt(str, text, to=model, on=where))
  working[name] = key
  return name


def debrief(name: str) -> str:
  return str(prompt(list[str], DEBRIEF, on=str(get(name)[3])))


def notice(key: str, message: str) -> None:
  if key not in noticed:
    noticed.add(key)
    prompt(None, message)


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


def tend() -> None:
  now = clock()
  for orig, relay in list(relays.items()):
    if relay and over(relay):
      close(peek(relay), id=orig)
      relays[orig] = ""
  for name in [x for x in working if over(x)]:
    results[name] = peek(name)
    prompt(None, f"The worker {working.pop(name)} ({name}) ended with:\n\n{str(results[name])[:2000]}")
  for name, key in working.items():
    where = working_at(name)
    if paused(where):
      wake(where)
    facts = transcript(where)
    for x in [*facts, *(transcript(str(get(name)[3])) if where != get(name)[3] else [])]:
      if x[0] == "prompt" and x[6] == OPERATOR and not over(x[1]) and x[1] not in relays:
        relays[x[1]] = str(prompt(x[4], f"{x[3]} asks the operator:\n\n{x[5]}"))
    if silent.get(where, (-1, now))[0] != len(facts):
      silent[where] = (len(facts), now)
    elif now - silent[where][1] >= 2700:
      notice(
        f"silent {where} {len(facts)}",
        f"The worker {key} on {where} has said nothing for {int(now - silent[where][1]) // 60} minutes. Look at it.",
      )


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
      prompt(None, order, on=working_at(own))
    elif now - since >= 1800 and key + " again" not in noticed:
      noticed.add(key + " again")
      prompt(None, "Second notice, 30 minutes later: " + order, on=working_at(own))
      prompt(None, f"#{n} still {problem} at {sha[:7]} after 30 minutes, so its worker {own} got a second notice.")
  elif key + " fixer" not in noticed:
    noticed.update({key, key + " again", key + " fixer"})
    thread = next((m[7:] for m in pr["labels"] if m.startswith("thread:")), "")
    hire(f"fix{n}", "", thread, FIX.replace("{n}", str(n)).replace("{problem}", problem).replace("{sha}", sha))
    if own:
      prompt(None, f"#{n} still {problem} at {sha[:7]} after an hour, so a fixer is hired for it.")
  return key


async def land(pr: dict, red: bool) -> None:
  n, sha = pr["n"], pr["sha"]
  behind = pr["state"] == "BEHIND"
  if not behind and pr["verdict"] == "pass":
    got = await bash(
      f"gh api 'repos/{{owner}}/{{repo}}/compare/main...{sha}' --jq .behind_by", show=HIDDEN, timeout=120
    )
    if got.code != 0:
      return
    behind = got.stdout.content.strip() != "0"
  if behind:
    if not red and f"update {sha}" not in noticed:
      noticed.add(f"update {sha}")
      await bash(f"gh pr update-branch {n}", show=HIDDEN, timeout=120)
    return
  if pr["verdict"] != "pass":
    return
  moves = "".join(f"gh pr edit {d} --base main >/dev/null 2>&1; " for d, up in prs.items() if up["base"] == pr["head"])
  got = await bash(
    f"{moves}gh pr merge {n} --squash --delete-branch >/dev/null 2>&1 || gh stack merge {n} --yes --squash >/dev/null 2>&1; sleep 5; gh pr view {n} --json state --jq .state",
    show=HIDDEN,
    timeout=400,
  )
  if "MERGED" in got.stdout.content:
    merged.add(n)
    prompt(None, f"#{n} is merged.")
  else:
    notice(f"unmerged {sha}", f"#{n} did not merge at {sha[:7]}. Look at it.")


async def review(seed: bool) -> None:
  got = await bash(REVIEWS, show=HIDDEN, timeout=600)
  fresh = [
    p for p in (x.split(" ", 3) for x in got.stdout.content.splitlines()) if len(p) == 4 and p[1] not in comments
  ]
  comments.update(p[1] for p in fresh)
  if seed or not fresh:
    return
  touched = sorted({p[0] for p in fresh})
  await bash("".join(f"gh pr edit {n} --add-label hold >/dev/null 2>&1; " for n in touched), show=HIDDEN, timeout=300)
  prompt(
    None,
    "New comments on the PRs, each of which now holds the label hold:\n"
    + "\n".join(f"#{p[0]} {p[1]} {p[2]}: {p[3][:600]}" for p in fresh),
  )


def decide(title: str, text: str) -> str:
  doc = read(DECISIONS, HIDDEN).content
  n = 1 + sum(1 for x in doc.splitlines() if x.startswith("## D"))
  entry = f"## D{n}. {title}\n{text}\nOwner:\n\n"
  doc = doc.replace("## News\n", entry + "## News\n", 1) if "## News\n" in doc else doc.rstrip("\n") + "\n\n" + entry
  noted[:] = [write(Text(DECISIONS, doc)).content]
  return f"D{n}"


def audit(red: bool) -> None:
  mine = [n for n, pr in prs.items() if "owner-merge" not in pr["labels"]]
  owner = [n for n in prs if n not in mine]
  stale = [n for n in mine if prs[n]["idle"] >= 60]
  measures.append(len(queue) + len(working) + len(mine))
  issues = [
    text
    for text, bad in (
      ("main is red", red),
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
  if noted and NEWS in noted[0]:
    noted[:] = [write(Text(DECISIONS, noted[0].replace(NEWS, NEWS + line + "\n", 1))).content]
  prompt(
    None,
    f"Audit of the work, with the measures {measures[-6:]}:\n{line}\nIssues: {'; '.join(issues) or 'none'}\nCheck each worker and each PR against it, fix what is off, and go on.",
  )


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
      else "pass"
      if pr["checks"] and all(c in GREEN for c in pr["checks"])
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
    live.add(mend(pr, now))
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
    old = set(noted[0].splitlines())
    prompt(
      None, "The owner changed DECISIONS.md:\n" + "\n".join([x for x in parts[3].splitlines() if x not in old][:60])
    )
  if parts[3]:
    noted[:] = [parts[3]]
  if now - timers.get("review", now - 3 * every) >= 3 * every:
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
