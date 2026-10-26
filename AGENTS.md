# furb

The engine is `src/furb/engine.py`, one file. It depends only on the python interpreter and on the ears of the outside
that boot is given, each a generator under a name: the ears of the World, the Kernel and the gate among them. It is
derived from the contract, `src/furb/engine.pyi`, and the suite in `test/` proves it. `src/furb/AGENTS.md` holds the
technical names of the engine and the laws that no test can hold. `DOCTRINE.md` is the doctrine of furb, which says
how a model works in furb: every model reads it once on each chain, in a paragraph that the chain tells just before
its first prompt to that model.
`script/AGENTS.md` says how to run the DeepSWE rig. Claude Code reads each `AGENTS.md` through a
`CLAUDE.md` beside it that holds only `@AGENTS.md`, and the skills of `.agents/skills` through `.claude/skills`, a
link to that folder.

The crate at the root, `furb`, runs the same file in monty, a python interpreter written in rust, behind an async
API of its own:

- `src/lib.rs` says what the crate gives: `Engine`, whose methods are the verbs of the contract, which `build.rs`
  makes from the contract, and `Ear`, the one trait of an ear, which a host writes as a generator of rust with `ear`.
- `src/world/` holds the ears of the World that the crate writes, one file each: the files in `files.rs`, the
  commands in `bash.rs`, time in `time.rs`, the store of builds in `builds.rs`, which holds the program of the command line of each version
  that ran on the machine, the store of the record in `store.rs`, as a file of lines or a database
  of SQLite, each a `Store` that the suite in `store.test.rs` holds to its laws, whose ear names in the record the
  engine that made it, the log of a life beside its record
  in `log.rs`, the reader of a record with no life in `reader.rs`, which opens a record of any engine to read and keeps what it tells
  in a cache beside the record, `<record>.told`, and the provider of models in `provider.rs`, which asks each model through rig and streams what it writes, and asks it again with a backoff after a failure that passes, as a lost connection, and at the reset of a usage limit of the claude command line, which it tells the chain. It reads the statements of the word that a model streams, as the model finishes them, and ends the reply at the first that cannot be python, with the word as far as that statement, which `provider/watch.rs` holds. Its catalog, `provider/catalog.rs`,
  knows the models of every provider that pi-ai serves and those of the claude command line, which
  `provider/claude/discover.rs` asks of the program itself and keeps in the cache until the program changes or `/reload`, and it offers each
  model whose credential stands in the environment, the claude command line when it finds the program, and each
  model for chat that a local server lists when `FURB_LOCAL_URL` names its address, which `provider/local.rs` asks.
  `docs/models.md` tells a user how furb finds a model and how to choose one.
  It says too what a first run checks of each: `setup` in `src/binding.rs` gives it to a host.
  `provider/search.rs` holds the native search of the web of each provider that has one, which the extension `web`
  asks.
  `operator.rs` holds the rules by which every console reads what the operator answers, and `scratch.rs` the
  scratch directory of a life, which the files serve as `scratch://` and the commands name as `$TMPDIR`.
- `src/extension.rs` says what an extension is and where a host finds the ones that a life runs, and holds the ear
  `extensions`, which enables each by a fact that the record keeps and plays its word as a rung on each chain, and
  `parts`, the part for the TUI of each extension, which `tui/src/parts.ts` loads live.
  `extensions/` holds the official extensions, `memory`, `skills`, `compaction`, `ouroboros` and `web`, each a word, a
  contract and a suite, and `src/extension/` holds the ear of the World of each that has one: `compaction.rs` answers a
  carry with the turns and the open prompts of a chain of another record, which the reader reads. `docs/extensions.md`
  says how an extension works.
- `src/life.rs` opens a life as every host opens one: the record, and the ears of the crate after the ears of the
  host, the provider among them, on the default actor and the roster that the host names from the catalog, or every
  model that the catalog offers when the host names no roster, or on a model of its own, and the extensions. A host adds only its own ears, such as the console of its operator.
  A life on a record that a build of another version made is refused before any replay, with both versions, and
  `tests/corpus` holds the records that each build of a version must replay with no drift. Every
  host gives it as one `Opening`, and each door gives it as `opened` and as `Engine.open`.
- `src/checkpoint.rs` holds the checkpoint of a life, a fact at the tip of its record that python of its
  own makes in the names of the engine: each chain that lives, with its turns, its module and its open prompts, as facts that
  the journal of any engine replays, so a later life goes on from it on any build. `furb checkpoint` writes one.
- `src/snapshot.rs` holds the snapshot of a life beside its record: the session of the sandbox that monty dumps and
  what the engine holds of the ears, which a later life on the same record and build opens from with no replay.
- `src/engine.rs` opens a life in the sandbox: it loads the engine, the sheet and the Kernel, each in a namespace of
  its own, and it carries each value between the sandbox and a host. An ear of a host is an object of the host in the
  sandbox, which the engine steps as a generator.
- The Kernel is `src/furb/kernel.py`, one module that the sandbox loads and the python package imports, each on the
  names of its engine. The gate, `src/gate.rs`, is the type checker of monty. It reads a word on the sheet of the
  engine, `src/furb/sheet.py`, against the typeshed of the sandbox, and it runs each import of the word in a sandbox
  of its own, which alone says which module runs. The gate of the python package reads through it too.
- `src/binding.rs` holds the host API that both doors give, each function once: the ears of the crate, the store, the
  opening of a life, the catalog, the extensions, the images and the gate. A door holds only how a value of its
  language crosses, its `Engine` and its `Act`, and the loop that drives them. The `Engine` of each door has the same
  methods, one for each verb of the contract, which the build makes from the contract.
- `src/binding/py.rs`, behind the `python` feature, is the door to python. `bind/python` is the package
  `furb-monty`. Its module
  `furb_monty.engine` gives every name of the contract over one life in the sandbox, and `FURB_ENGINE=monty` makes
  `from furb import engine` give it.
- `cli/` is `furb-cli`, the second package of the workspace of the crate, whose program is `furb`, on clap. With no
  command it hands the terminal to the TUI, which it finds in `FURB_TUI`, as `furb-tui` on PATH, or in the checkout
  it was built from. `--mode rpc` serves one life as a JSON-RPC on stdin and stdout, as Pi does, which
  `docs/rpc.md` says. `prompt`, `turns`, `run` and `extensions` each open one life on the record they are given, and `facts` prints
  each fact of a record after the number of its line, filtered by an act or by a chain, and opens no life.
  `checkpoint` writes the checkpoint of a life at the tip of its record.
  Every life runs on the ears of the crate, on the model of its default actor and the models that `--roster` names,
  or every model of the catalog when `--roster` names none, and on a console of the operator. The package `furb` depends on the wheel of `furb-cli`, so `pip install furb`
  gives the command `furb`.

The suite runs on both engines, but for the tests that `MONTY_SKIPS` in `test/conftest.py` names: these boot on a Kernel
of this interpreter, and the engine of monty holds its own. The suite of each extension, `extensions/<name>/test/`, runs
on both engines too, and its conftest, `extensions/conftest.py`, hands it the hooks of the harness and the helpers that
only the suites of the extensions need. `test/outside/test_monty.py` proves what the door carries that no sentence of
the contract says.

The TypeScript side is a bun workspace at the root, with three packages:

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

Read the whole file and list each incoherence: a truth with two homes, a word with two meanings, a part held by the
wrong owner. Probe each, build its reshape, and measure its size and the suite. Keep what adds coherence, then read
the whole file again.

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
5. `engine.py`, minified with its program kept as it is, costs fewer than 6500 tokens of `o200k_base`, the
   tokenizer of OpenAI, and the doctrine of furb, `DOCTRINE.md`, costs at most 2000. The test counts with `o200k_base`
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

- `uv sync`: install the environment, which builds the crate with its `python` feature into the package
  `furb-monty`, and the command line into the package `furb-cli`, and builds them again after a change that their
  `cache-keys` name. `uv sync --reinstall-package furb-monty --reinstall-package furb-cli` builds them again in any
  case. This build, `cargo test`
  and `bun run build` take the dev profile, which optimizes every dependency, the interpreter among them, so a word
  runs as fast as in a release while the crates of the workspace build quickly. A wheel for PyPI takes the release.
- `script/builds.sh <version>`: build the command line at the last commit of `origin/main` that holds a version, and
  put it in the store of builds of the machine, so a record of that version opens with `furb` again.
- `script/worktree.sh <path> <branch> [base]`: make a git worktree on a new branch from `base`, `origin/main` by
  default, whose build starts warm. Run it from any directory. It copies on write the build directories of the main
  checkout into the worktree: the `target` of cargo, which maturin and napi share, with its timestamps, the
  `node_modules` of bun, the build of the TypeScript package, and the cache of the tests of the TUI. A copy on write
  shares the blocks of each file until a build changes it, so it takes less than 1 GiB of the disk. Then cargo builds
  only the crates of the workspace again, because their path changed, and no dependency. The script does not copy
  `.venv`, whose scripts name the path of the main checkout: `uv sync` makes it again on the warm `target`. The copy on
  write works on APFS on macOS and on Btrfs or XFS on Linux. On another file system, the script makes a plain worktree.
  Use it for each new worktree of this repository: a first build takes about 6 minutes, against about 35 in a worktree
  that `git worktree add` makes. Remove such a worktree with `git worktree remove --force <path>`, since its build
  directories are not in git, then `git branch -D <branch>`.
- `uv run pytest -q`: the suite on both engines, with the coverage of `furb` and of `furb_monty`, which must be
  whole.
- `uv run pytest -q test/test_hygiene.py`: the hygiene laws alone.
- `uv run ruff format src test script extensions` then `uv run ruff check src test script extensions`: format and
  lint. Two spaces of indentation, 120 columns.
- `uv run ty check --error-on-warning`: the type check. The tests are checked against `engine.pyi`.
- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace`:
  the gates of the crate and of the command line. The tests of a module stand beside it, `src/engine.test.rs` beside
  `src/engine.rs`, and those of the engine drive the real engine on ears in rust. `cli/tests/furb.rs` runs `furb` as
  a process, on a claude command line that answers from a script.
- `cargo run -p furb-cli -- <words>`, or `uv run furb <words>`: the command line, as `furb --help` says.
- `uv run pre-commit run --all-files`: every gate the commit hook runs.
- `uv run python script/smoke.py`: one real life on the default actor: claude-cli:opus/high through the claude command
  line on PATH, or the one `FURB_CLAUDE_BIN` names, and with no claude the first model of the network or of the local
  server that the catalog offers, which may cost dollars. `--model` names the actor, as `--model local:qwen3` on the
  local server that `FURB_LOCAL_URL` names. It is no test of the suite and spends one prompt.
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
- `python script/bench/loop.py make|furb|furb-own|furb-batch|claude|score <dir>`: the bench of a loop, which its
  docstring says how to run. It makes support tickets from a seed with a known label each, triages them on the same
  local server with a word of furb that asks one prompt per ticket, with a life of furb whose model writes its own
  words, with a word of furb that asks one prompt per batch, and with the claude command line, and scores each arm on
  the totals of the server and the share of right labels. It asks a model and spends what the server serves.
- `script/real.py` is no command: it holds what `smoke.py`, `play.py` and `deepswe.py` share, which is the life
  they open on the engine of this interpreter, the claude they ask, and what the answers of a record cost.
- `uv run python script/catalog.py`: make the snapshot of the catalog, `src/world/provider/catalog.json`, again from
  models.dev and from the data of pi-ai. A life refreshes its own copy of models.dev in the cache of furb once a
  day, and the newer of the two holds for each provider.
- `uv run python script/links.py`: check that each link in a README that a package carries works on the page of the
  package on PyPI, npm or crates.io, which read no relative link. It lists each package and its README from the
  manifests, and fails on a relative link, which `--fix` writes as absolute, at `main` on GitHub. The hooks run it.
- `bun install && bun run build`: install the TypeScript workspace, and build the N-API package that the TUI uses.
- `bun run check`, `bun run lint` and `bun run test`: the type check, the lint, and the tests of the TypeScript
  side. Each test has thirty seconds, and the check and the lint have no limit. The tests read the configs and the
  cache of `.furb/tests`, and not those of the machine. `tui/script/test.ts` deals the tests of each file into runs,
  each a process of its own that runs one test at a time, so a test counts its own time alone and no test runs
  beside one that changes the environment or the clock. Twice as many runs go at once as the machine has cores. A
  word after `bun run test` filters the files. Each script of `bind/typescript/test/programs`, which a test runs as a
  program, compiles before any run starts, so no compile takes the machine from a test.
  As root, the test of a folder that cannot be read fails, since root reads every folder.
- `bun run demo` and `bun run tui`: the TUI on the demo session, which asks no model, or on a real life.
- `bun run docs`: write the tables of keys and commands in `tui/README.md` again from `tui/src/keys.ts` and
  `tui/src/commands.ts`.
- `bun run bench`: the speed of the TUI on a recorded life at the weight of a real record, about 2.6 KB for each fact:
  one of about 160000 facts and one of a tenth of that. It measures what a new fact costs in each life, a line of a
  command, a new act, a paragraph told on the root chain and a token of a model, and in the long life the first frame, cold and warm, the frames while the
  feed scrolls and a command streams, and the memory of a process that opened the life. It fails over its budget,
  which the gates hold on Linux. Each budget is set from a measure of the machine with no code of furb, so that it holds
  on a fast runner and on a slow one: the pace, the time that the machine takes to parse the facts of the long life,
  and the frames that the renderer draws with nothing to show. The warm first frame takes less than 32 paces, the
  renderer draws at least 95 in each hundred of the frames that it draws with nothing to show, the 95th percentile of
  the frames takes less than a pace and less than 16.7 ms, and a new fact costs in the long life at most twice what it
  costs in the short one, or 2 ms. `FURB_BENCH_JSON` names a file that takes the measures of each run, from which the
  budgets are set again, and `FURB_BENCH_FACTS` sets the size of the long life.
- `bun run screenshots` and `bun run animation`: capture `docs/screenshots/` and `docs/furb.gif` again from the
  real renderer. Run both after a change that a screen shows, and read each capture. One run of each at a time holds
  its home in `/tmp`, and a second run waits until the first ends. A screen that does not come in three minutes ends
  the run with an error that names the screen, and a run never waits past thirty minutes.

- `python script/budgets.py`: set the budget of time of each job of the gates, in `.github/budgets.json`, from the
  times of its last green runs on main. The round of the conductor holds each job to its budget, and turns a job that
  passes it into an item, as it turns into an item each test that fails, then passes on the same commit.

Every pull request and every push to `main` runs the gates in `.github/workflows/gates.yml`: the hooks, which
include the gates of the crate and the type check. The suite runs on Linux, on macOS and on Windows, and the gates
of the crate run on Windows too. The gates of the TypeScript side run on Linux, on macOS, and on Windows.

A release is one version of every package: the crates `furb` and `furb-cli` on crates.io, the package `furb` of npm
with the package of its engine for each system, and `furb`, `furb-monty` and `furb-cli` on PyPI.
`release-please-config.json` names each file that holds the version, and `.github/workflows/release.yml` keeps the
pull request of the next release open on main, with `CHANGELOG.md`. A merge of that pull request tags the release,
and `.github/workflows/publish.yml` publishes it from the tag, once it proved each package from its built artifacts
in a clean environment. The merge of any other pull request publishes nothing. crates.io takes no crate that
stands on git, and `furb` stands on the fork of monty by git: the job of crates.io publishes `furb` and `furb-cli`
only when no dependency of either stands on git, which is when monty stands on crates.io with what furb needs, and
until then it says why it skips.

- `sh script/prove-crate.sh [folder]`: pack the crates of monty at the revision of the fork that `Cargo.toml` names,
  and `furb` and `furb-cli` on them by their version, as patches of crates.io, then add `furb` to a new crate that
  takes every other crate from crates.io with no lock of furb, run the example chains there, and install `furb-cli`.
  It proves that the crates pack and build as crates.io will give them once monty stands there. A crate that breaks
  the build in a fresh resolve takes a pin with `=` in `Cargo.toml`, as salsa does. It takes about 5 minutes.
- `sh script/prove-npm.sh <packs>`: install the packs of `npm pack` of `furb` and of the engine of this system in a
  new folder, and run the example chains there with node.

## Prose

**Write all prose in Simplified Technical English**, for a general non-technical audience. The
rules are ASD-STE100, at <https://www.asd-ste100.org/>. This holds for a chat message, a commit
message, a pull request, a review comment, a code comment and a document.

Prose here uses no em dash and no en dash. A docstring is a document.

## Pull Requests

Read `.github/pull_request_template.md` whole and apply it strictly.

It is the one home of the rules of a pull request: its scope, its title, which release-please reads, and its body.
The check `the title`, in `.github/workflows/titles.yml`, runs `script/title.py` on each title, at each change of the
pull request and at each edit of its title, and reads the types from their one home, the `changelog-sections` of
`release-please-config.json`. The commits before this rule stay as they are.

## Comments

A comment says what the code cannot tell by itself. It is a disclaimer: the code does not behave as a reader would
assume, and one sentence spares them a debugging session. Documentation adds edges to symbols and enumerates the
invariants; a public type of furb always gets it, because a client developer reads it: a type of the contract
`src/furb/engine.pyi`, and a type of a door, the crate in `src/lib.rs`, `bind/python` or `bind/typescript`. Everything
else the code, its names, the module tree and `git log` already say.

A comment older than your change is not yours to remove. `TODO` and `FIXME` go with the change that completes the
work they name. Write for the reader a year out: state what is true, never what changed. No em dash. Fill each line to
column 120, the width that the formatters keep here, then wrap; a docstring of a contract keeps one sentence per line.
