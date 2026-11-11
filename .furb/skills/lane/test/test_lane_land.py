"""lane_land, the rebase and the push of a lane, with no model."""

from skills.conftest import Bench

from ...conftest import gitted


def commit(where: object, name: str, text: str) -> None:
  (where / name).write_text(text, encoding="utf-8")
  gitted(where, "add", name)
  gitted(where, "commit", "-q", "-m", name)


async def test_lane_land_rebases_and_pushes_the_head(laning: Bench) -> None:
  """lane_land(tree) rebases the head of tree on origin/LANE_BASE and pushes it there, with no model, and gives True with the last lines of the push."""
  other = laning.tmp.parent / f"{laning.tmp.name}.other"
  gitted(laning.tmp.parent, "clone", "-q", str(laning.origin), str(other))
  commit(other, "theirs", "theirs\n")
  gitted(other, "push", "-q", "origin", "main")
  commit(laning.tmp, "ours", "ours\n")
  landed, _ = await laning.value(f"await lane_land({str(laning.tmp)!r})")
  assert landed is True
  assert gitted(laning.origin, "log", "--format=%s", "main").split() == ["ours", "theirs", "base"]


async def test_lane_land_aborts_a_conflict_and_tries_a_push_three_times(laning: Bench) -> None:
  """It aborts a rebase that conflicts and gives False, and it tries a push that fails again, three times in all."""
  other = laning.tmp.parent / f"{laning.tmp.name}.other"
  gitted(laning.tmp.parent, "clone", "-q", str(laning.origin), str(other))
  commit(other, "f", "theirs\n")
  gitted(other, "push", "-q", "origin", "main")
  commit(laning.tmp, "f", "ours\n")
  tree = str(laning.tmp)
  assert (await laning.value(f"await lane_land({tree!r})"))[0] is False
  assert not (laning.tmp / ".git" / "rebase-merge").exists()
  assert gitted(laning.tmp, "log", "--format=%s").split() == ["f", "base"]
  gitted(laning.tmp, "reset", "-q", "--hard", "origin/main")
  commit(laning.tmp, "g", "g\n")
  tries = laning.tmp.parent / f"{laning.tmp.name}.tries"
  hook = laning.origin / "hooks" / "pre-receive"
  hook.write_text(f"#!/bin/sh\necho try >> {tries}\nexit 1\n", encoding="utf-8")
  hook.chmod(0o755)
  assert (await laning.value(f"await lane_land({tree!r})"))[0] is False
  assert tries.read_text(encoding="utf-8").split() == ["try", "try", "try"]
