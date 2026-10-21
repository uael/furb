<div align="center">

# furb

**The model answers in Python. You read and steer each word it runs.**

[![gates](https://github.com/uael/furb/actions/workflows/gates.yml/badge.svg)](https://github.com/uael/furb/actions/workflows/gates.yml)

<img src="docs/furb.gif" alt="The furb TUI: a message, the Python that the model writes, a Python word of the operator, the three views, the command palette, the rewind tree, and a change of theme" width="880">

</div>

## What makes furb different

Most harnesses give a model a menu of tools and read one JSON call at a time. furb gives the model a Python
module, and each answer of the model is a program that runs in it.

- **One answer, one program.** In one word, the model can read files, run commands, ask other models, wait,
  and combine the results with loops and conditions. It does not have to spend one turn on each call.
- **The system prompt is one file that you can read.** The engine is `src/furb/engine.py`. Minified, it is the
  whole system prompt, and it costs fewer than 6000 tokens. No text is hidden from you.
- **Every word is checked before it runs.** A type checker reads each word first. A word that it refuses never
  runs, so it has no effects, and the model reads why.
- **Work runs side by side.** A command, a question to a model, and a wait are acts that a word can await
  together. Chains work in parallel, and each chain has its own module and folder.
- **A scratch directory for each session.** A word keeps its notes and scratch files in `scratch://`, and a command
  finds them in `$TMPDIR`. A resumed session finds them again, and the record makes again the files that its words
  wrote.
- **Rewind to any point.** A new chain can start from any point of another chain. In the TUI, press Escape twice
  to open the rewind tree.
- **You see what the model sees.** The transcript is the exact text that the model reads. The record holds every
  act of the operator and every answer of the World, and a later run replays it. Work that was interrupted
  starts again when you resume it.
- **Limits that hold.** A chain pauses when it reaches a dollar ceiling or a share of the context window.
- **No permissions, by design.** Each act runs with the rights of the user who runs furb, and the record shows who
  made each act. For isolation, run furb in a virtual machine. An extension can
  [ask you before an act](docs/extensions.md#permissions), but furb has none built in.
- **The model can ask you.** A question to the operator waits in the feed until you answer it.
- **Extensions are words.** An extension is Python that each chain runs as a rung, which the model reads in its turns.
  The official ones tell a chain the `CLAUDE.md` files of its project and the skills it can read. The system prompt
  stays the engine alone, so an extension never breaks what a provider caches.
- **One contract, proven twice.** `src/furb/engine.pyi` holds each law of the engine as one sentence, and the
  suite has one test for each sentence. The suite runs on CPython, and on monty, a Python interpreter written in
  Rust.

## Try it

```sh
pip install furb                     # The engine, and furb, the command line. It needs Python 3.14 or later.
```

`furb` is a program of Rust. It runs the engine in monty. It asks the models of the claude command line, the models of
each provider whose credential, such as `ANTHROPIC_API_KEY`, stands in the environment, and the models of the local
server whose address `FURB_LOCAL_URL` names:

```sh
furb                                 # The TUI, when furb finds one.
furb --mode rpc                      # A JSON-RPC on stdin and stdout, with no TUI.
furb prompt "How many lines does a.txt hold?" --shape int
furb run "close(1 + 1)"              # A Python word, run on the root chain.
furb turns --record session.jsonl    # The turns of a life made again from its record, which it only reads.
furb extensions                      # The extensions that a life runs here.
```

A local server serves the chat completions of OpenAI under `/v1` and lists its models at `/v1/models`, as llama.cpp, LM
Studio, Ollama, vLLM and Strata do. `FURB_LOCAL_KEY` gives its key when it takes one. furb names each model for chat
that the server lists `local:` and its id, as in `--model local:qwen3`. Such a model takes no effort, costs nothing, and
writes a quarter of its window at most in one turn, or the number that `FURB_LOCAL_TOKENS` gives. It thinks 8192 tokens
at most in one turn, or the number that `FURB_LOCAL_THOUGHT` gives, and 0 sets no most: past it, furb asks the model
again with that thought and with its thought turned off, so it writes its word at once. Its window is the one
that the server gives, or the number that `FURB_LOCAL_WINDOW` gives, and furb offers no model whose window neither
gives. Ollama serves a model in the `num_ctx` of its parameters, or in `OLLAMA_CONTEXT_LENGTH`: set it to 32768 or more,
since the system prompt of furb is about 9000 tokens. Ollama tells that window only for a model that it loaded, so give
the same number to `FURB_LOCAL_WINDOW`. furb asks the server only for a name of the provider `local` or of a provider
that furb does not know, as `local:qwen3` or `qwen3:8b`, when a host lists the models, as the TUI and the scripts do,
and when no other model stands for the default actor. `local:` and the id is the sure name, as
`local:mistral:7b`, since `mistral:7b` names the provider `mistral`. A list that the server gave holds for the process,
but for a model that it did not list or gave no window of, which furb asks for again after 30 seconds, as it asks
again a server that answered nothing. When furb finds
no claude command line, the default actor is a model of a provider of the network whose credential stands, and then a
model of the local server.

The TUI runs from a clone of this repository. It needs uv, the Rust toolchain, and Bun 1.4.2 or later:

```sh
uv sync                              # The Python environment, with furb, which the build reads.
bun install && bun run build         # The TUI, and the engine that it runs through the crate.
bun run demo                         # A scripted life on a sample project. It asks no model.
bun run tui                          # A real life, through the claude command line.
```

Then `uv run furb` opens the TUI of the clone, and so does a `furb` built from it. A `furb` from anywhere else runs the
program that `FURB_TUI` names, or the `furb-tui` on PATH.

## Learn more

- [The examples](docs/examples/README.md): short programs in python, TypeScript and Rust that show a durable life, a
  tree of chains, a prompt to the operator, a grant and a door.
- [The TUI](tui/README.md): the views, the keys, the commands, and [a gallery of each screen](docs/tui.md).
- [The JSON-RPC](docs/rpc.md): the commands and the events of `furb --mode rpc`.
- [The extensions](docs/extensions.md): what an extension is, the configs that turn one on, and the official ones.
- [The developer guide](docs/developer-guide.md): the tools to install, how a word runs, and how to change the TUI.
- [The contract](src/furb/engine.pyi): every law of the engine, one sentence per line.

## License

> Copyright (C) 2026 Abel Lucas. furb is free software under the GNU Affero General Public License, version 3,
> which `LICENSE` holds. You may use, study, change, and share it. Anyone who ships it, or who runs a changed furb
> as a service, must offer the source under the same terms. The engine carries no notice of its own, because its
> text is the system prompt of a model and every word of it counts.
