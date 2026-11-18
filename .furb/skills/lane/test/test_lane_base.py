"""LANE_BASE, the branch of the lanes."""

from skills.conftest import Bench


async def test_lane_base_is_main_until_the_root_binds_another(laning: Bench) -> None:
  """LANE_BASE is the branch that each lane works from and lands on, main until the root binds another, which a new load of the kit keeps."""
  assert laning.ns["LANE_BASE"] == "main"
  await laning.word('LANE_BASE = "w/other"')
  await laning.word(laning.text)
  assert laning.ns["LANE_BASE"] == "w/other"
