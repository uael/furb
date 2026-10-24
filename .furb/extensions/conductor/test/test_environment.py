import os

import pytest


def test_no_test_of_the_conductor_sees_a_variable_of_a_commit(monkeypatch: pytest.MonkeyPatch) -> None:
  assert [name for name in os.environ if name.startswith("GIT_")] == []
