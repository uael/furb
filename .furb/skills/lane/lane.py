from furb.engine import (
  HIDDEN,
  Act,
  Refused,
  bash,
  cancel,
  chain,
  clock,
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
)

LANE_BASE: str = globals().setdefault("LANE_BASE", "main")
LANE_LOOK = 600.0
LANE_CHECKS = {
  "the title": 'python3 script/title.py "$(git log -1 --format=%s)"',
  "a dash in the prose": "! git diff BASE...HEAD -- . ':(exclude)tests/corpus' | grep -q '^+.*[—–]'",
  "the format of the Rust": "cargo fmt --all --check",
  "a comment past 100 columns": "python3 script/rustwidth.py BASE",
  "the walls of the engine and the doctrine": ".venv/bin/python script/tokens.py",
  "the hygiene laws": ".venv/bin/python -m pytest -q test/test_hygiene.py -p no:cacheprovider --no-cov",
}
lane_runners: dict[str, tuple[str, str]] = globals().setdefault("lane_runners", {})
lane_jobs: dict[str, tuple[str, float, float]] = globals().setdefault("lane_jobs", {})

LANE_RULES = """You take one item of the work on the branch <BASE> of furb, from A to Z: the investigation, the change, its proof, the commit and the push. No part of it goes to another chain, and it ends accepted or blocked with its cause, never partly.

The rules:
- Work in the tree <TREE> alone, and start with cd("<TREE>"). <STAND> Read its AGENTS.md first, and run skill("rust") before you change a Rust file.
- Fix each failure at its cause. Add no `#[allow]`, make no timeout longer, skip no test and weaken no assertion. When a test is wrong and the code is right, say why in your report.
- The owner defines done: the least code, the simplest and the most readable, that says the most coherent meaning. Add no second home for a truth and no second way to do a thing.
- Give each command a timeout of twice the time that it should take, and make a long command print its progress.
- A `git commit` and a `git push` each run the hooks, with `cargo test` among them, and each can take 20 minutes: give each a timeout of 2400 seconds, and run the push as its own command. Never commit or push with --no-verify.
- Commit with a title that `python3 script/title.py "<title>"` accepts, and a body that says why. Quote no word of a person in a commit.
<PUSH>
- Read the log of a job of CI with `gh api repos/uael/furb/actions/jobs/<id>/logs`, where <id> ends the URL of the job. `gh workflow run <file> --ref <branch>` starts a workflow by hand on a pushed branch.
- Change nothing outside your tree but $TMPDIR, where each script of your oracle takes a name that starts with a word of your item, since other items write there too. Never run `furb host upgrade`, and end no process by its name: the daemon of the owner runs his lives.
- Other lanes work in other trees of the clone at the same time. Run `bun run test` only when your item is of the TypeScript side, since two runs of it at once break each other. When git says `cannot lock ref`, run the command again after a few seconds.
- A word that makes one act where it could make many wastes a turn: make the reads and the searches of a step at once.
"""
LANE_OPENING = "\nChange no file of the tree yet. You may read, search and run probes, and you may write the scripts of your oracle in $TMPDIR. Close with a tuple of two strings: your reading of this item in your own words, with what you found that changes it, and one shell command, run in the tree, that fails now and passes only when the item is done."
LANE_PROBE = "Change no file of the tree. The acceptance command of this item passes on the head. Find what it does not hold: one case of the ask that the change gets wrong, on the path of the owner or at an edge of it. Write your probe under $TMPDIR, never in the tree, and run it yourself first. Close with one shell command, to run in the tree, that exits 0 when the item is right, that exits non-zero on this head for the reason that you mean, and that ends in less than 10 minutes. Close with None when you find no such case: a probe that asks for more than the ask is no case."
LANE_STEER = "\nThese are the measures of the lane now, and the items since the last time. This chain holds the rules of the owner and his rulings, and it read the contract of the kit of the lane, whose sentences hold what the lane learned. Read as he reads: which questions would he ask now, from these measures and from his rulings? Close with at most three rows of three strings: the question in his voice, the measure or the fact that makes him ask it, and the change that answers it. The change is a change of a function of the kit with the sentence of its contract that holds it, a new command of its ratchet, an item with its command, or a change of the measures themselves: one shell command that prints one line with the number that his question needs and that no measure gives yet, or the name of a measure that can answer none of his questions and so goes. A row whose change is a sentence of intent is no row. Close with an empty list when the measures give him nothing to ask."
LANE_NUDGE = "Your last word closed nothing and left no act open, so the chain asked you no more, and code sent this message. Go on from where you stood."
LANE_HUNG = "A reply of your model did not end in six looks of the lane, and code cancelled it. Go on from where you stood."


def lane_rules(tree: str, branch: str = "", lands: bool = True) -> str:
  base = LANE_BASE
  if not branch:
    stand = f"It is a worktree of furb whose head follows origin/{base}: run `git fetch origin` and `git merge --ff-only origin/{base}` before you change a file."
    push = f"- Push with `git push origin HEAD:{base}`, a fast-forward with no force. When it is refused because another lane pushed first, run `git pull --rebase origin {base}` and push again. When git refuses a push over SSH, AGENTS.md gives the push over HTTPS."
  else:
    stand = f"It is a worktree of furb on the branch {branch}. When it does not exist yet, make it first, from the checkout where your chain stands, with `sh script/worktree.sh {tree} {branch} origin/{base}`, then run the `uv sync` that it prints."
    push = f"- Push to your branch alone, with `git push origin HEAD:refs/heads/{branch}`. " + ("Code lands the branch on " + base + " when the item is accepted." if lands else "A probe is evidence, and nothing lands it.")
  return LANE_RULES.replace("<STAND>", stand).replace("<PUSH>", push).replace("<TREE>", tree).replace("<BASE>", base)


def lane_brief(ask: str, evidence: str, tree: str, branch: str = "", lands: bool = True) -> str:
  return lane_rules(tree, branch, lands) + "\nThe ask, in the words of the owner. They stay in this life: quote them in no commit, pull request or issue.\n" + ask + "\n\nThe evidence:\n" + evidence + "\n"


def lane_quiet(job: str) -> bool:
  return not paused(job) and not any(x[0] in ("reply", "run", "bash", "wait") and x is get(x[1]) and not over(x[1]) for x in transcript(scope(job)))


async def lane_settled(job: Act, seconds: float) -> object:
  left, words, same = seconds, -1, 0
  while left > 0:
    look = min(LANE_LOOK, left)
    if await wait(look, job) is not None:
      return peek(job)
    replies = transcript(scope(job), "reply")
    same = same + 1 if len(replies) == words else 0
    hung = [x[1] for x in replies if not over(x[1])]
    if same and lane_quiet(job):
      prompt(None, LANE_NUDGE, on=job)
    elif same >= 6 and hung:
      cancel(hung[0], "no end in six looks")
      prompt(None, LANE_HUNG, on=job)
      same = 0
    left, words = left - look, len(replies)
  return Refused(f"{job} did not close in {seconds} s")


async def lane_open(label: str, ask: str, evidence: str, tree: str, worker: str = "claude-cli:sonnet/high", branch: str = "", source: str = "", patience: float = 3600.0) -> tuple[str, str, str]:
  c = chain(label, source=source)
  got = await lane_settled(prompt(tuple[str, str], lane_brief(ask, evidence, tree, branch) + LANE_OPENING, to=worker, on=c), patience)
  if isinstance(got, tuple) and len(got) == 2:
    return str(c), str(got[0]), str(got[1])
  return str(c), "", repr(got)[:600]


async def lane_checks(tree: str, base: str) -> list[str]:
  runs = {name: bash("cd " + tree + " && " + check.replace("BASE", base), ok=True, show=HIDDEN, timeout=900.0) for name, check in LANE_CHECKS.items()}
  flaws: list[str] = []
  for name, run in runs.items():
    out = await run
    if out.code != 0:
      flaws.append(name + ": " + out.stdout.content[-400:])
  return flaws


async def lane_break(c: str, tree: str, breakers: list[str]) -> tuple[str, str]:
  for who in breakers:
    got = await lane_settled(prompt("str | None", LANE_PROBE, to=who, on=c), 2700.0)
    if isinstance(got, str) and got.strip():
      ran = await bash("cd " + tree + " && " + got, ok=True, show=HIDDEN, timeout=900.0)
      if ran.code != 0:
        return got, ran.stdout.content[-2000:]
  return "", ""


async def lane_tip(tree: str) -> str:
  return (await bash("git -C " + tree + " rev-parse HEAD", show=HIDDEN, timeout=30.0)).stdout.content.strip()


async def lane_work(c: str, accept: str, first: str, tree: str, worker: str = "claude-cli:sonnet/high", breakers: list[str] | None = None, patience: float = 7200.0, accept_s: float = 3600.0, begun: bool = False) -> tuple[str, str]:
  base = await lane_tip(tree)
  n = 0
  while n < 16:
    ran = await bash("cd " + tree + " && " + accept, ok=True, show=HIDDEN, timeout=accept_s)
    if ran.code == 0 and n == 0 and not begun:
      return "void", "the acceptance command passed before any work"
    before = await lane_tip(tree)
    if ran.code != 0:
      asked = (first if n == 0 else "") + f"\nThe acceptance command, which a word with no model runs in the tree after each close of yours, and whose exit code alone ends this item:\n{accept}\n\nIt exits {ran.code} now. Its last lines:\n{ran.stdout.content[-3000:]}\n\nDo the work from A to Z until it passes. Then close with one line for each thing that you found and that the lead cannot see in the tree, or with an empty line. When something blocks you, close at once with what blocks you."
    elif flaws := await lane_checks(tree, base):
      asked = "The acceptance command passes, and these checks of code fail. Fix each one at its cause, commit and push:\n" + "\n".join(flaws)
    else:
      probe, shown_tail = await lane_break(c, tree, breakers or ["claude-cli:opus/high", "claude-cli:fable/high"])
      if not probe:
        return "accepted", accept
      accept = accept + " && ( " + probe + " )"
      asked = f"A probe of another model fails on the head, and it is now a part of the acceptance command:\n{probe}\n\nIts last lines:\n{shown_tail}\n\nMake it pass at its cause, and keep its case as a test of the tree. When the probe asks for what the ask does not, change nothing and close with the reason."
    got = await lane_settled(prompt(str, asked, to=worker, on=c), patience)
    if isinstance(got, BaseException):
      return "blocked", repr(got)[:600]
    if await lane_tip(tree) == before:
      return "blocked", "the worker closed with no new commit: " + str(got)[:1500]
    n += 1
  return "blocked", "16 closes of the worker, and the acceptance command still fails"


async def lane_land(tree: str) -> tuple[bool, str]:
  tail = ""
  for _ in range(3):
    ran = await bash(f"cd {tree} && git fetch -q origin && {{ git rebase origin/{LANE_BASE} || {{ git rebase --abort; exit 9; }}; }} && git push origin HEAD:{LANE_BASE}", ok=True, show=HIDDEN, timeout=2700.0)
    tail = ran.stdout.content[-800:]
    if ran.code in (0, 9):
      return ran.code == 0, tail
  return False, tail


async def lane_steer(tree: str, ground: str, since: str, overdue: str = "", model: str = "claude-cli:fable/high") -> list[tuple[str, str, str]]:
  ran = await bash(f"cd {tree} && sh .furb/skills/lane/measures.sh . {LANE_BASE}", ok=True, show=HIDDEN, timeout=180.0)
  c = chain("steer", source=ground)
  rung('read("skills://lane/lane.pyi")', to=model, on=c)
  got = await lane_settled(prompt("list[tuple[str, str, str]]", ran.stdout.content + "open jobs past their time: " + (overdue or "none") + "\n\nThe items since the last time:\n" + since + LANE_STEER, to=model, on=c), 1800.0)
  return [(str(a), str(b), str(d)) for a, b, d in got] if isinstance(got, list) else []


def lane_run(label: str, word: str) -> Act:
  kit = read("skills://lane", HIDDEN).content
  runner, played = lane_runners.get(label, ("", ""))
  runner = runner or str(chain("run " + label))
  if played != kit:
    rung(kit, on=runner)
  lane_runners[label] = (runner, kit)
  return rung(word, on=runner)


def lane_job(label: str, job: Act, started: float, patience: float) -> Act:
  lane_jobs[label] = (str(job), started, patience)
  return job


def lane_overdue() -> list[str]:
  now = clock()
  return [f"{label}: {job} is open for {(now - started) / 60:.0f} minutes, past its {patience / 60:.0f}" + " (paused)" * paused(job) for label, (job, started, patience) in lane_jobs.items() if not over(job) and now - started > patience]


def lane_state(c: str) -> str:
  t = transcript(c)
  replies = [x[1] for x in t if x[0] == "reply"]
  still = {kind: [x[1] for x in t if x[0] == kind and x is get(x[1]) and not over(x[1])] for kind in ("prompt", "bash", "wait", "rung")}
  last = peek(replies[-1], "an open reply") if replies else "no reply"
  return f"{c} {get(c)[4]!r}: paused={paused(c)} open={still} replies={len(replies)} last word={str(last[1] if isinstance(last, tuple) else last)[-400:]!r}"
