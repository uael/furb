import json
import re

from furb.engine import (
  HIDDEN,
  Exit,
  Refused,
  bash,
  clock,
  span,
  wait,
)

PULLING = """pr(title, why, outcome, checks) opens the pull request of the branch of your worktree, or updates it when it stands,
with the body that the template of the owner asks, and refuses one that breaks a rule of that template, with why.
prcheck(title, why, outcome, breaking) checks the title and the size of the body before you commit, and gives the
count of its words. shq(s) quotes s for sh.
checked(number) waits for the checks of a PR, and never in a loop of your own. It reads gh pr checks once a minute for
an hour at most, and gives a tuple: "pass", "fail", or "pending" when the bound ended, and lines. A line names the
head. For each failed check it gives [machine] when the log shows a fault of the runner or the network, [defect]
when it shows a fault of the code, [cancelled], or [unread], with its run and its job and its first failed lines. It
waits until no check pends and no run of the head is open. The jobs of a push or of a rerun are not all listed at once, and a check that fails first may be the only one that fails. Rerun a check only when it
is [machine] or [cancelled], once for each head, with gh run rerun <run> --failed, and then await checked(number)
again."""


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


async def pr(
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
) -> Exit:
  prcheck(title, why, outcome, breaking)
  if len(checks) != len(PR_BOXES) or not all(c.strip() and c.strip().lower() != "not:" for c in checks):
    raise Refused(f"checks needs {len(PR_BOXES)} lines, one for each box of the checklist in its order: the evidence of how you verified that box, or not: and the work that it still needs")
  body = pr_body(why, outcome, breaking or [], fixes, checks, part_of)
  name = shq(title)
  flags = "".join(f" --label {shq(x)}" for x in labels) if isinstance(labels, list) else labels
  flags = f" {flags.strip()}" if flags.strip() else ""
  names = labels if isinstance(labels, list) else re.findall(r"--label '?([^'\s]+)'?", labels)
  made = "".join(f"gh label create {shq(x)} --force >/dev/null 2>&1; " for x in names)
  rules = 'R=CLAUDE.md; [ -f AGENTS.md ] && R=AGENTS.md; sed "s/{rules}/$R/g" | '
  if number:
    command = rules + f"gh pr edit {number} --title {name} --body-file - >/dev/null && gh pr view {number} --json url --jq .url"
  else:
    opening = PR_OPEN.replace("@NAME@", name).replace("@BASE@", shq(base))
    opening = opening.replace("@ADDED@", flags.replace("--label", "--add-label")).replace("@FLAGS@", flags)
    command = (f"cd {shq(where)} || exit 1; " if where else "") + made + rules + opening
  got = await bash(command, stdin=body, show=span(-2, -1), timeout=180)
  if got.code != 0:
    said = " ".join((got.stderr.content + " " + got.stdout.content).split())[-500:]
    raise Refused(f"gh did not open or edit the PR, exit {got.code}: {said}")
  return got


CHECK_POLL = r"""for try in 1 2 3; do
  rows=$(gh pr checks "$n" --json name,bucket,link 2>/dev/null)
  case "$rows" in "["*) break ;; esac
  sleep 2
done
head=$(gh pr view "$n" --json headRefOid --jq .headRefOid 2>/dev/null)
echo "$head"
echo "$rows"
[ -n "$head" ] && gh run list --commit "$head" --json status,workflowName --limit 100 2>/dev/null
echo
"""

CHECK_LOG = r"""ok=
for try in 1 2 3; do
  if log=$(gh api "repos/{owner}/{repo}/actions/jobs/$id/logs" 2>&1); then ok=1; break; fi
  sleep 2
done
if [ -z "$ok" ]; then echo unread; printf '%s\n' "$log" | head -3 | cut -c1-300; exit 0; fi
if printf '%s\n' "$log" | grep -qE 'runner has received a shutdown signal|lost communication with the server|No space left on device|ECONNRESET|ETIMEDOUT|Could not resolve host|TLS handshake timeout|Connection reset by peer|runner is not responding|The operation was canceled'; then echo machine; else echo defect; fi
found=$(printf '%s\n' "$log" | grep -E 'FAILED|panicked|Error|error\[|AssertionError|\(fail\)|timed out|##\[error\]' | grep -v 'Process completed with exit code' | head -12 | cut -c1-300)
[ -n "$found" ] || found=$(printf '%s\n' "$log" | tail -45 | cut -c1-300)
printf '%s\n' "$found"
"""


async def checked(number: int, where: str = "", bound: float = 3600.0, every: float = 60.0) -> tuple[str, list[str]]:
  start = clock()
  enter = f"cd {shq(where)} || exit 1; " if where else ""
  while True:
    got = await bash(f"{enter}n={int(number)}\n{CHECK_POLL}", show=HIDDEN, timeout=180)
    head, _, text = got.stdout.content.partition("\n")
    found = []
    for line in [*text.splitlines(), "", ""][:2]:
      try:
        found.append(json.loads(line))
      except ValueError:
        found.append(None)
    rows, runs = (x if isinstance(x, list) else None for x in found)
    pending = [str(one["name"]) for one in rows or [] if one["bucket"] == "pending"]
    unfinished = [str(one["workflowName"]) for one in runs or [] if one["status"] != "completed"]
    if rows and runs is not None and not pending and not unfinished:
      break
    spent = clock() - start
    if spent + every > bound:
      seen = "no check has started"
      if pending:
        seen = ", ".join(pending)
      elif unfinished:
        seen = "the run of " + ", ".join(unfinished)
      elif rows:
        seen = "the runs are unread"
      return "pending", [f"head {head.strip()[:7]}: still pending after {int(spent)} s: {seen}"]
    await wait(every)
  failed = [one for one in rows if one["bucket"] in ("fail", "cancel")]
  lines = [f"head {head.strip()[:7]}: {len(rows)} checks, {len(failed)} failed"]
  for one in failed:
    name, hit = str(one["name"]), re.search(r"/runs/(\d+)/job/(\d+)", str(one["link"]))
    if one["bucket"] == "cancel" or not hit:
      lines.append(f"{name} [{'cancelled' if one['bucket'] == 'cancel' else 'unread'}] {one['link']}")
      continue
    log = await bash(f"{enter}id={hit[2]}\n{CHECK_LOG}", show=HIDDEN, timeout=180)
    kind, *found = log.stdout.content.splitlines() or ["unread"]
    lines.append(f"{name} [{kind}] run {hit[1]} job {hit[2]}")
    lines += [f"  {x}" for x in found]
  return ("fail" if failed else "pass"), lines


def shq(s: str) -> str:
  home = "~" if s == "~" else "~/" if s.startswith("~/") else ""
  return home + ("'" + s[len(home) :].replace("'", "'\\''") + "'" if s != "~" else "")
