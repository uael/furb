# The official extensions

Each folder here is an extension that comes with furb and that each life runs, unless a config turns it off.
[Extensions](../docs/extensions.md) says what an extension is, how a life runs one, how to turn one off, and how to
write your own.

| Extension | What it gives a chain |
| --- | --- |
| [`catchup`](catchup) | What changed in the files that the chain read or wrote, told at each stand. |
| [`memory`](memory) | The `AGENTS.md` or `CLAUDE.md` files of the project that apply to the files that the chain reads. |
| [`skills`](skills) | The skills of the project and of the user, which `skills()` lists and `skill(name)` reads. |
| [`compaction`](compaction) | `compact()`, `condense()` and `compacted()`, which cut a chain in place to the work that it still needs. |
| [`ouroboros`](ouroboros) | How a model extends furb, and `extend()`, which writes an extension of the project. |
| [`web`](web) | `web_search(query)` and `web_fetch(url)`. |
| [`mcp`](mcp) | The tools, resources and prompts of the servers of MCP, as `await github.create_issue(...)`. |

Each folder holds these parts:

- `furb.json`, the manifest, which names the word, and the life word and the part for the TUI when the extension has
  them.
- `tui.ts`, the part for the TUI, in `compaction` and `mcp`.
- `<name>.py`, the word: the Python that each chain runs.
- `<name>.pyi`, the contract: the laws of the extension, one sentence on each line.
-  `test/`, the suite, with one test for each sentence of the contract. `uv run pytest -q extensions/<name> --no-cov`
  runs it.

The parts of an extension that reach the disk or the network are ears of the crate, in `src/extension/`.
