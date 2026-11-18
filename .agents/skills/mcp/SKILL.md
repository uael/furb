---
name: mcp
description: Connects a chain to the servers of the Model Context Protocol, and binds their tools as typed functions.
metadata:
  furb-word: mcp.py
  furb-life: mcp_index()
  furb-interface: mcp.pyi
  furb-process: process.py
  furb-acts: mcp_servers mcp_call mcp_read mcp_prompt mcp_control
  furb-config: keys.json
---
mcp tells each chain the servers and their tools, and binds the tools of each short server as a class. Its contract
is `mcp.pyi`.
