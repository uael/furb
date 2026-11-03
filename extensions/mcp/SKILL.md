---
name: mcp
description: Connects a chain to the servers of the Model Context Protocol, and binds their tools as typed functions.
metadata:
  furb-word: mcp.py
  furb-life: await mcp_index()
  furb-interface: mcp.pyi
  furb-view: tui.ts
---
mcp tells each chain the servers and their tools, and binds the tools of each short server as a class. Its contract
is `mcp.pyi`.
