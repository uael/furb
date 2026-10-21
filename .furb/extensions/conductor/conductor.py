from furb.engine import HIDDEN, OPERATOR, acting, bash, chain, close, cwd, get, paused, peek, prompt, scope, transcript, wait, wake

CONDUCTING = """Conduct work in one loop, and plan no further than the next step.
Put each item in queue as (key, brief), then await conduct(): it hires a worker chain for each item, CAP at once, awaits the first that ends, and keeps its answer in results.
A worker does one item in a worktree of its own, runs only the tests that its change touches, opens its PR, and closes with its URL.
conduct() relays to you each question that a worker puts to the operator, wakes a worker that paused, and hires a worker for each open PR that conflicts or whose checks fail.
merge() hires a worker that merges each open PR whose checks pass.
debrief(name) asks the chain of a worker that ended for the frictions that it met in furb.
A review blocks only on a defect that a command reproduces, or on a false sentence of a contract."""

CAP = 5
ACTOR = "claude-cli:opus/medium"
queue: list[tuple[str, str]] = []
working: dict[str, str] = {}
results: dict[str, object] = {}
relays: dict[str, str] = {}
heads: set[str] = set()

WORKER = """You are a worker on the repository {repo}. Do one item, and open its PR yourself.
1. Make a worktree: git -C {repo} fetch -q origin main && git -C {repo} worktree add -q {repo}-w-{key} -b w/{key} origin/main, then work there.
2. Change it as the CLAUDE.md files of the repository say.
3. Run only the tests that your change touches.
4. Commit with a message in the style of git log, push the branch, and open the PR with gh pr create --base main.
5. Remove the worktree, and close with the URL of the PR alone.
Run no real life of a model. Ask the one who gave you this work on its chain: await prompt(str, question, on="{home}").

The item, {key}:
{what}"""

FIX = """The open PR #{n} conflicts with its base or has a failed check. Read it with gh pr view, gh pr checks and gh run view --log-failed. Rerun once a check that fails on one system alone. Else fix it in a worktree of its branch: resolve a conflict with the meaning of both sides, and for a stack read skill("gh-stack") and use gh stack sync. Run the tests that it touches, push with --force-with-lease, remove the worktree, and close with the URL."""

MERGE = """Merge each open PR whose checks pass, from the lowest number: gh pr merge N --squash --delete-branch, and for a stack read skill("gh-stack") and use gh stack merge <top> --yes --squash. Leave a draft, and a PR that conflicts or has a failed check. Close with one line for each PR: merged, or why not."""

DEBRIEF = """Your work is done. As a user of furb, list each friction, defect or confusion that you met in this chain, with its evidence and what would fix it. Close with a list of str, one for each, or an empty list."""

PRS = """gh pr list --state open --json number,mergeable,headRefOid,statusCheckRollup --jq '.[] | "\\(.number) \\(.mergeable) \\(.headRefOid) \\([.statusCheckRollup[]?.conclusion] | index("FAILURE") != null)"'"""


def hire(key: str, what: str, actor: str = ACTOR) -> str:
  brief = WORKER.replace("{repo}", cwd()).replace("{key}", key).replace("{home}", scope(acting())).replace("{what}", what)
  where = chain("w " + key)
  rung(f"actor = {actor!r}", on=where)
  name = str(prompt(str, brief, to=actor, on=where))
  working[name] = key
  return name


def merge() -> str:
  return hire("merge", MERGE)


def debrief(name: str) -> str:
  return str(prompt(list[str], DEBRIEF, on=str(get(name)[3])))


def tend() -> None:
  for orig, relay in list(relays.items()):
    if relay and peek(relay, ...) is not ...:
      close(peek(relay), id=orig)
      relays[orig] = ""
  for name in working:
    where = str(get(name)[3])
    if paused(where):
      wake(where)
    for x in transcript(where):
      if x[0] == "prompt" and x[6] == OPERATOR and x[1] not in relays and peek(x[1], ...) is ...:
        relays[x[1]] = str(prompt(x[4], f"{where} asks the operator:\n\n{x[5]}"))


async def conduct(every: float = 600) -> dict[str, object]:
  while queue or working:
    while queue and len(working) < CAP:
      hire(*queue.pop(0))
    await wait(every, *working, *[x for x in relays.values() if x])
    for name in [x for x in working if peek(x, ...) is not ...]:
      results[working.pop(name)] = peek(name)
    tend()
    listed = await bash(PRS, show=HIDDEN)
    for line in listed.stdout.content.splitlines():
      parts = line.split()
      if len(parts) == 4 and parts[2] not in heads and (parts[1] == "CONFLICTING" or parts[3] == "true"):
        heads.add(parts[2])
        hire("fix" + parts[0], FIX.replace("{n}", parts[0]))
  return results
