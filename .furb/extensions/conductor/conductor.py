from collections.abc import Generator
from typing import Any

from furb.engine import (
  HIDDEN,
  OPERATOR,
  ROOT,
  Act,
  Exit,
  Text,
  act,
  bash,
  chain,
  clock,
  controlled,
  dump,
  filled,
  operated,
  over,
  peek,
  prompt,
  scope,
  transcript,
  under,
  wait,
  write,
)

ROOM = {"opus/high": 3, "opus/medium": 3, "sonnet/high": 3, "sonnet/medium": 3, "gpt-6-astra/high": 2}
CLASSES = {one: f"claude-cli:{one}" for one in ("opus/high", "opus/medium", "sonnet/high", "sonnet/medium")}
WIDTH = {"engine": 2, "host": 2, "tui": 3, "skills": 1, "rust": 2, "ci": 2, "tidy": 1, "review": 2, "qa": 1}
AGG, QA, REVIEW = "converge", "qa", "review"
FIX_MODEL, AGG_MODEL, QA_MODEL = CLASSES["sonnet/high"], CLASSES["opus/medium"], CLASSES["opus/medium"]
ASTRA = "codex:gpt-6-astra/high"
EVERY, FLOOR, QA_EVERY, REVIEW_EVERY, REGROUND = 300.0, 900.0, 900.0, 3600.0, 0.5
REPO = "~/src/github.com/uael/furb"
PAGE, WORDS = "scratch://converge/desk.md", "scratch://converge/words-life.md"
PAGED = ["conduct", "beat", "next_item", "seats", "fix", "ground_step", "queue_rows", "queue_reviews", "news", "words"]
CLAIM = ' Judge the row against main first: it is a claim of an audit.'
QA_BRIEF = 'A QA run of the tip of w/converge, by the charter of this chain. If origin/w/converge stands at the sha of the last run in scratch://converge/qa.md, close at once with: same head <sha>. Else run it, and append to scratch://converge/qa.md ($TMPDIR/converge/qa.md) the sha, the minutes that the run took, the result of the scenarios, and each finding with its capture. Close with the rows of the new findings, or with clean, the sha and the minutes.'
WORDS_BRIEF = 'The audit of the words of the owner, top 10 and the bar, read only. Read whole: scratch://converge/words-life.md (each message of the owner in this life and each of his answers, in order), ~/Library/Caches/furb/handoff/converge/plan.md, the picks (~/Library/Caches/furb/handoff/converge/decisions/picks.md, with its rulings N, N2, N3, A, O and W), and scratch://converge/desk.md (the code and the values that run the converge now: the desk, its queues, the method of the topics, the head of a fix and the brief of the aggregator). In the ledger ~/Library/Caches/furb/handoff/converge/asks.json, read the asks that these words name or change. For each ask in these words (an order, a rule, a number, a choice, or a question that wants a change), find what serves it: a line of the code or a value of desk.md, a queued or ended item, a commit on origin (git log origin/w/converge and origin/c/*), or a file of scratch://converge/. A later word of the owner rules over an earlier one. Write scratch://converge/words-audit.md ($TMPDIR/converge/words-audit.md): the missing and partly served asks first, then a table of each ask with its source id and its state (served, with where; queued, with the item; partly, with what lacks; missing; or overruled, with the later word). State each ask in plain English and quote no words. Close with the missing and partly served asks, one line each: the source id, the ask in one sentence, and the change that serves it (an item of a topic with its class of model and its paths, or a change of the desk for the lead).'
ALIGN_BRIEF = 'The audit of alignment, top 10 and the bar, read only. Read the ledger of the asks (~/Library/Caches/furb/handoff/converge/asks.json), the top 10 (~/Library/Caches/furb/handoff/converge/top10.md) and the picks (~/Library/Caches/furb/handoff/converge/decisions/picks.md). Then run git fetch origin and read the commits of w/converge since the last audit: git log --no-merges --stat <base>..origin/w/converge, where <base> is the sha that the last entry of scratch://converge/align.md names, else origin/main, and git show for each commit that serves an ask of the top 3. Judge whether each change goes the way of its ask or drifts from it. Name each change that serves no ask or goes against one, and name the asks of the top 3 of the top 10 that no change serves yet. Append to scratch://converge/align.md ($TMPDIR/converge/align.md) the sha that you read and these lists. Close with the rows of the drifts and of the gaps, by the charter of this chain.'
PERF_BRIEF = 'The census of speed, again, read only: by the charter of this chain, compare with the last entries of scratch://converge/perf.md and scratch://converge/ci-time.md. Read the times of the jobs and steps of the latest runs of the gates on w/converge (gh run list, then gh api repos/uael/furb/actions/runs/<id>/jobs), the numbers of bun run bench in the log of the job of the speed of the TUI, the slowest tests that the logs now print (pytest --durations and the times of the cargo binaries), and the token walls of engine.py and DOCTRINE.md. Append to scratch://converge/perf.md the sha, each measure against the last entry, each regression past a fifth and each gain. Close with a row for each regression and each new lever.'
REVIEW_BRIEF = 'The review of the paths of the topic {t}, top 3 and the bar, read only. Run git fetch origin. Read each change that the converge made in the paths that scratch://converge/{t}.md gives to {t}, whichever topic made it: git log --no-merges -p <base>..origin/w/converge -- <paths>, where <base> is the sha of origin/w/converge that the last entry of scratch://converge/review-{t}.md names (an entry that names only a sha of c/{t} counts as none, so take origin/main),, the resolutions of the merges in those paths (git log -p --cc --merges <base>..origin/w/converge -- <paths>), and then the commits of the topic that w/converge does not hold yet: git log --no-merges -p origin/w/converge..origin/c/{t}. Review each change as an expert reviewer, by the charter of this chain. Append to scratch://converge/review-{t}.md ($TMPDIR/converge/review-{t}.md) the sha of origin/w/converge and of origin/c/{t} that you read, a verdict (clean, or the count of findings), and each finding with its commit, its path and lines, the rule that it breaks and its fix. Close with the rows of the findings, or with clean and the two shas.'
FIX_HEAD = "A failure of the CI of w/converge in a path that this topic owns. It goes before your other items: fix it at its cause in your worktree, as one commit, with the checks of what you touched, then push your branch. Close with the sha and a report. The live items of this topic may hold uncommitted edits in the tree: commit only the paths of your fix (git add <path>), and leave their edits as they stand. A test in a path of another topic that your topic's change made wrong is yours to fix in your commit, by the carry rule: name its path in the body, and never leave it as a Needs line. Commit with the paths named (git commit <path>... -m <title>), since the index may hold the edits of another item. The brief of the aggregator:\n\n"
CYCLE = 'One cycle of the aggregator. The charter of this chain, scratch://converge/converge.md, holds your rules and the paths of each topic.\n1. In your worktree: git fetch origin, then merge each topic branch that holds commits that w/converge lacks, in the order engine, host, tui, skills, rust, ci, with git merge --no-ff --no-edit c/<topic> (the local branches of this clone, which the topics also push to origin). Resolve a conflict by the meaning of both sides. A conflict that needs the knowledge of a topic goes to that topic as a failure: abort that merge and route it.\n2. When a merge changes the source of a generated file, make the file again (bun run protocol, bun run build, bun run docs) and commit it. Run the quick checks of what the merges touched, through sh script/build-slot.sh, never the whole suite.\nThen apply each line of scratch://converge/converge-needs.md that is not marked done, as one commit of the paths that you own, and mark it done there with its sha.\n3. Push w/converge. While no PR stands for it, open one ready, never a draft, with the label hold so that it never merges by itself: await pr(title, why, outcome, checks, where=<your worktree>, labels="hold") of the skill pr, whose rules the template gives. Its body says that the branch converges the work of the owner since 11-03, topic by topic, and lists the PRs that it carries. Update the body when a cycle adds a topic or a fold, with one line "Fixes #N." for each issue that scratch://converge/issues.md says that w/converge carries, once the commits that fix it stand on the branch.\n4. Read the checks of the PR once, with no wait: gh pr checks <n> --json name,bucket,link. For each failed check of the newest head that ran, read the log of its failed job (gh api repos/uael/furb/actions/jobs/<id>/logs | grep and tail) and find the failing test or file, then the topic that owns that path. A check that main fails too goes to the topic of its path as well, marked as a failure that main shares, since w/converge must be green to merge.\n5. Close with a tuple: a dict that maps each topic to the brief of its failures (the check, the link of the job, the head, the failing test or file, and the lines of the log that show the cause), with each failure routed once, and a summary in 150 words at most: the commits merged from each topic, the head pushed, and the state of the checks. Route nothing to a topic twice for the same failure at the same head: say in the summary that it is still red.\n\n\nBefore each push of w/converge, run on the merged head, through sh script/build-slot.sh: cargo clippy --workspace --all-targets -- -D warnings, bun run lint, bun run check and python3 script/rustwidth.py origin/main. Route each finding at once to the topic that owns its path, as a failure of CI, before the push, so the topic fixes it while the CI runs, and the CI finds only what needs the three systems.'

width: dict[str, int] = globals().setdefault("width", dict(WIDTH))
chains: dict[str, str] = globals().setdefault("chains", {})
queues: dict[str, list[dict]] = globals().setdefault("queues", {})
live: dict[str, dict[str, dict]] = globals().setdefault("live", {})
ended: list[tuple[str, str, str]] = globals().setdefault("ended", [])
fixing: dict[str, str] = globals().setdefault("fixing", {})
fixwait: dict[str, str] = globals().setdefault("fixwait", {})
ground: dict[str, str] = globals().setdefault("ground", {})
stuck: dict[str, str] = globals().setdefault("stuck", {})
notes: list[str] = globals().setdefault("notes", [])
state: dict[str, Any] = globals().setdefault(
  "state", {"id": "", "cycle": "", "due": "", "print": "", "qa_print": "", "last": 0.0, "cycle_at": 0.0, "qa_at": 0.0, "review_at": 0.0, "qa": 0, "reviews": 0}
)


def chain_of(t: str) -> str:
  if t not in chains:
    chains[t] = str(chain(t))
  return chains[t]


def queue(t: str) -> list[dict]:
  return queues.setdefault(t, [])


def kind(model: str) -> str:
  return model.split(":")[-1]


def seats() -> dict[str, int]:
  models = [str(item["model"]) for held in live.values() for p, item in held.items() if not over(p)]
  models += [FIX_MODEL for one in fixing.values() if not over(one)]
  models += [AGG_MODEL] if state["cycle"] and not over(state["cycle"]) else []
  used: dict[str, int] = {}
  for one in models:
    used[kind(one)] = used.get(kind(one), 0) + 1
  return used


def seat(model: str) -> bool:
  return seats().get(kind(model), 0) < ROOM.get(kind(model), 0)


def fix(t: str, why: str) -> str:
  if not seat(FIX_MODEL):
    fixwait[t] = why
    return ""
  fixwait.pop(t, None)
  fixing[t] = str(prompt(str, FIX_HEAD + why, to=FIX_MODEL, on=chain_of(t)))
  return fixing[t]


def item_text(t: str, item: dict) -> str:
  return f"Item {item['key']} of the topic {t}. The charter of this chain, scratch://converge/{t}.md, holds its rules, its paths and the asks of the owner that it carries.\n\n{item['brief']}"


def next_item(t: str, now: float) -> dict | None:
  if t == QA and not queue(QA) and not live.get(QA) and state["print"] != state["qa_print"] and now - state["qa_at"] >= QA_EVERY:
    state.update(qa=state["qa"] + 1, qa_print=state["print"], qa_at=now)
    queue(QA).append({"key": f"qa-run-{state['qa']}", "model": QA_MODEL, "brief": QA_BRIEF})
  done, used = {k for _, k, _ in ended}, seats()
  held = ({kind(FIX_MODEL)} if fixwait else set()) | ({kind(AGG_MODEL)} if state["due"] else set())
  for n, item in enumerate(queue(t)):
    k = kind(str(item["model"]))
    if k not in held and all(one in done for one in item.get("after", [])) and used.get(k, 0) < ROOM.get(k, 0):
      return queue(t).pop(n)
  return None


def watch(t: str) -> None:
  why = ""
  if queue(t) and not live.get(t) and not fixing.get(t):
    why = f"{t} holds {len(queue(t))} queued items and runs none: " + (f"its fix waits for a seat of {kind(FIX_MODEL)}" if t in fixwait else "no item has its after done and a seat of its class free")
  if why and stuck.get(t) != why:
    notes.append(why)
  stuck[t] = why


def grounding(acts: list[tuple]) -> list[tuple]:
  asked = [a[1] for a in acts if a[0] == "prompt"]
  open_ = [a[1] for a in acts if a[0] == "prompt" and not over(a[1])]
  return [a for a in acts if (a[0] == "read" and not any(under(a[1], p) for p in asked)) or any(under(a[1], p) for p in open_)]


def kept_reads(ids: set[str]):  # noqa: ANN201
  def keep(acts: list[tuple]) -> list[tuple]:
    open_ = [a[1] for a in acts if a[0] == "prompt" and not over(a[1])]
    return [a for a in grounding(acts) if (a[0] == "read" and a[1] in ids) or any(under(a[1], p) for p in open_)]

  return keep


def ground_note(t: str) -> str:
  return f"This chain is the topic {t} of the converge. It was regrounded past {REGROUND:.0%} of its window: it keeps the reads that grounded it, read again to show what changed, and its open items. The work of each ended item stands in the commits of its branch. Its charter, scratch://converge/{t}.md, holds its rules, its paths and the asks of the owner that it carries."


def ground_step(t: str) -> str:
  c = chains[t]
  replies = [str(x[1]) for x in transcript(c, "reply") if over(x[1])]
  last = replies[-1] if replies else ""
  share = (filled(last) or 0.0) if last else 0.0
  mark = ground.get(t, "")
  if mark.startswith("c:"):
    if not over(pick := mark[2:]):
      return ""
    got = peek(pick)
    ground[t] = "g:" + last
    if isinstance(got, tuple) and len(got) == 2 and isinstance(got[0], list):
      ids = {str(i) for i in got[0]}
      compact([kept_reads(ids), latest()], summary=ground_note(t) + "\n" + str(got[1]).strip(), on=c)
      return f"{t} ({c}) still read {share:.0%} of its window after its reground, so a cheap model chose the acts to keep ({len(ids)})."
    return f"{t} ({c}): the cheap choice of its reads gave {str(got)[:300]}, so the chain stays as it is."
  if not last or share <= REGROUND:
    if mark.startswith("g:") and mark != "g:" + last:
      ground[t] = ""
    return ""
  if mark == "g:" + last:
    return ""
  if mark.startswith("g:"):
    ground[t] = "c:" + str(choice(on=c))
    return ""
  compact([grounding, latest()], summary=ground_note(t), on=c)
  ground[t] = "g:" + last
  return f"{t} ({c}) read {share:.0%} of its window, so it was regrounded on its reads."


def finger() -> str:
  heads = "\n".join(f'echo "c/{t} $(git -C {REPO} rev-parse --short c/{t} 2>/dev/null)"' for t in width)
  checks = "gh pr checks \"$n\" --json bucket --jq '[.[] | .bucket] | group_by(.) | map(\"\\(.[0]) \\(length)\") | join(\", \")'"
  return f"{heads}\nn=$(gh pr list --head w/{AGG} --state open --json number --jq '.[0].number // empty')\necho \"pr ${{n:-none}}\"\n[ -n \"$n\" ] && {checks}\ntrue\n"


def beat() -> Generator[set[str], object, None]:
  now = clock()
  for t in width:
    held = live.setdefault(t, {})
    if (one := fixing.get(t, "")) and over(one):
      notes.append(f"{t} ended the fix {one} of CI: {str(peek(one))[:2000]}")
      fixing.pop(t)
    if t in fixwait and not fixing.get(t) and seat(FIX_MODEL):
      fix(t, fixwait[t])
    for p in [p for p in held if over(p)]:
      item = held.pop(p)
      ended.append((t, str(item["key"]), p))
      notes.append(f"{t}: the item {item['key']} ({p}, {item['model']}) ended: {str(peek(p))[:3000]}")
    while not fixing.get(t) and t not in fixwait and len(held) < width[t] and (item := next_item(t, now)):
      held[str(prompt(str, item_text(t, item), to=item["model"], on=chain_of(t)))] = item
    watch(t)
  for t in [t for t in [*width, AGG] if t in chains]:
    if said := ground_step(t):
      notes.append(said)
  if (cycle := state["cycle"]) and over(cycle):
    got, state["cycle"] = peek(cycle), ""
    if isinstance(got, tuple) and len(got) == 2 and isinstance(got[0], dict):
      for t, why in got[0].items():
        if t in width and why and not fixing.get(t) and t not in fixwait:
          fix(t, str(why))
      notes.append(f"cycle {cycle}: {got[1]}" + (f" Sent to {', '.join(got[0])}." if got[0] else ""))
    else:
      notes.append(f"cycle {cycle} ended with {str(got)[:1500]}")
  if not state["cycle"] and now - state["last"] >= EVERY:
    ran = bash(finger(), show=HIDDEN, ok=True, timeout=120)
    while not over(ran):
      yield {ran}
    out = peek(ran)
    seen = out.stdout.content if isinstance(out, Exit) else ""
    state["last"] = now
    if seen != state["print"] and not ("pending" in seen and now - state["cycle_at"] < FLOOR):
      if seat(AGG_MODEL):
        state.update(print=seen, due="", cycle_at=now)
        state["cycle"] = str(prompt(tuple[dict[str, str], str], CYCLE, to=AGG_MODEL, on=chain_of(AGG)))
      else:
        state["due"] = "1"
  if now - state["review_at"] >= REVIEW_EVERY:
    state["review_at"] = now
    queue_reviews()
  if notes:
    prompt(None, "Conductor:\n" + "\n\n".join(notes), on=ROOT)
    notes.clear()


def conduct(on: str = "") -> Act:
  def ear(id: str):
    yield "started", id
    state["id"] = id
    while state["id"] == id:
      yield from beat()
      watched = [*(p for held in live.values() for p in held), *fixing.values(), *([state["cycle"]] if state["cycle"] else [])]
      bound = wait(EVERY, id, *watched)
      while not over(bound):
        yield {bound}
    yield "done", id, None

  return act("conduct", on, controlled(ear), EVERY, FLOOR)


def review_rows(n: int) -> list[dict[str, str]]:
  rows = [{"key": f"{name}-{n}", "model": ASTRA, "brief": brief} for name, brief in (("words-audit", WORDS_BRIEF), ("align-audit", ALIGN_BRIEF), ("perf", PERF_BRIEF))]
  return rows + [{"key": f"review-{t}-{n}", "model": ASTRA, "brief": REVIEW_BRIEF.replace("{t}", t)} for t in width if t not in (REVIEW, QA, "tidy")]


def queue_reviews() -> list[str]:
  state["reviews"] += 1
  words()
  pending = {str(x["key"]).rsplit("-", 1)[0] for x in [*queue(REVIEW), *live.get(REVIEW, {}).values()]}
  rows = [x for x in review_rows(state["reviews"]) if x["key"].rsplit("-", 1)[0] not in pending]
  queue(REVIEW).extend(rows)
  return [x["key"] for x in rows]


def queue_rows(text: str, source: str, before: str = "") -> list[str]:
  taken = {str(x["key"]) for q in queues.values() for x in q} | {str(i["key"]) for held in live.values() for i in held.values()} | {k for _, k, _ in ended}
  put = []
  for line in text.splitlines():
    parts = [p.strip() for p in line.split("|")]
    if len(parts) < 5:
      continue
    key, t, model = parts[0].removeprefix("- ").strip(), parts[1], CLASSES.get(parts[2].split(":")[-1])
    if t in width and t not in (REVIEW, QA) and model and key not in taken:
      row = {"key": key, "model": model, "brief": f"A finding of {source}: {' | '.join(parts[4:])} Paths: {parts[3]}. Judge it against w/converge first, then fix it at its cause as one commit, with a test or a scenario that fails before the fix." + CLAIM}
      q = queue(t)
      at = next((i for i, x in enumerate(q) if str(x["key"]).startswith(before)), len(q)) if before else 0
      q.insert(at, row)
      taken.add(key)
      put.append(f"{t}:{key}")
  return put


def news(seconds: float = 1800.0, on: str = "") -> Act:
  def ear(id: str):
    yield "started", id
    bound = wait(seconds, id)
    while True:
      heard = yield {id, "prompt", bound}
      if heard[0] == "prompt" and scope(heard[1]) == ROOT:
        yield "done", id, str(heard[1])
        return
      if heard[0] == "done" and heard[1] == bound:
        yield "done", id, None
        return

  return act("news", on, controlled(ear), seconds)


def words() -> str:
  said = []
  for x in transcript(ROOT, "prompt"):
    if operated(str(x[2])):
      said.append(f"## {x[1]}, a message of the owner\n{x[5]}")
    elif str(x[6]) == OPERATOR and str(x[4]) != "None" and over(x[1]) and not isinstance(peek(x[1]), BaseException):
      said.append(f"## {x[1]}, his answer to a question of the lead: {str(x[5])[:400]}\n{peek(x[1])}")
    elif str(x[6]) == OPERATOR and str(x[4]) == "None":
      said.append(f"## {x[1]}, a message of the lead to the owner, made by {x[2]}\n{str(x[5])[:1500]}")
  write(Text(WORDS, "# The words of the owner in this life, and the messages of the lead to him, in order\n\n" + "\n\n".join(said) + "\n"))
  rows = "\n".join(f"- {t}: width {width[t]}; live {[str(i['key']) for i in live.get(t, {}).values()]}; queued {[str(x['key']) for x in queue(t)]}" for t in width)
  values = f"ROOM = {ROOM!r}\nchains = {chains!r}\nfixing = {fixing!r}\nfixwait = {fixwait!r}\nground = {ground!r}\nseats() = {seats()!r}\nstate = {state!r}"
  page = f"# The converge as it runs\n\nTopics, widths and queues:\n{rows}\n\nEnded items: {', '.join(k for _, k, _ in ended)}\n\n## Values bound now\n{values}\n\n## The conductor as it runs\n{dump(*PAGED)}\n"
  write(Text(PAGE, page))
  return f"{len(said)} entries, {len(page)} chars"
