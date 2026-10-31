# furb

The engine is `src/furb/engine.py`, one file. It depends only on the Python interpreter and on the ears of the outside
that boot is given: the ears of the World, the Kernel and the gate. The contract, `src/furb/engine.pyi`, defines it, and
the suite in `test/` proves it. `src/furb/AGENTS.md` holds the technical names of the engine and the laws that no test
can hold. `DOCTRINE.md` tells a model how to work in furb, and the entries doctrine and setup of `src/furb/AGENTS.md`
say how a life reads it. `script/AGENTS.md` says how to run the DeepSWE rig. Claude Code reads each `AGENTS.md` through
a `CLAUDE.md` beside it that holds only `@AGENTS.md`, and the skills of `.agents/skills` through `.claude/skills`, a
link to that folder.

The crate at the root, `furb`, runs the same file in monty, a python interpreter written in rust, behind an async
API of its own:

- `src/lib.rs` says what the crate gives: `Engine`, whose methods are the verbs of the contract, which `build.rs`
  makes from the contract, and `Ear`, the one trait of an ear, which a host writes as a generator of rust with `ear`.
- `src/world/` holds the ears of the World that the crate writes, one file for each concern: the files, the commands,
  the time, the store of the record and the store of builds, the log of a life, the reader of a record with no life, the
  scratch directory, the rules by which a console reads the operator, and the provider of models, which asks each model
  through rig and streams what it writes. The doc comments of `src/world.rs` and of each module say what each does, the
  tables of [`CONTRIBUTING.md`](CONTRIBUTING.md#the-world-and-the-life) list its files, and `docs/models.md` tells a
  user how furb finds a model and how to choose one.
- `src/extension.rs` says what an extension is and where a host finds the ones that a life runs. Its ear `extensions`
  enables each by a fact that the record keeps, plays its word as a rung on each chain where its manifest says, every
  chain or the root alone, and plays the setup of the root on a new record. `parts` gives the part of each extension for
  the TUI, which `tui/src/parts.ts` loads. `extensions/` holds the official extensions, `catchup`, `memory`, `skills`,
  `compaction`, `ouroboros`, `web` and `mcp`, each a word, a contract and a suite, and `src/extension/` holds the ear of
  the World of each that has one, as `compaction.rs` and `mcp.rs`. `docs/extensions.md` says how an extension works.
- `src/life.rs` opens a life as every host opens one, from one `Opening`: the record, the ears of the crate after the
  ears of the host, the default actor, the roster that the host names or every model of the catalog, and the extensions.
  A host adds only its own ears, as the console of its operator. A life on a record of another version is refused before
  any replay, and `tests/corpus` holds the records that each build must replay with no drift. Each door gives it as
  `opened` and as `Engine.open`.
- `src/checkpoint.rs` holds the checkpoint of a life: a fact at the tip of its record with each chain that lives, its
  turns, its module and its open prompts, from which a later life of any build goes on. `furb checkpoint` writes one.
- `src/snapshot.rs` holds the snapshot of a life beside its record: the session of the sandbox that monty dumps and
  what the engine holds of the ears, which a later life on the same record and build opens from with no replay.
- `src/engine.rs` opens a life in the sandbox: it loads the engine, the sheet and the Kernel, each in a namespace of its
  own, and carries each value between the sandbox and a host, whose ears the engine steps as generators. An ear that
  raises refuses the act that it heard, and starts again from a fresh ear when it can, as the Kernel, the gate and each
  ear that `lasting` in `src/ear.rs` made do; the life tells the operator once which ear raised and why. A fault that no
  code of the sandbox catches, as the bound of the memory in `src/sand.rs`, cuts the run with no `finally`, so the life
  stops: it keeps no snapshot, refuses each later entry with that fault, and gives it to a host as `stopped`.
- The Kernel is `src/furb/kernel.py`, one module that the sandbox loads and the python package imports, each on the
  names of its engine. The gate, `src/gate.rs`, is the type checker of monty. It reads a word on the sheet of the
  engine, `src/furb/sheet.py`, against the typeshed of the sandbox, and it runs each import of the word in a sandbox
  of its own, which alone says which module runs. The gate of the python package reads through it too.
- `src/binding.rs` holds the host API that both doors give, each function once: the ears of the crate, the store, the
  opening of a life, the catalog, the extensions, the images and the gate. A door holds only how a value of its
  language crosses, its `Engine` and its `Act`, and the loop that drives them. The `Engine` of each door has the same
  methods, one for each verb of the contract, which the build makes from the contract.
- `src/binding/py.rs`, behind the `python` feature, is the door to python. `bind/python` is the package `furb-monty`,
  whose module `furb_monty.engine` gives every name of the contract over one life in the sandbox, and
  `FURB_ENGINE=monty` makes `from furb import engine` give it.
- `cli/` is `furb-cli`, the second package of the workspace of the crate, whose program is `furb`, on clap. With no
  command it hands the terminal to the TUI, which it finds in `FURB_TUI`, as `furb-tui` on PATH, or in the checkout it
  was built from. `--mode rpc` serves one life as a JSON-RPC on stdin and stdout, which `docs/rpc.md` says. `prompt`,
  `turns`, `run` and `extensions` each open one life on a record, `facts` prints each fact of a record with no life,
  `checkpoint` writes the checkpoint of a life, and `catalog` shows where the models came from. The package `furb`
  depends on the wheel of `furb-cli`, so `pip install furb` gives the command `furb`.

The suite runs on both engines, but for the tests that `MONTY_SKIPS` in `test/conftest.py` names: these boot on a Kernel
of this interpreter, and the engine of monty holds its own. The suite of each extension, `extensions/<name>/test/`, runs
on both engines too, and its conftest, `extensions/conftest.py`, hands it the hooks of the harness and the helpers that
only the suites of the extensions need. `test/outside/test_monty.py` proves what the door carries that no sentence of
the contract says.

The TypeScript side is a bun workspace at the root, whose packages are the three below and the examples of
`docs/examples`:

- `bind/typescript` is the crate through N-API. Its views, its queries and its controls are synchronous, and an
  act that is answered later can be awaited. It includes a `Session`: an engine on the ears of the crate, the
  provider among them, and on a console of the operator. `bind/typescript/README.md` says how to use it. Its
  model of a life, `furb/life` in `src/life`, gives the views of a life that every host draws, with no native
  code. The TUI imports its build in `bind/typescript/dist`, which `bun run build` makes again.
- `tui` is the OpenTUI application on that package. `tui/README.md` says what the TUI does, and `docs/tui.md` shows
  each screen. A life of the TUI runs in the worker of `tui/src/worker.ts`, which `tui/src/host.ts`, the host of the
  life, holds in a process of its own behind a socket that `tui/src/hosting.ts` places, so the life outlives each
  window: `tui/src/bridge.ts` attaches a window to the host, starts it, or takes over a host of another build, and keeps
  the worker in the window where no socket serves. The host serves the life over the network too, to a window of
  another operator that holds the key of a role, to read and write or to read only, which `tui/README.md` says.
- `gui` is the desktop app, which runs a life in a window as the TUI runs one in a terminal: its core in
  `gui/src-tauri`, a Cargo workspace of its own on Tauri 2, holds the window and starts the host of a life in
  `gui/host`, a program of bun on the `Workspaces` and the `Session` of the TUI, and its UI in `gui/src` reads the
  model of a life from its source. `gui/README.md` says how to build it, start it and use it, and `gui/DESIGN.md`
  says its design.

`docs/examples/` holds the examples of the framework: five short programs for each door, in `python/`,
`typescript/` and `rust/`, which a user runs as they stand, and which `docs/examples/README.md` shows. They stand on
a life as a program holds it, which each door gives with the same names: `src/furb/life.py`, which `furb.open`
gives, `bind/typescript/src/framework.ts`, and `src/life/host.rs`. `test/outside/test_examples.py`,
`bind/typescript/test/examples.test.ts` and `tests/examples.rs` run each example as a process, on a claude command
line that answers from a script.

`CONTRIBUTING.md` is the guide for a person who changes the repository: the tools to install, how to set up a clone,
how a word runs, the files of each part, the gates, and how a pull request lands. For everything else, it links to
this file. `.github/pull_request_template.md` holds the rules of a pull request, and `.github/ISSUE_TEMPLATE/` the
forms of an issue: a defect and a request. `CHANGELOG.md` holds the releases, which release-please writes above the
history of furb before 0.2.0.

## The contract

`src/furb/engine.pyi` gives the typed surface of the engine, and the docstring of each definition holds the laws of
that definition, one sentence per line, in Simplified Technical English.

The contract is a safeguard. It serves the gate and the suite, and it keeps the engine from drift. It is not the
source of truth: the owner is. A request of the owner changes the contract. Change each signature and each sentence
that the request makes wrong, in the same work, and name each change. Never refuse a request, and never work around
it, to keep a sentence of the contract as it is. The contract gives matter for questions: when a request breaks one
of its laws, name that law, and ask or challenge the request before the work, then do what the owner decides.

Read it whole before you do any work. Do not change it on your own initiative. When you find a hole or a
contradiction in it, say so, with the change that fixes it.

## The meanings

The owner is the source of truth of what a word means, and a name serves its meaning strictly. When a name of the
engine does not serve its meaning, the name is wrong, and the name changes.

- A fact is inert: a piece of information dropped into the life, with no response, which each ear that waits for it
  hears in the order of the log.
- A question is an act: it takes a name when it is said, it goes to the ears in turn until one owns it, and what it
  came to the life holds under that name. A fact with an answer makes no sense: the answer belongs to the question.
- An act is answered now, or later, and it is awaited for its answer. The ear that owns it says so: a done for an
  act it answers now, or a started now and a done later. A query is only an act that no ear started.
- started or done is ownership, and the owner of an act answers it. The life refuses an act that no ear owns.
- A view reads the life, and makes nothing and keeps nothing: get, peek and transcript, and all that is derived
  from them.
- No hidden control flow and no magic, which is a hard rule of the owner: a model can rebuild the whole flow of what it
  read from the exact code that ran. Each change of the module of a chain comes from a word that the transcript of that
  chain shows as a turn, and the transcript is the whole of what ran: no run is made on the side, and no control flow
  is hidden. furb hosts itself with its own verbs. Only the Kernel and the World stand outside `src/furb/engine.py`,
  and the seeding of a chain, each turn that furb crafts for it, lives in `engine.py` alone. The test
  `each_run_of_a_chain_of_the_corpus_has_a_turn_that_shows_its_word` in `src/life.test.rs` holds the first part.
- The record is the one truth of a life. A cache of a life is derived from its record and checked against it, and a
  reader drops a cache that does not agree: the log of a life holds every fact that the life said, and each later
  life writes it again from the record. A record is made again by running its words, or loaded from the snapshot
  that a run of those words made on the same record, which the life checks, so a chain stays a function of its
  record.

## The bar

You are not here to please the owner, and not here to reach a green gate as fast as possible. You are here to reach
perfection, whatever the effort and the time. The engine is the system prompt of a model, and each word and each
control flow in it changes the odds of a good or a bad response, so a flaw in it costs before any inference. When
two shapes both pass every gate, the one that serves the meaning of every word is the one, and a shape that is
green and incoherent is not done.

Read the whole of `src/furb/engine.py` and list each incoherence: a truth with two homes, a word with two meanings, a
part held by the wrong owner. Probe each, build its reshape, and measure its size and the suite. Keep what adds
coherence, then read it again.

## Questions of the owner

A question of the owner about a state or a delay, as "Why is it not built yet?", "Is it fine?" or "Why is this chain
paused?", most often points at a defect of the agent that he asks, and he wants that defect fixed durably. Read it as
that request. Fix the state at once, find the cause in your own work, make its durable fix at once (a change of the
instructions, of a tool or of a test, in a pull request), read the state again to verify, and only then answer, with
the fix and its proof. An answer that only explains is not done.

## The suite

The suite drives the engine through its public API alone, end to end, from the module the operator gives.

- `test/conftest.py` is the harness. `Sand` is a World in memory: files by path, scripted words by chain id, the calls
  it performed, the entries it kept, and what it fed its commands. It stands on `STANDS`, and a World of one test
  overrides its `hear` for the facts it does otherwise. `Dead` refuses every question but the standing, and `Where`
  asks the chain where it stands at every path. `Py` is the Kernel of `furb.kernel` on the engine of this
  interpreter, with the gate of the crate, which refuses a word that is not python or that names what nothing binds,
  such as `BAD`. `life` boots a life on them, `born` boots a life on the World of the suite, which holds one file,
  with the words its models answer the root with, `settle` gives the loop room, `plain` sends a record through the
  wire and back, `said` reads the facts, and `paragraphs` and `heads` read the turns, which are python.
- One test file per definition of the contract: `test_<name>.py` for a function, a global or a type alias,
  `test_<class>_<method>.py` for a method, in lower case, with dunder underscores stripped. A capitalized definition
  whose lower-case name is another definition's, `Bash` beside `bash`, has `test_<name>_shape.py`.
- One test per sentence. The docstring of a test is exactly one sentence of the contract, and nothing else.
- A test arranges the World, acts through `engine.<verb>(...)` with `from furb import engine`, and asserts exact
  values: names, entries of `sand.record`, results through `engine.peek`, turns through `engine.turns(on=root)`,
  and the calls of the doubles. It reads a fact by position, as the contract declares it, and never weakens an
  assertion to pass: a sentence the engine fails stays red until the engine, or the sentence, is right.
- A test of speed compares two times that it measures in one run, and takes the least of several runs of each. A bound
  in seconds fails on a machine under load.
- A helper that only one file needs lives in that file. A helper that several files need lives in
  `test/conftest.py`, and one that only the suites of the extensions need lives in `extensions/conftest.py`.
- `test/outside/` holds the tests of the World, the Kernel, the door, the switch of the package that
  `FURB_ENGINE` sets, and the harness. These tests stand outside the hygiene laws.

## The hygiene laws

`test/test_hygiene.py` holds each contract and its suite to these laws, and it must stay green. The contracts are
`engine.pyi`, whose suite is `test/`, and the contract of each extension, `extensions/<name>/<name>.pyi`, whose suite
is the folder `test` beside it.

1. Every sentence of a definition has exactly one test, in the file of that definition, whose docstring is that
   sentence. A constructor and a property are no definitions of their own: their sentences are the class's.
2. Every test carries a sentence of the contract.
3. Every definition has a file of its own and at least one sentence. Two definitions never share a file.
4. The module docstring of each contract is empty.
5. `engine.py`, minified with its program kept as it is, costs fewer than 7200 tokens of `o200k_base`, the
   tokenizer of OpenAI, and the doctrine of furb, `DOCTRINE.md`, costs at most 1000. The test counts with `o200k_base`
   because it is local and the same on each machine: the tokenizer of Claude is not public, and only the network
   counts with it. The wall is a proxy for the cost to Claude, which reads about 1.5 tokens for each token of
   `o200k_base` in the engine.
6. The system prompt parses to the same program as `engine.py` on disk.
7. No name in `engine.py` is bound again beneath a scope that already binds it: a parameter, a local, a loop
   target or an import never takes the spelling of a name of an enclosing function or of the module. Every word
   keeps one meaning.
8. The word of an official extension parses to the program of its file less the imports of furb.
9. The word of an official extension binds no name of the engine, and, read after the engine as one module, binds
   no name again beneath a scope that already binds it.

## Commands

Run every command from the root of the repository. Before it runs a command, `uv run` syncs the environment. The
sync builds a package of the workspace again when the environment does not hold it, as in a new checkout, or when a
file that its build reads changed, which the `cache-keys` of its `pyproject.toml` name: the crate, the engine and the
python that the crate carries, and the extensions. So a change of `src/furb/engine.py` reaches the suite at the next
`uv run`, after a build of a few seconds, and a short command can take minutes. To run with no sync, which builds
nothing, use `uv run --no-sync`, or run the tools in `.venv/bin`.
A worktree holds no `.venv`: run one test file there with the `.venv/bin/python` of the main checkout, as
`<main>/.venv/bin/python -m pytest -q <file> -p no:cacheprovider --no-cov`, which syncs nothing.

Set the timeout of each command, and the bound of each wait, to about twice the time that the step should take, never a
round 600 or 900 seconds in case: a ceiling that is too large hides a hang for as long. Make a step that can take
minutes print its progress, so that a hang shows at once.

Search the code with `git grep`, which reads the tracked files alone, and never with `grep -r` or `rg` on the root. A
worktree holds `target/` and `node_modules/`, and a walk of them takes minutes and ends at the timeout. To search an
ignored or an untracked file, give `grep -r` the path of a folder, as `src` or `docs`.

Never end a process by its name or by a pattern of its command line: not `pkill`, not `killall`, not `pkill -f`. A
pattern matches the windows of the owner, the processes of other workers and your own shell, whose command line holds
the pattern. Start a process that you may need to end with its pid written to `$TMPDIR`, or in a process group of its
own, and end that pid or that group alone.

Make a small change, run the checks that it touches, and compare a check that fails with the base before the change:
fix what your change broke, and report what was broken before.

- `uv sync`: install the environment, which builds the crate with its `python` feature into the package `furb-monty`,
  and the command line into the package `furb-cli`, and builds them again after a change that their `cache-keys` name.
  `uv sync --reinstall-package furb-monty --reinstall-package furb-cli` builds them again in any case. This build,
  `cargo test` and `bun run build` take the dev profile, which optimizes every dependency, the interpreter among them,
  so a word runs as fast as in a release while the crates of the workspace build quickly. A wheel for PyPI takes the
  release.
- `script/builds.sh <version>`: build the command line at the last commit of `origin/main` that holds a version, and
  put it in the store of builds of the machine, so a record of that version opens with `furb` again.
- `script/worktree.sh <path> <branch> [base]`: make a git worktree whose build starts warm. It checks out a branch that
  stands on origin or in this clone, and moves a local branch that lags behind origin up to it; a new branch starts from
  `base`, `origin/main` by default. It copies on write the build directories of the checkout whose build stands nearest
  to the new commit, so cargo builds only the crates of the workspace again: the `target` of cargo, the `node_modules`
  of bun, the build of the TypeScript package and the cache of the tests of the TUI. Then it runs
  `bun install --offline` and `bun run build`. It does not copy `.venv`: run the `uv sync` that it prints, and before
  that, probe with the `.venv/bin/python` of the main checkout. A first build takes about 6 minutes, against about 35 in
  a worktree of `git worktree add`. Copy on write needs APFS, Btrfs or XFS; on another file system, the script makes a
  plain worktree. Remove a worktree with `git worktree remove --force <path>`, then `git branch -D <branch>`. The
  checkout at the root can stand behind `origin/main`, so read and change code in a worktree.
- `uv run pytest -q`: the suite on both engines, with the coverage of `furb` and of `furb_monty`, which must be
  whole.
- `uv run pytest -q test/test_hygiene.py`: the hygiene laws alone.
  `addopts` in `pyproject.toml` holds `-n auto`, so `-p no:xdist` fails with an unrecognized argument. Run the
  suite in one process with `-n0`, as `uv run pytest -q -n0 <path>`.
- `uv run python script/tokens.py`: the cost of the minified engine and of `DOCTRINE.md` in tokens of `o200k_base`,
  against their walls, from the code of law 5 and in seconds. It builds nothing: it counts the newest minified engine
  that is the program of `engine.py`, from the installed `furb_monty` or a `system.py` in `target`, and when none is, it
  names `cargo check` to make one.
- `uv run ruff format src test script extensions` then `uv run ruff check src test script extensions`: format and
  lint. Two spaces of indentation, 120 columns.
- `.furb` is outside the formatter, even for a path that you give it: its words keep the form of the engine. Ruff
  lints it for what breaks a word, which is a name that nothing binds, a bug of the family B and an error of the family
  PLE, and for no style. A word imports each name of the engine that it uses from `furb.engine`, which the crate leaves
  out of the word. A name of a word before it in `.furb/config.json` is in `builtins` of `[tool.ruff]`, which
  `.furb/extensions/test_words.py` holds to a definition. Run the tests of an extension of the project alone with
  `uv run --no-sync pytest -q <folder> -p no:cacheprovider --no-cov`, since the coverage of 100 applies to the whole
  suite only.
- `uv run ty check --error-on-warning`: the type check. The tests are checked against `engine.pyi`, and the words of
  `.furb/extensions` too, less their tests.
- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace`:
  the gates of the crate and of the command line. The tests of a module stand beside it, `src/engine.test.rs` beside
  `src/engine.rs`, and those of the engine drive the real engine on ears in rust. `cli/tests/furb.rs` runs `furb` as
  a process, on a claude command line that answers from a script.
- `cargo run -p furb-cli -- <words>`, or `uv run furb <words>`: the command line, as `furb --help` says.
- `uv run pre-commit run --all-files`: every gate the commit hook runs. A `git commit` runs the hooks, `cargo test`
  among them, and takes minutes: give it a timeout in `bash` of twice the time that it took last. The hook
  `script/hooks/pre-commit`, which `git config core.hooksPath script/hooks` turns on, prints only what fails. Run the
  push as its own command after the commit, so that its result stands in the tail of that command. The hook `cargo test` runs only when a file that the crate
  or its tests read changed, and the gates run every hook on every file. The hook of the links runs only for a README or
  a manifest, and reads git alone. The hook `stray` refuses a `*.rej`, `*.orig` or `*-E` file. The hook `paths` runs on
  each commit, since a move of a file leaves its old path in a script that the commit does not touch.
- `uv run python script/smoke.py`: one real life on the default actor of the crate, the first model that the catalog
  offers, which may cost dollars. `--model` names the actor, as `--model local:qwen3` on the local server that
  `FURB_LOCAL_URL` names. It is no test of the suite and spends one prompt.
- `uv run python script/play.py`: one real life that uses every part of the runtime, and a second life on its
  record.
- `uv run python script/deepswe.py`: the DeepSWE rig, which `script/AGENTS.md` says how to run.
- `uv run python script/bench/onelife.py`: the proof of one life. It builds the command line three times, with the
  engine changed twice in a way that makes the same acts and the last build of the next version, and runs one record
  over the three builds on a claude that answers from a script: the record goes on with no drift, reloads an extension,
  compacts a chain in place and crosses the version through a checkpoint, and no chain is lost, no name is taken twice
  and no command or turn runs twice. Its docstring says how to run it.
- `uv run python script/bench/replay.py <task> --out <directory>`: the bench of replay. One life of the local model
  works a task of DeepSWE, later lives open on its record, one more opens after a change of the World, and Claude
  Code on the same model works the task as the baseline. Its docstring says how to run it.
- `python script/bench/audit.py <record> <sha256>`: the audit of a record. It checks the sha256 of the record, replays
  it in a life that only inspects it, with no model and no network, and checks that each word and each answer hashes
  as the record says. It imports furb alone, so an outsider runs it on `pip install furb`, as `docs/audit.md` says.
- `uv run python script/bench/inert.py <record>`: the proof that a replay is inert. It replays a copy of the record
  in a life that only inspects it, and fails when, before the tip, the life puts an act on to the World that the
  record answers, an ear of the World says that it started an act, or the journal keeps a fact.
- `python script/bench/loop.py make|furb|furb-own|furb-batch|claude|score <dir>`: the bench of a loop, which its
  docstring says how to run. It makes support tickets from a seed with a known label each, triages them on the same
  local server with a word of furb that asks one prompt per ticket, with a life of furb whose model writes its own
  words, with a word of furb that asks one prompt per batch, and with the claude command line, and scores each arm on
  the totals of the server and the share of right labels. It asks a model and spends what the server serves.
- `script/real.py` is no command: it holds what `smoke.py`, `play.py` and `deepswe.py` share, which is the life they
  open on the engine of this interpreter, the actor they ask, and what the answers of a record cost.
- `uv run python script/catalog.py`: make the snapshot of the catalog, `src/world/provider/catalog.json`, again from
  models.dev and from the data of pi-ai. A life updates its own copy of models.dev and of the providers of
  pi-ai in the cache of furb once a day, and the newer of the cache and the snapshot holds for each provider.
  `furb catalog [--update]` shows where the models came from and updates them at once; a source that fails leaves the
  last valid copy, and a life that stands again reads the caches again.
- `uv run python script/links.py`: check that each link in a README that a package carries works on the page of the
  package on PyPI, npm or crates.io, which read no relative link. It lists each package and its README from the
  manifests, and fails on a relative link, which `--fix` writes as absolute, at `main` on GitHub. The hooks run it.
- `uv run python script/paths.py`: check that each path of the repository that a file of `script/` or of
  `.github/workflows/` names is a file or a folder that git holds, or a file that a build makes and git ignores. It
  names the file, the line and the path of each one that stands no more, and it reads git alone. The hooks run it.
- `bun install && bun run build`: install the TypeScript workspace, and build the N-API package that the TUI uses.
- `bun run check`, `bun run lint` and `bun run test`: the type check, the lint and the tests of the TypeScript side.
  `tui/script/test.ts` deals the tests into runs, each a process that runs one test at a time with a limit of thirty
  seconds, and runs twice as many at once as the machine has cores. A word after `bun run test` filters the files by
  their path or by `package/file`, as `tui/host`, and a filter that matches no file fails. Use it, and not a plain
  `bun test <file>`, which keeps `claude` on the PATH and fails where a test names a model, and says so; a bunfig.toml
  in the root and in each package gives a plain run the limit of thirty seconds and the folders of the runner.
  `FURB_TEST_OUTPUT=1` shows all that each run wrote. The tests read the built package, so a build older than its
  sources, as after a switch of branch, fails them as they fail on main: the runner warns before the run and after a
  failure, and `bun run build` makes the build again. The tests read the configs and the cache of `.furb/tests` of the
  root, from any folder, not those of the machine. As root, the test of a folder that cannot be read fails, since
  root reads every folder.
  A script of bun in `$TMPDIR` cannot import the package `furb`, so write a quick check in the worktree, where
  `node_modules` stands.
- `bun run demo` and `bun run tui`: the TUI on the demo session, which asks no model, or on a real life.
- `bun run docs`: write the tables of keys and commands in `tui/README.md` again from `tui/src/keys.ts` and
  `tui/src/commands.ts`.
- `bun run bench`: the speed of the TUI on two recorded lives of about 2.6 KB for each fact, one of about 160000 facts
  and one of a tenth of that: the cost of a new fact, the first frame, the frames while the feed scrolls and a command
  streams, and the memory on Linux. It fails over its budgets, which the docstring of `tui/script/bench.ts` gives in paces of the
  machine, and which the gates hold on Linux. `FURB_BENCH_JSON` names a file that takes the measures of each run, and
  `FURB_BENCH_FACTS` sets the size of the long life.
- `bun run screenshots` and `bun run animation`: draw `docs/screenshots/` and `docs/furb.gif` again from the real
  renderer. Run both after a change that a screen shows, and read each capture. Do not commit them: after a push to
  `main` that changes a path that a capture reads, `.github/workflows/gallery.yml` draws both again on a runner of CI
  and keeps one pull request open on the branch `w/gallery`. One run at a time holds its home in `/tmp`, and
  `FURB_GALLERY_ROOT` names another folder for it; `tui/script/stage.ts` says how long a run waits.
  `FURB_GALLERY_ONLY=<pattern>` writes only the captures whose names match, `FURB_GALLERY_TEXT=<folder>` writes the text
  of each capture, which a model that cannot see an image reads, `FURB_GALLERY_CELLS=<folder>` writes the cells of each
  capture as JSON, which the extension `converge-tui` of the project reads, and `FURB_STILL_CLOCK` sets the time of the
  view, which stands still by default. A model sees a capture as an image when its word reads the PNG.
- `python script/budgets.py`: set the budget of time of each job of the gates in `.github/budgets.json`, from its last
  green runs on `main`. The round of the conductor, an extension of the project in `.furb/extensions/conductor`, queues
  an item for a job that passes its budget.

Every pull request and every push to `main` runs the gates in `.github/workflows/gates.yml`: the hooks, which
include the gates of the crate and the type check. The suite runs on Linux, on macOS and on Windows, and the gates
of the crate run on Windows and on macOS too. The gates of the TypeScript side run on Linux, on macOS, and on Windows.

A release is one version of every package: `@uael/furb` on npm, which a user installs as
`npm install furb@npm:@uael/furb`, with the engine of each system as `@uael/furb-<system>`, and `furb`, `furb-monty` and
`furb-cli` on PyPI. `release-please-config.json` names each file that holds the version. `.github/workflows/release.yml`
keeps the pull request of the next release open on `main`, with `CHANGELOG.md`, and its job `daily` merges it each day
when its gates and the gates of `main` passed. The merge tags the release, and `.github/workflows/publish.yml` publishes
it from the tag, once it proved each package from its artifacts in a clean environment. Below 1.0.0, a fix or a feat
raises the patch version and a breaking change the minor version, as `bump-patch-for-minor-pre-major` and
`bump-minor-pre-major` say. Only the owner raises the major version, with
`gh workflow run release.yml -f version=1.0.0`; from 1.0.0, release-please raises the minor version for a feat and the
major version for a breaking change. crates.io holds only the empty crates `furb` and `furb-cli` 0.0.0, which keep the
names: it takes no crate that stands on git, and `furb` stands on the fork of monty by git, so the job of crates.io
publishes nothing while a dependency stands on git, and says why.

- `sh script/prove-crate.sh [folder]`: pack the crates of monty at the revision of the fork that `Cargo.toml` names,
  and `furb` and `furb-cli` on them by their version, as patches of crates.io, then add `furb` to a new crate that
  takes every other crate from crates.io with no lock of furb, run the example chains there, and install `furb-cli`.
  It proves that the crates pack and build as crates.io will give them once monty stands there. A crate that breaks
  the build in a fresh resolve takes a pin with `=` in `Cargo.toml`, as salsa does. It takes about 5 minutes.
- `sh script/prove-npm.sh <packs>`: install the packs of `@uael/furb` and of `@uael/furb-<system>` for this system, as
  `npm pack` names them, in a new folder, and run the example chains there with node.

## Prose

Write all prose in Simplified Technical English, for a general non-technical audience. The rules are ASD-STE100, at
<https://www.asd-ste100.org/>. They hold for a chat message, a commit message, a pull request, a review comment, a code
comment and a document.

Prose here uses no em dash and no en dash. A docstring is a document.

## Pull requests

Read `.github/pull_request_template.md` whole and apply it strictly.

It is the one home of the rules of a pull request: its scope, its title, which release-please reads, and its body. The
check `the title`, in `.github/workflows/titles.yml`, runs `script/title.py` on each title, at each change of the pull
request and at each edit of its title, and reads the types from their one home, the `changelog-sections` of
`release-please-config.json`.

## Comments

A comment says what the code cannot tell by itself. It is a disclaimer: the code does not behave as a reader would
assume, and one sentence spares them a debugging session. Documentation adds edges to symbols and enumerates the
invariants; a public type of furb always gets it, because a client developer reads it: a type of the contract
`src/furb/engine.pyi`, and a type of a door, the crate in `src/lib.rs`, `bind/python` or `bind/typescript`. The code,
its names, the module tree and `git log` say the rest.

A comment older than your change is not yours to remove. `TODO` and `FIXME` go with the change that completes the work
they name. Write for the reader a year out: state what is true, never what changed. Fill each line to column 120, the
width that the formatters keep here, then wrap; a docstring of a contract keeps one sentence per line.
