"""Check that each workflow in .github/workflows/ parses and keeps its shape.

A bad merge of a workflow shows in CI only after minutes of runs. So this check parses each file with PyYAML and fails
on a syntax fault, on a key that a mapping holds twice (which a merge of two edits makes and YAML would take silently),
on a leftover marker of a conflict, on a job with neither `runs-on` nor `uses`, on a step with neither `run` nor `uses`,
and on a `needs` that names no job. It reads files alone, so it runs in a second, on a merge commit too.

Usage: python3 script/workflows.py
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parent.parent
MARKER = re.compile(r"^(<{7}|={7}|>{7})( |$)", re.MULTILINE)


class Loader(yaml.SafeLoader):
  """A loader that refuses a key that a mapping holds twice."""


def mapping(loader: Loader, node: yaml.MappingNode, *, deep: bool = False) -> dict:
  seen: set[object] = set()
  for key, _ in node.value:
    name = loader.construct_object(key, deep=True)
    if name in seen:
      why = f"line {key.start_mark.line + 1}: the key {name!r} stands twice in one mapping"
      raise yaml.YAMLError(why)
    seen.add(name)
  return yaml.SafeLoader.construct_mapping(loader, node, deep=deep)


Loader.add_constructor("tag:yaml.org,2002:map", mapping)


def faults(text: str) -> list[str]:
  """Each fault of one workflow, as a sentence."""
  if MARKER.search(text):
    return ["a marker of a merge conflict stands in the file"]
  try:
    flow = yaml.load(text, Loader)  # noqa: S506
  except yaml.YAMLError as why:
    return [" ".join(str(why).split())]
  jobs = flow.get("jobs") if isinstance(flow, dict) else None
  if not isinstance(jobs, dict) or not jobs:
    return ["the workflow holds no jobs"]
  found = []
  for name, job in jobs.items():
    if not isinstance(job, dict):
      found.append(f"job {name} is no mapping")
      continue
    if "runs-on" not in job and "uses" not in job:
      found.append(f"job {name} has neither runs-on nor uses")
    needs = job.get("needs", [])
    found += [
      f"job {name} needs {x}, which is no job" for x in ([needs] if isinstance(needs, str) else needs) if x not in jobs
    ]
    for number, step in enumerate(job.get("steps", []), 1):
      if not isinstance(step, dict) or not ({"run", "uses"} & step.keys()):
        found.append(f"step {number} of job {name} has neither run nor uses")
  return found


def main() -> int:
  failed = False
  for path in sorted((ROOT / ".github" / "workflows").glob("*.yml")):
    for fault in faults(path.read_text()):
      print(f"{path.relative_to(ROOT)}: {fault}")  # noqa: T201
      failed = True
  return 1 if failed else 0


if __name__ == "__main__":
  sys.exit(main())
