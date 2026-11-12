"""lane_land, the rebase and the push of a lane, with no model."""

import shlex

import pytest

from skills.conftest import Bench
from furb.engine import Refused

from ...conftest import gitted


def commit(where: object, name: str, text: str) -> None:
  (where / name).write_text(text, encoding="utf-8")
  gitted(where, "add", name)
  gitted(where, "commit", "-q", "-m", name)


async def test_lane_land_rebases_and_pushes_the_head(laning: Bench) -> None:
  """lane_land(tree, verdict) rebases the head of tree on origin/LANE_BASE and pushes it there, with no model, and gives True with the last lines of the push, and it refuses a verdict that is not accepted."""
  other = laning.tmp.parent / f"{laning.tmp.name}.other"
  gitted(laning.tmp.parent, "clone", "-q", str(laning.origin), str(other))
  commit(other, "theirs", "theirs\n")
  gitted(other, "push", "-q", "origin", "main")
  commit(laning.tmp, "ours", "ours\n")
  landed, _ = await laning.value(f"await lane_land({str(laning.tmp)!r}, \"accepted\")")
  assert landed is True
  assert gitted(laning.origin, "log", "--format=%s", "main").split() == ["ours", "theirs", "base"]
  with pytest.raises(Refused, match="lands only an accepted item, and its verdict is blocked"):
    await laning.word(f"await lane_land({str(laning.tmp)!r}, 'blocked')")


async def test_lane_land_aborts_a_conflict_and_tries_a_push_three_times(laning: Bench) -> None:
  """It aborts a rebase that conflicts and gives False, and it tries a push that fails again, three times in all."""
  other = laning.tmp.parent / f"{laning.tmp.name}.other"
  gitted(laning.tmp.parent, "clone", "-q", str(laning.origin), str(other))
  commit(other, "f", "theirs\n")
  gitted(other, "push", "-q", "origin", "main")
  commit(laning.tmp, "f", "ours\n")
  tree = str(laning.tmp)
  assert (await laning.value(f"await lane_land({tree!r}, \"accepted\")"))[0] is False
  assert not (laning.tmp / ".git" / "rebase-merge").exists()
  assert gitted(laning.tmp, "log", "--format=%s").split() == ["f", "base"]
  gitted(laning.tmp, "reset", "-q", "--hard", "origin/main")
  commit(laning.tmp, "g", "g\n")
  tries = laning.tmp.parent / f"{laning.tmp.name}.tries"
  hook = laning.origin / "hooks" / "pre-receive"
  hook.write_text(f"#!/bin/sh\necho try >> {shlex.quote(str(tries))}\nexit 1\n", encoding="utf-8")
  hook.chmod(0o755)
  assert (await laning.value(f"await lane_land({tree!r}, \"accepted\")"))[0] is False
  assert tries.read_text(encoding="utf-8").split() == ["try", "try", "try"]


async def test_lane_land_removes_a_clean_tree_and_its_branch(laning: Bench) -> None:
  """Once it lands a tree on the branch of its own, it removes the tree and the branch when the tree is clean and origin holds its head."""
  clean, dirty = (laning.tmp.parent / f"{laning.tmp.name}.{one}" for one in ("clean", "dirty"))
  for tree in (clean, dirty):
    gitted(laning.tmp, "worktree", "add", "-q", "-b", tree.name, str(tree))
    commit(tree, tree.name, "x\n")
  (dirty / "left").write_text("left\n", encoding="utf-8")
  for tree in (clean, dirty):
    assert (await laning.value(f"await lane_land({str(tree)!r}, 'accepted', {tree.name!r})"))[0] is True
  assert (clean.exists(), dirty.exists()) == (False, True)
  assert gitted(laning.tmp, "branch", "--list", "--format=%(refname:short)", clean.name, dirty.name).split() == [dirty.name]
