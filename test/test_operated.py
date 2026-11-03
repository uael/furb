"""operated, whether a site is of the operator."""

from conftest import born
from furb import engine


async def test_operated_tells_whether_a_site_is_the_operator_or_a_member_of_the_operator() -> None:
  """operated tells whether a site is the operator or a member of the operator: the site operator, or a site of the form operator/id."""
  born()
  assert [engine.operated(x) for x in ("operator", "operator/uael", "operators", "chain1", "x/operator", "")] == [
    True,
    True,
    False,
    False,
    False,
    False,
  ]
