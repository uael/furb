# Ledger: the extension system (work in progress, delete this file when the work is done)

Branch: `claude/extension-system-design-j95po6`. Push there only. No pull request unless the owner asks.

## The request of the owner

Design the extension system. Move (cd/cwd/read/write/Text), (grant) and (bash) out of the engine as builtin,
opt-out extensions. An extension has three parts: python code played as a rung (on the root chain, and on every
chain without a source), TypeScript code for the World, TypeScript code for the TUI. Define the configuration file,
the cache, and how external extensions are loaded (by path, git, npm). Same for the local `.furb/` directory. Prove
it with an official `skills` extension built outside the core in a new folder `extensions/skills/`, served like any
external extension. Tell the owner if the pypi package `furb` (and npm `@furb/skills`) needs a release by them.

## Decisions the owner made (AskUserQuestion)

1. Contract: as proposed. Builtins leave `engine.pyi`; each builtin has its own `.pyi` and suite. The core gains a
   `ladder` query (the chain answers it for its prompts; the files extension serves the prompt door with it). A
   Showing note is `(path, content, show)`. The World answers extension questions with plain data, and the
   extension verb makes its own class from it (so a record holds no class of an extension).
2. New chains: the HOST PLAYS THE EXTENSIONS ON EVERY CHAIN WITHOUT A SOURCE, a model's too. Rule used: the host
   plays nothing while boot replays the record (an act made during replay shifts names and drifts). After boot, it
   plays the missing words on every chain without a source (compare the chain's `program` query values with the
   words), and from then on it plays them at the birth of each chain without a source (hear the `chain` fact whose
   source word is ""). It plays as the World (site `world`), so the journal keeps the rung as an act of the outside
   and a later life says it again. Known limit: a model that opens a chain and prompts it in the same word gets its
   first ask before the extension rungs are told; they run before its answer.
3. Layout: XDG. Config `$XDG_CONFIG_HOME/furb/config.json` (FURB_CONFIG_DIR overrides, as ui.json already does,
   `%APPDATA%\furb` on Windows). Cache `$XDG_CACHE_HOME/furb` (`~/.cache/furb`, `%LOCALAPPDATA%\furb` on Windows).
   Local `<project>/.furb/config.json` overrides the home config by extension name. All fetched extensions go to the
   home cache. Make `furbDirectory` write `.furb/.gitignore` with `*` and `!config.json`, and write it whenever it is
   missing (not only when it makes `.furb`).

## Design (planned, write it in docs/extensions.md)

- Config: `{"extensions": {"<name>": false | true | "<path>" | {"path"} | {"git", "ref"?, "path"?} | {"npm", "version"?}}}`.
  Keys are extension names; builtins `files`, `bash`, `grant` are on unless `false`. A path resolves against the
  directory of that config file; `~` expands. The manifest name must equal the key.
- Manifest: `package.json` with a `furb` field: `{"name", "python", "world"?, "tui"?, "requires"?: [...]}`.
  Builtins have their manifest in code. Order: builtins (files, bash, grant), then home config, then local; sort
  by `requires`; refuse one whose requirement is off. bash requires files.
- Cache: git → `git clone --depth 1 [--branch ref]` into `<cache>/extensions/git/<sha256(url#ref)>/`, subfolder by
  `path`; npm → `npm pack <spec>` then `tar -xzf` into `<cache>/extensions/npm/<name>@<version>/`; fetched once,
  refreshed by an explicit update. Runtime deps: `npm install --omit=dev --ignore-scripts` when the package has any.
- World part (TS): a function of a context giving a part that hears facts the core World does not consume; it may
  yield calls to the life (`{verb, args, kwargs}`) and return a saying; it speaks later as the World. It declares its
  pending kinds and may update the live value of its acts in `Activity` (bash: `out` facts). A `start` of a kind no
  part knows is closed with `Refused("the World does no <kind>")` (new contract law, already in engine.pyi Ear).
  File changes go through a generic `world.change({path, before, after})` so the Changes view stays in the TUI core.
- TUI part: slash commands, how to show acts of its kinds (title, subject, preview, whether it is work, whether it is
  hidden), sidebar rows (grant: ceiling and meter), and a prefix (`!` for bash). Types live in `@furb/engine`
  (extension.ts) since `@furb/tui` is private. Replace the old `tui/src/extensions.ts` and `--extension`/`/extension`.
- A value of a class a word defined crosses to TS as `{is: "instance", class: {...}, value: {fields}}`; add a helper
  to unwrap it. Verbs of extensions are called by name with `on`: the preamble `called()` now resolves a verb in the
  module of the chain `on` names when that module binds it (DONE). The TS World asks `cwd` by `{verb: "cwd", kwargs: {on}}`
  and falls back to its own directory when the chain binds no cwd; it asks `merged` of the bash act.
- Python host (`src/furb/world.py`, `cli.py`): generic core + python world parts for files and bash; plays the
  builtins the same way (read the words with importlib.resources from `furb/builtin/*.py`). It loads no external
  extension (world parts are TS). Rust crate: `World` trait keeps stand, clock, chance, keep, ask, wait, prompt and
  gains a generic hears/answered for other facts (the preamble `worldly` forwards unknown facts); remove typed
  read/write/cd/cwd/grant/bash from `Life`, `Text`/`Exit` from value.rs and wire.rs, fix gate.rs and life.test.rs
  tests (play `include_str!("furb/builtin/*.py")` where a test needs a builtin). napi: remove TextValue, ExitValue,
  BashOptions, GrantOptions and the typed builtin methods; rebuild `index.cjs`/`index.d.cts`.
- The npm package bundles the builtin words (bind/typescript/build.ts writes them like system.json).

## Done so far (committed and pushed, commit 3f3fa27, plus uncommitted edits in test/)

- `src/furb/engine.pyi` and `engine.py`: builtins removed; `ladder` query answered in the chain (`("ladder", qid,
  by, on, prompt, *word)`); Showing `(str, str, Show)`; `shown(note, seen)`; sentences rewritten (see `git diff
  b104b8c -- src/furb/engine.pyi`). Engine is 4567 tokens.
- `src/furb/builtin/{files,bash,grant}.py` words and `.pyi` contracts. files: span, grep, differs, HEAD, HIDDEN,
  door, landed, showing, read, write, cd, cwd, Text; write asks `("write", on, path, content)`; read of a door asks
  `ladder`; cwd reads the `transcript` and `stand` queries. bash: TIMEOUT, TAIL, bash, Exit. grant: WINDOW, grant.
- `src/preamble.py`: `called()` resolves verbs in the chain module; `fielded()` carries the fields of a word's
  dataclass out one by one (nested Text in Exit reached python as "no class this interpreter knows"). Rebuild the
  crate after any change of engine.py or preamble.py: `uv sync --reinstall-package furb-monty` (10 s).
- `test/conftest.py`: `BUILTIN`, `FILES`/`BASH`/`GRANT` word lists, `Sand(words=...)` plays them on chains without a
  source (after boot in `life()`, and at chain birth), `where(on)`, `verb(name, on)`, `Bound(on)` (attribute access
  to a chain module, keeps `__monty__` handles), `plays()`, `finishing`/`job(on)` (a pausable act the World finishes
  with a `finished` fact, answers `look`), WORD is now clock + wait (`k == 6`), `lived()` gives 6, DOOR answers the
  plain string "kept". Sand answers read/write with plain `{"path","content"}`, refuses an unknown started act.
- `test/test_hygiene.py`: CONTRACTS = engine.pyi → test/, builtin/<x>.pyi → test/<x>/; no-shadow law also for the
  builtin words (engine names + other words as the outer scope).
- Builtin test files moved (git mv) to `test/files/`, `test/bash/`, `test/grant/` but NOT yet rewritten.
- Core test files rewritten and green on both engines: ask, ask_shape, act, act_await, acts, act_shape, asked,
  actor_shape (window test moved to test/grant/test_window.py), boot, bound, cancel, cancel_shape, close,
  close_shape, control, covers, done, drift, drive, ear, ended, entry, fact, filter, gate_shape, get, headed, holds,
  keep, note, outcomes, pause, pause_shape, pausing, peek, peek_shape, prompt, question, question_shape, ready,
  refused, send, wake, wake_shape.

## Next steps, in order

1. Finish the core suite: rung (next; see grep of bash/read/Text there), run, saying, scope, sent, site, stand,
   standing, start, started, stood, take, tell, told, transcript, turns, turns_of, under, unquoted, usage, wait,
   wants, world, show, showing, shown, chain (78 tests), and a new `test/test_ladder.py` for the Ladder sentences.
   Patterns: a command → `wait(n)` (World done: `engine.send("done", id, v, by=WORLD)`), a paused act the World
   finishes → `job(root)` + `("finished", id, v)`, a read → `clock()` (a World query the record keeps), a door read →
   `engine.ask("ladder", chain, prompt[, word])`, a telling act → a prompt `to=OPERATOR` closed by `engine.close`,
   a tell of a showing → `tell('seen', 'x', ('/p', 'content', lambda lines: [1]))` inside a rung. An ear that calls
   the bus must not do so before its first yield (monty primes it on the main thread: "Already borrowed").
   Then list mismatches with the script used before (hygiene check of MISSING/STRAY per contract).
2. Rewrite test/files, test/bash, test/grant with `Sand(words=BASH|FILES|GRANT)`, `Bound(root)` for verbs, fix ids
   (extension rungs take rung1.. and their words stand in the turns), one test per sentence of each .pyi.
   Exclude `src/furb/builtin/*.py` from ty and coverage; ruff per-file-ignores F821 etc. for them.
3. Python host (world.py, cli.py, test/outside), crate (preamble worldly, world.rs, life.rs, value.rs, wire.rs,
   gate.rs, life.test.rs, py.rs, furb_monty engine: drop "span", "grep", ... from PURE and bash/grant from ACTS).
4. TypeScript loader + generic World + builtin world parts, napi, README; then TUI parts and app.ts/session.ts
   (bash/grant/read/cd specifics), docs, screenshots (bun must be >= 1.4.2 for the TUI; host has 1.3.11).
5. `extensions/skills/`: package.json manifest, skills.py + skills.pyi + tests, world.ts (finds SKILL.md under
   `.furb/skills`, the config dir `skills/`, and `.claude/skills`), tui.ts (`/skills`, `/skill <name>`); prove path,
   git (local bare repo) and npm (local tarball) loading in tests.
6. docs/extensions.md, CLAUDE.md, src/furb/CLAUDE.md (technical names: door, text, command, merged, show, grant,
   ledger move to the extensions; add ladder, extension), developer guide, READMEs; every gate green
   (`uv run pytest -q`, hygiene, ruff format/check, `uv run ty check --error-on-warning`, cargo fmt/clippy/test,
   `uv run pre-commit run --all-files`, bun check/lint/test). Delete this ledger. Tell the owner a release of `furb`
   (pypi, tag `v*`) and a publish of `@furb/skills` (npm) are theirs to do.

## Environment notes

- Use the uv at `~/.local/bin/uv` (0.12.17) and Python 3.14.7; the preinstalled one only had 3.14.0rc2.
- Run a file on both engines: `uv run pytest -q -p no:cacheprovider --no-cov -n0 -o timeout=20 test/test_x.py`.
- Prose in Simplified Technical English, no em dash or en dash. Commit trailer:
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` and
  `Claude-Session: https://claude.ai/code/session_01Luoyf6crfHuvNmZs94como`.
