"""mcp, the doctrine of the extension."""

from pathlib import Path

from extensions.conftest import mcped
from furb import engine


async def test_mcp_is_the_doctrine_of_the_extension(tmp_path: Path) -> None:
  """mcp is the doctrine of the extension, a quote that says what the index tells and what each of mcp_tools, mcp_tool, mcp_use, mcp_call, mcp_read, mcp_prompt, mcp_servers and mcp_control does."""
  root = mcped(tmp_path, {})[1]
  doctrine = await engine.rung("close(mcp)", on=root)
  assert isinstance(doctrine, str)
  names = ["mcp_tools", "mcp_tool", "mcp_use", "mcp_call", "mcp_read", "mcp_prompt", "mcp_servers", "mcp_control"]
  assert [name for name in [*names, "index", "@server:uri"] if name not in doctrine] == []
