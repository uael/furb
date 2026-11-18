"""LANE_GUARD, the check of a head before its push."""

from skills.conftest import Bench


async def test_lane_guard_is_empty_until_the_root_binds_one(laning: Bench) -> None:
  """LANE_GUARD is a shell command that lane_land runs in the tree on the head that it is about to push, empty until the root binds one, which a new load of the kit keeps."""
  assert laning.ns["LANE_GUARD"] == ""
  await laning.word('LANE_GUARD = "true"')
  await laning.word(laning.text)
  assert laning.ns["LANE_GUARD"] == "true"
