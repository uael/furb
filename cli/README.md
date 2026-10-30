# furb, the command line

`furb` is the command line of furb. With no command, it opens the TUI. With a command, it runs one life and prints
what it gave. With `--mode rpc`, it serves one life to another program.

```sh
pip install furb                     # Gives the command furb. It needs Python 3.14 or later.
```

From a clone, `uv run furb` runs the `furb` of the clone, and `cargo run -p furb-cli --` builds it and runs it.

## The commands

| Command | What it does |
| --- | --- |
| `furb` | Open the TUI. `--demo` opens it on a life that asks no model. |
| `furb prompt MESSAGE` | Ask the default actor, or the actor that `--to` names, and print its answer. `--shape` gives the type of the answer, `str` when unsaid. |
| `furb run WORD` | Run a word of Python on the root chain, and print what it gave to `close`. |
| `furb turns --record FILE` | Print the turns of the root chain of a record, as the model reads them. It replays the record in a life that only inspects it, and asks no model. |
| `furb facts --record FILE` | Print each fact of a record, after the number of its line. `--act` and `--chain` filter them. It opens no life. |
| `furb extensions` | Print the extensions that a life here runs, each with its life word. |
| `furb checkpoint --record FILE` | Write at the tip of a record the checkpoint of its life, from which a build of a later version goes on. It asks no model. |
| `furb catalog` | Print where the models of this machine came from. `--update` fetches them again now. |
| `furb --mode rpc` | Serve one life on stdin and stdout, as [the JSON-RPC](https://github.com/uael/furb/blob/main/docs/rpc.md) says. |

These words go with each command that opens a life. `extensions` and `checkpoint` take only `--record` and `--cwd`:

- `--record FILE` keeps the life in a record, and goes on from it when it holds a life already. A name that ends in
  `.sqlite`, `.sqlite3` or `.db` makes a database of SQLite. With no record, the life keeps nothing.
- `--cwd DIR` is the directory where the chains start, the current directory when unsaid.
- `--model ACTOR` and `--roster MODEL` choose the models, as
  [the page of models](https://github.com/uael/furb/blob/main/docs/models.md) says.

`furb --help`, and `furb <command> --help`, give every word.

## Examples

```sh
furb run "close(1 + 1)"                                       # Prints 2, and asks no model.
furb prompt "How many files?" --shape int --to operator      # Asks you, and prints the number you type.
furb prompt "How many lines does README.md hold?" --shape int # Asks the default model.
furb prompt "Fix the failing test." --record fix.jsonl        # Keeps the life, so a second run goes on from it.
furb turns --record fix.jsonl                                 # Shows what the model read.
```

## Where furb finds the TUI

The TUI is a program of its own. `furb` runs the one that `FURB_TUI` names, else `furb-tui` on PATH, else the TUI of
the clone that `furb` was built from. A `furb` that finds none says so. The TUI runs from a clone, as
[the TUI](https://github.com/uael/furb/blob/main/tui/README.md) says. The words after `--` go to the TUI, as
`furb -- --view loom`.
