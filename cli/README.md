# furb, the command line

`furb` is the command line of furb. With no command, it opens the window. With a command, it runs one life and prints
what it gave.

```sh
pip install furb                     # Gives the command furb. It needs Python 3.14 or later.
```

From a clone, `uv run furb` runs the `furb` of the clone, and `cargo run -p furb-cli --` builds it and runs it.

## The commands

| Command | What it does |
| --- | --- |
| `furb` | Open the window. It takes no word: its input opens a session in the project of its folder. |
| `furb prompt MESSAGE` | Ask the default actor, or the actor that `--to` names, and print its answer. `--shape` gives the type of the answer, `str` when unsaid. |
| `furb run WORD` | Run a word of Python on the root chain, and print what it gave to `close`. |
| `furb turns --record FILE` | Print the turns of the root chain of a record, as the model reads them. It replays the record in a life that only inspects it, and asks no model. |
| `furb facts --record FILE` | Print each fact of a record, after the number of its line. `--act` and `--chain` filter them. It opens no life. |
| `furb skills` | Print the skills that a life here runs, each with its life word. |
| `furb checkpoint --record FILE` | Write at the tip of a record the checkpoint of its life, from which a build of a later version goes on. It asks no model. |
| `furb catalog` | Print where the models of this machine came from. `--update` fetches them again now. |

These words go with each command that opens a life. `skills` and `checkpoint` take only `--record` and `--cwd`:

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

## Where furb finds the window

The window is a program of its own, `furb-window`. `furb` runs the one that `FURB_WINDOW` names, else the one beside
`furb`, else the one on PATH, else the one that the clone that `furb` was built from built. A `furb` that finds none
says so. The packages of PyPI and npm do not carry the window yet: build it from a clone with
`cargo build -p furb-window`, or start it with `cargo run -p furb-window`, as
[the window](https://github.com/uael/furb/blob/main/CONTRIBUTING.md#the-window) says. Its first build needs
[Zig](https://ziglang.org/download/) 0.17.1 on PATH.
