"""The boundary of the monty binding: the paths that carry a value across, which the suite over the engine does not
reach, since the suite drives the engine and these are the plumbing under it.

The suite rebinds a name that is the engine module, so this module reads the binding as ``furb_monty.engine.<name>``
through the package, which the rebinding does not reach.
"""

import math
from collections.abc import Generator

import pytest

import furb_monty.engine
from conftest import STANDS, Sand


def test_a_map_crosses_as_a_map_of_plain_values() -> None:
  """A map crosses to the engine and back as a map of plain values."""
  assert furb_monty.engine.wire({"a": 1, "b": [2, 3]}) == {"a": 1, "b": [2, 3]}
  assert furb_monty.engine.unwire({"a": 1, "b": [2, 3]}) == {"a": 1, "b": [2, 3]}


def test_a_value_that_is_no_plain_data_and_no_name_cannot_cross() -> None:
  """A value that is neither plain data, a shape, a callable nor a generator cannot cross to the engine."""
  with pytest.raises(TypeError, match="cannot cross"):
    furb_monty.engine.wire(object())


def test_a_name_that_is_no_shape_and_no_exception_is_refused() -> None:
  """A name the engine holds as neither a shape nor an exception this interpreter knows is refused."""
  with pytest.raises(TypeError, match="no shape and no exception"):
    furb_monty.engine.known("Nonesuch")


def test_a_float_that_is_not_finite_is_written_as_the_call_that_makes_it() -> None:
  """A float that is not finite is written as the call float(...) that makes it again where a word runs."""
  assert furb_monty.engine.written(math.inf) == "float(inf)"
  assert furb_monty.engine.written(-math.inf) == "float(-inf)"


def test_a_host_callable_that_raises_answers_the_sandbox_with_what_it_raised() -> None:
  """A callable the host handed a verb, called back from the sandbox, answers with what it raised, not a value."""
  life = furb_monty.engine.Living()

  def bad() -> object:
    raise ValueError("no")

  n = life.calls(bad)
  got = life.host(furb_monty.engine.CALL, [n, []])
  assert isinstance(got, dict) and "raised" in got


async def test_the_engine_of_monty_holds_its_kernel_and_refuses_one_given() -> None:
  """The engine of monty holds its Kernel, so a generator given under the name kernel is refused."""

  def g() -> Generator[tuple | None, tuple]:
    yield

  with pytest.raises(furb_monty.engine.python.Refused, match="kernel hears"):
    furb_monty.engine.boot(kernel=g())


async def test_a_map_of_the_life_names_itself_and_refuses_a_key_it_does_not_hold() -> None:
  """A map of the life read where it stands names itself, and refuses a key it does not hold."""
  assert repr(furb_monty.engine.Held("acts")) == "acts of the life"
  furb_monty.engine.boot((), world=Sand(stands=STANDS).hears())
  with pytest.raises(KeyError):
    _ = furb_monty.engine.Held("acts")["nowhere"]
  with pytest.raises(KeyError):
    _ = furb_monty.engine.Modules("modules")["nowhere"]
