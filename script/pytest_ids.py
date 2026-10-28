"""Keep the names of pytest cases from the reference run.

A case takes a reference name only when its test definition and parameter values match.
A missing or ambiguous match keeps the name that pytest gave it.
"""

from __future__ import annotations

import ast
import hashlib
import inspect
import json
import math
import os
import textwrap
import time
from collections import Counter
from pathlib import Path

import pytest


def value_of(value: object, active: set[int]) -> object:
  kind = type(value)
  if value is None or kind in (str, int, bool):
    return [kind.__name__, value]
  if isinstance(value, float) and kind is float and math.isfinite(value):
    return ["float", value.hex()]
  if isinstance(value, bytes) and kind is bytes:
    return ["bytes", value.hex()]
  if not isinstance(value, (list, tuple, dict)) or kind not in (list, tuple, dict) or id(value) in active:
    message = "This parameter has no stable value"
    raise TypeError(message)
  active.add(id(value))
  try:
    if isinstance(value, dict):
      return ["dict", [[value_of(key, active), value_of(part, active)] for key, part in value.items()]]
    return [kind.__name__, [value_of(part, active) for part in value]]
  finally:
    active.remove(id(value))


def identity(item: pytest.Item, definitions: dict[object, str]) -> str | None:
  if not isinstance(item, pytest.Function):
    return None
  call = getattr(item, "callspec", None)
  if call is None:
    return None
  try:
    params = [[name, value_of(value, set())] for name, value in sorted(call.params.items())]
    function = getattr(item.function, "__func__", item.function)
    if function not in definitions:
      definitions[function] = ast.dump(ast.parse(textwrap.dedent(inspect.getsource(function))))
    parent = item.nodeid.rsplit("::", 1)[0]
    name = getattr(item, "originalname", None) or item.name.split("[", 1)[0]
    data = json.dumps([parent, name, definitions[function], params], separators=(",", ":"))
    return hashlib.sha256(data.encode()).hexdigest()
  except (TypeError, ValueError, OSError, SyntaxError, RecursionError):
    return None


def captured(path: Path, groups: dict[str, list[pytest.Item]]) -> None:
  lock = path.with_suffix(".lock")
  path.parent.mkdir(parents=True, exist_ok=True)
  deadline = time.monotonic() + 10
  while True:
    try:
      lock.mkdir()
      break
    except FileExistsError:
      if time.monotonic() >= deadline:
        message = "The reference case map is held by another collector"
        raise TimeoutError(message) from None
      time.sleep(0.01)
  try:
    cases = json.loads(path.read_text()) if path.is_file() else {}
    for key, group in groups.items():
      cases[key] = sorted(set(cases.get(key, [])) | {item.nodeid for item in group})
    draft = path.with_suffix(".tmp")
    draft.write_text(json.dumps(cases))
    draft.replace(path)
  finally:
    lock.rmdir()


def pytest_collection_modifyitems(items: list[pytest.Item]) -> None:
  """Capture reference names, or restore an unambiguous name for the same definition and inputs."""
  path = Path(os.environ["FURB_PYTEST_CASES"])
  definitions: dict[object, str] = {}
  groups: dict[str, list[pytest.Item]] = {}
  for item in items:
    key = identity(item, definitions)
    if key is not None:
      groups.setdefault(key, []).append(item)
  if os.environ["FURB_PYTEST_CASE_MODE"] == "capture":
    captured(path, groups)
    return
  reference = json.loads(path.read_text())
  proposed = {
    group[0]: reference[key][0] for key, group in groups.items() if len(group) == 1 and len(reference.get(key, [])) == 1
  }
  names = Counter(proposed.get(item, item.nodeid) for item in items)
  for item, name in proposed.items():
    if names[name] == 1:
      # Reports read nodeid, while fixtures can read name, so both use the reference identity.
      item._nodeid = name
      item.name = name.rsplit("::", 1)[-1]
