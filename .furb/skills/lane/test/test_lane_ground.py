"""LANE_GROUND, the rules of the owner that a row names."""

from skills.conftest import Bench


async def test_lane_ground_is_empty_until_the_root_binds_one(laning: Bench) -> None:
  """LANE_GROUND is a shell command that prints the rules of the owner whose ids it takes as arguments, empty until the root binds one, which a new load of the kit keeps."""
  assert laning.ns["LANE_GROUND"] == ""
  await laning.word('LANE_GROUND = "echo"')
  await laning.word(laning.text)
  assert laning.ns["LANE_GROUND"] == "echo"
