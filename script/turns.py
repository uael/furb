"""Every turn of the suite, rendered by the crate and by the python World, held against each other.

What a model reads of a chain is the text of its turns, and two hosts make that text: `src/furb/world.py` in
python and `src/turn.rs` in the crate. They must make the same text of the same turn, word for word, or one
chain reads two ways and the cache of a provider holds across neither.

This runs the whole suite with the World doubles behind the plain boundary, keeps every turn that crossed, and
renders each of them twice: once by the two functions of the python World, taken from the file itself, and once
by the crate. A turn that reads differently is printed with both readings.

    uv run python script/turns.py

It spends nothing and asks no model.
"""

import ast
import builtins
import json
import os
import subprocess
import sys
from collections.abc import Callable
from pathlib import Path

HERE = Path(__file__).resolve().parent
"""HERE is the directory of the rig, which holds the plugin it runs pytest with."""
SRC = HERE.parent / "src"
"""SRC holds the preamble, the engine and the World the crate is held against."""

if str(SRC) not in sys.path:
  sys.path.insert(0, str(SRC))

from furb import engine  # noqa: E402
from preamble import outside, unwire, wire  # noqa: E402

NAMES = {**vars(builtins), **vars(engine)}
"""NAMES is what a plain value is made again by: the names of the interpreter and the names of the engine."""
KEPT = (
  Path(os.environ.get("FURB_TURNS", "")) if os.environ.get("FURB_TURNS") else HERE.parent / "target" / "turns.jsonl"
)
"""KEPT is where the turns of the run wait between the suite and the reading of them."""
WORDS = ("cargo", "run", "--quiet", "--example", "turns")
"""WORDS is how the crate is asked to render the turns it is given."""
SHOWN = 5
"""SHOWN is how many turns that read differently the rig prints, since one fault is often every fault."""


def worlds() -> Callable[..., object]:
  """The function of the python World that makes the text of a turn, taken from the file and nothing else.

  The module imports a provider this rig has no use for, so the rig reads the source of the two functions it
  needs and runs them alone. What it holds the crate against is the code that ships, and never a copy of it.
  """
  tree = ast.parse((SRC / "furb" / "world.py").read_text(encoding="utf-8"))
  wanted: list[ast.stmt] = [
    one for one in tree.body if isinstance(one, ast.FunctionDef) and one.name in ("shown", "rendered")
  ]
  held: dict[str, object] = {}
  exec(compile(ast.Module(body=wanted, type_ignores=[]), "world.py", "exec"), held)  # noqa: S102
  got = held["rendered"]
  assert callable(got)
  return got


def crossed(made):  # noqa: ANN001, ANN201
  """One World double behind the boundary, which keeps every turn that an ask carries."""

  def hears(self):  # noqa: ANN001, ANN202
    held = {"g": made(self)}
    held["g"].send(None)

    def host(_name: str, said: object) -> object:
      if isinstance(said, dict) and said.get("args") and said["args"][0] == "ask":
        with KEPT.open("a", encoding="utf-8") as file:
          for turn in said["args"][5]:
            file.write(json.dumps(turn) + "\n")
      got = unwire(said, NAMES)
      out = []
      try:
        back = held["g"].send(tuple(got) if isinstance(got, list) else got)
        while back is not None:
          out.append(wire(back))
          back = held["g"].send(None)
      except StopIteration:
        held["g"] = made(self)
        held["g"].send(None)
      return ("say", out)

    return outside("world", host, NAMES)

  return hears


def pytest_sessionstart(session: object) -> None:  # noqa: ARG001
  """Every World double of the harness, as the suite loaded it, put behind the boundary where it stands."""
  done = set()
  for mod in list(sys.modules.values()):
    for name in ("Sand", "Dead", "Where"):
      cls = getattr(mod, name, None)
      if isinstance(cls, type) and "hears" in cls.__dict__ and id(cls) not in done:
        done.add(id(cls))
        cls.hears = crossed(cls.__dict__["hears"])


def kept() -> list[str]:
  """Every turn the suite carried to a model, each as one line of the plain form, and each of them once."""
  KEPT.parent.mkdir(parents=True, exist_ok=True)
  KEPT.unlink(missing_ok=True)
  KEPT.touch()
  got = subprocess.run(
    [sys.executable, "-m", "pytest", "--no-cov", "-q", "-p", "turns", "test", "--ignore=test/outside"],
    cwd=HERE.parent,
    env={**os.environ, "PYTHONPATH": f"{HERE}:{SRC}", "FURB_TURNS": str(KEPT)},
    check=False,
    capture_output=True,
    text=True,
  )
  if got.returncode:
    sys.stderr.write(got.stdout + got.stderr)
    raise SystemExit(1)
  held = [one for one in KEPT.read_text(encoding="utf-8").split("\n") if one.strip()]
  said = list(dict.fromkeys(held))
  KEPT.write_text("\n".join(said) + "\n", encoding="utf-8")
  return said


def main() -> int:
  """Every turn, rendered twice, and the turns that read differently."""
  held = kept()
  world = worlds()
  got = subprocess.run([*WORDS, str(KEPT)], capture_output=True, text=True, cwd=HERE.parent, check=False)  # noqa: S603
  if got.returncode:
    sys.stderr.write(got.stderr)
    return 1
  crate = [json.loads(one) for one in got.stdout.split("\n") if one.strip()]
  if len(crate) != len(held):
    sys.stdout.write(f"the crate read {len(crate)} turns of the {len(held)} that crossed\n")
    return 1
  faults = 0
  for line, theirs in zip(held, crate, strict=True):
    turn = unwire(json.loads(line), NAMES)
    assert isinstance(turn, (list, tuple))
    mine = str(world(turn[1]))
    if mine != theirs:
      faults += 1
      if faults <= SHOWN:
        sys.stdout.write(f"the python World reads\n{mine!r}\nand the crate reads\n{theirs!r}\n\n")
  sys.stdout.write(f"{len(held) - faults} of {len(held)} turns read the same\n")
  return 1 if faults else 0


if __name__ == "__main__":
  sys.exit(main())
