"""Say which jobs of the gates a change needs, from the paths that it changes.

The jobs on Linux always run: they hold the whole proof of the logic. A job on macOS or on Windows proves the system
alone, and a runner of macOS waits in a queue for minutes, so a change runs those jobs only when one of its paths can
behave in another way there. The paths come one on each line from the standard input, and the answer is the lines
`key=value` that a step of GitHub takes into its outputs. A pull request and a push to main give their paths. With
`--all`, which a run by hand and the run of each night take, every job runs. A change to the files of the gallery alone
runs no hook, which the output hooks says.
"""

import json
import re
import sys

LINUX = ["ubuntu-latest"]
EVERY = ["ubuntu-latest", "macos-latest", "windows-latest"]

# A path that no job needs on another system: prose, the tools of the repository, and the workflows that hold their
# own proof.
INERT = re.compile(
  r"(.*\.md|docs/(?!examples/).*|LICENSE|\.gitignore|\.furb/(?!extensions/|config\.json$).*|\.agents/.*|\.claude/.*|script/.*|reserve/.*"
  r"|\.github/(ISSUE_TEMPLATE/.*|budgets\.json|workflows/(publish|release|titles|contract)\.yml)"
  r"|release-please-config\.json|\.release-please-manifest\.json)"
)
# A path that only the suite of python reads.
PYTHON = re.compile(r"(test/.*|bind/python/.*|pyproject\.toml|uv\.lock|docs/examples/python/.*)")
# A path that only the bindings of TypeScript and the TUI read.
TYPESCRIPT = re.compile(
  r"((tui|bind/typescript)/.*|package\.json|bun\.lock|biome\.json|tsconfig.*\.json|docs/examples/typescript/.*)"
)
# A path that the suite of python reads on Linux alone: the extensions of the project and the config that enables
# them, whose suite in .furb/extensions gates each word as a chain plays it.
PROJECT = re.compile(r"\.furb/(extensions/.*|config\.json)")
# A path of pure logic that every job reads, but that behaves the same on each system: the engine and the code that
# monty runs beside it, whose sandbox gives them no access to the machine, and the corpus of the records. The World
# alone speaks to the machine. A change of these runs every job on Linux alone. In 36 hours of main, 19 of 125 commits
# touched them, and 6 of them touched no other path that runs the jobs on macOS and on Windows.
LOGIC = re.compile(r"src/furb/(engine\.pyi?|sheet\.py|kernel\.py)|tests/corpus/.*")
# A path that the crate builds, which every job reads.
RUST = re.compile(
  r"((src|cli|tests|extensions|\.cargo)/.*|Cargo\.(toml|lock)|build\.rs|rust-toolchain\.toml|docs/examples/rust/.*)"
)
# A render of the gallery changes only these files, which no gate reads, so it runs no gate, by the ruling of the owner
# on 10-29. The job of the gates alone reports on it.
GALLERY = re.compile(r"docs/(screenshots/.*|furb[.]gif)")


def scope(paths: list[str]) -> dict[str, object]:
  """The systems of the suite and of the bindings, and whether the crate on Windows and on macOS runs.

  The hooks run unless the change is a render of the gallery alone.
  """
  kinds = {"python": False, "typescript": False, "rust": False, "other": False, "project": False, "logic": False}
  for path in paths:
    if INERT.fullmatch(path):
      continue
    if LOGIC.fullmatch(path):
      kinds["logic"] = True
    elif PROJECT.fullmatch(path):
      kinds["project"] = True
    elif PYTHON.fullmatch(path):
      kinds["python"] = True
    elif TYPESCRIPT.fullmatch(path):
      kinds["typescript"] = True
    elif RUST.fullmatch(path):
      kinds["rust"] = True
    else:
      kinds["other"] = True
  gallery = bool(paths) and all(GALLERY.fullmatch(path) for path in paths)
  every = kinds["rust"] or kinds["other"]
  return {
    "suite_os": EVERY if every or kinds["python"] else LINUX,
    "ts_os": EVERY if every or kinds["typescript"] else LINUX,
    "crate": every,
    "code": any(kinds.values()),
    "hooks": not gallery,
  }


def lines(answer: dict[str, object]) -> str:
  """The answer as the lines of an output of a step."""
  return "".join(f"{key}={json.dumps(value, separators=(',', ':'))}\n" for key, value in answer.items())


def main() -> None:
  # A run that is no pull request has no change to read, and a path that no rule names runs every job.
  paths = ["every"] if "--all" in sys.argv[1:] else [line.strip() for line in sys.stdin if line.strip()]
  sys.stdout.write(lines(scope(paths)))


if __name__ == "__main__":
  main()
