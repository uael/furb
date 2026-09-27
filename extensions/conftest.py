"""The harness of the engine, whose hooks and fixtures reach the suite of each extension only through a conftest above
it."""

from conftest import engine_of, pytest_generate_tests  # noqa: F401
