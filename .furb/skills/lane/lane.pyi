from furb.engine import Act

LANE_BASE: str
"""LANE_BASE is the branch that each lane works from and lands on, main until the root binds another, which a new load of the kit keeps."""

LANE_ITEMS: str
"""LANE_ITEMS is the file of the rows of the lane, .furb/lane.json, which git excludes, until the root binds another path, which a new load of the kit keeps."""

def lane_ask(id: str, ask: str, source: str, done: str = "", **row: object) -> None:
  """lane_ask(id, ask, source, done) keeps the ask verbatim as an open row of LANE_ITEMS, with its source, the command that says it is done and each other key of row, and refuses an id that a row holds.
  It refuses a worker above high with no why, since the effort follows the kind of the work: medium for work, high for a hard row, and more only with a reason.
  """

def lane_next(red: bool = False) -> str:
  """lane_next() gives the id of the hardest open row whose after list names only landed or void rows and whose tree and branch no running row holds, or an empty string.
  When red says that main is red, it gives only a row whose key main says that it fixes main.
  """

async def lane_settled(job: Act, seconds: float) -> object:
  """lane_settled(job, seconds) awaits job in looks of LANE_LOOK seconds, and gives its value once it is over, or a Refused once seconds pass.
  After a look with no new reply, it adds LANE_NUDGE to the thread of job and tells the operator in one line when the chain of job is not paused and holds no open reply, run, command or wait, since the engine asks a thread no more after a word that closes nothing and leaves no act open.
  After six looks with no new reply, it cancels the reply of the chain that does not end, adds LANE_HUNG to the thread of job, and tells the operator in one line.
  """

async def lane_open(id: str, evidence: str, tree: str, worker: str = "claude-cli:sonnet/medium", branch: str = "", source: str = "", patience: float = 3600.0) -> tuple[str, str, str]:
  """lane_open(id, evidence, tree) makes the chain id, asks worker there, under the rules of the lane and with the ask of the row id verbatim, for its reading of the item and one shell command that fails now, and gives the chain, the reading and the command.
  It gives the chain, an empty reading and the refusal when the worker gives no pair of strings within patience seconds.
  """

async def lane_work(c: str, accept: str, first: str, tree: str, worker: str = "claude-cli:sonnet/medium", breakers: list[str] | None = None, patience: float = 7200.0, accept_s: float = 3600.0, begun: bool = False) -> tuple[str, str]:
  """lane_work(c, accept, first, tree) runs accept in tree, asks worker on c with first to make it pass while it fails, and runs it again after each close, so a red command turns green.
  It gives void when accept passes before any work, unless begun says that the item stands in its middle.
  It gives blocked with the close of the worker when the head of tree did not move, since a close with no commit changes nothing that accept reads.
  Once accept passes, it runs each check of LANE_CHECKS in tree, with BASE put to the head where the item began, and asks worker to fix each check that fails.
  Once the checks pass, it asks each breaker, one breaker by default, on c for a probe, a shell command that fails on the head, and a probe that fails joins accept, which worker then makes pass.
  It refuses the probe of a breaker that changed the files or the head of tree, and asks the next breaker.
  It gives accepted with accept once no breaker gives a probe that fails, and blocked after 16 closes of worker.
  """

async def lane_land(tree: str, verdict: str, branch: str = "") -> tuple[bool, str]:
  """lane_land(tree, verdict) rebases the head of tree on origin/LANE_BASE and pushes it there, with no model, and gives True with the last lines of the push, and it refuses a verdict that is not accepted.
  It aborts a rebase that conflicts and gives False, and it tries a push that fails again, three times in all.
  Once it lands a tree on the branch of its own, it removes the tree and the branch when the tree is clean and origin holds its head.
  """

async def lane_item(id: str) -> tuple[str, str]:
  """lane_item(id) runs the row id from A to Z: lane_open on its tree, lane_work on its done command or else on the command of the worker, and lane_land once accepted, and it keeps the state, the cause, the done command and the time of the end in the row.
  It refuses a row whose tree or branch a row that is not over holds, and names that owner.
  Its worker is the one of the row, else sonnet at high for a hard row and at medium for any other.
  """

async def lane_fill() -> list[str]:
  """lane_fill() starts lane_item on the runner of each next row while fewer rows run than the key cap of the skill in the configs, and gives the ids that it started.
  It reads main as red when the last run of the gates on main failed.
  After three blocked rows in a row, it starts nothing and tells the operator once.
  It keeps script/memwatch.sh running while a row runs, and ends it when none runs.
  """

async def lane_steer(tree: str, ground: str, since: str, overdue: str = "", model: str = "claude-cli:fable/high") -> list[tuple[str, str, str]]:
  """lane_steer(tree, ground, since) runs measures.sh of the kit in tree, and asks model, on a chain that goes on from ground and reads the contract of the kit, for at most three rows: a question of the owner, the measure that makes him ask it, and the change of the kit that answers it.
  """

def lane_run(label: str, word: str) -> Act:
  """lane_run(label, word) runs word on the runner chain of label, which it makes once, with the LANE_BASE and the LANE_ITEMS of its caller, so the lanes of two labels run at once.
  It plays the word of the kit on the runner before, and again when the kit changed since, so a change of a function reaches each lane at its next word.
  """

def lane_job(label: str, job: Act, started: float, patience: float) -> Act:
  """lane_job(label, job, started, patience) keeps job as the job of label, and gives it.
  """

def lane_overdue() -> list[str]:
  """lane_overdue() names each job that is open past its patience by the clock, with its minutes, and says when it is paused.
  """

def lane_state(c: str) -> str:
  """lane_state(c) gives one line of the chain c: its label, whether it is paused and why, its open prompts, commands, waits and rungs, its replies and the end of its last word.
  """
