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

4. Later added by the owner: World parts may be Python as well as TypeScript, later (not in this effort). Keep the
   door open: the manifest `world` field names one file per language, `{"world": {"ts": "world.ts", "py":
   "world.py"}}`, and the World part has the same shape in both languages: it hears a fact, yields calls to the
   life, gives a saying, and speaks later as the World.
5. Later added by the owner: the python part of an extension may be real code. Both forms hold, generically: (a) a
   word, a python file played as it is; (b) a real python module that imports what it uses from the engine (`from
   furb.engine import ask, act, tell, Show`) and from the extensions it requires (`from furb.builtin.files import
   Text`), so that editors, ruff and ty check it, with its own tests and gates. The builtins become modules of form
   (b), and lose their per-file ignores and their exclusion from ty.
   Choice (ours, write it in docs/extensions.md): the host turns a module into the word it plays by blanking every
   top-level `from furb...` import (`furb.engine`, `furb.builtin.*`, and `furb.extensions.*`, the package an
   external extension names its python part under), each line of such a statement made an empty line, so every line
   keeps its number and a finding of the gate points at the line of the file. A word of form (a) has no such import,
   so it is played as it is: one rule serves both forms. Why not teach the gate and the Kernel the imports: the
   module of a chain already binds every name of the engine and of each word played before it, and the engine reads
   its own names through the globals of the chain, so a name a later rung rebinds is used from the next use on; an
   import would pin the object of the module it names, and a rebound verb would not reach the word. The sandbox of
   monty holds no package `furb`, the gate and the Kernel stay blind to packages, and the program a model reads holds
   no import it cannot run.
6. Later added by the owner: the shared machinery lives in the crate, so the TypeScript host, the Python host and a
   future Rust host share one implementation: reading and merging config.json (XDG home and the local `.furb`), the
   cache, fetching by path, git and npm, reading manifests, ordering by `requires`, turning a module into a word, and
   the rule that plays the words on every chain without a source after boot and at each chain birth. napi and pyo3
   expose it. Only what is truly per language stays in TS and Python: the World parts, the TUI parts, and the dynamic
   import of their code.

7. Later answered by the owner (PLAN.md part 1, decision 13): a manifest may name a `life` word beside `python`,
   as `{"python": "skills.py", "life": "skills()"}`. The module is played once per chain by the play rule. The life
   word is played as the World in every life: after boot on each live chain without a source, and at the birth of
   each such chain right after the module words, never during replay. The crate carries it with the play rule
   (Opening takes the life words beside the words, and the preamble plays them). `skills.py` defines names only,
   and `skills()` tells only the skills that are new, changed or gone against the transcript of its chain. The TUI
   part gives `/reload-skills`.
8. Later answered by the owner (decision 16): Text, read and write keep their signatures and meanings. A record of
   0.1.0 must still open: every record reader (`unwire` in `src/furb/world.py`, `again` in `src/preamble.py`, the
   TypeScript reader) keeps the mark of a class it does not know, `{"is": "Text", ...}`, as its plain fields, and
   `landed()` makes the Text. A test opens a real record of 0.1.0 on each host.
9. Later answered by the owner (decision 17): a requirement that is off refuses the extension that needs it, with a
   message that says to turn that one off too. An upgraded word is played as one more rung, and it binds last.

10. Ruled by the lead, who holds every decision of the owner (the owner approved the changes of the lead to
    engine.pyi): the sentence of `Show` says "A show is no word of an act", since a tell carries a showing, and so
    its show, as a word, while the record keeps no tell. The sentence of `turns` says "a showing stands by the lines
    the model has not seen", since text is a word of the files extension now. The tests carry the new sentences.
11. Ruled by the lead: a record of 0.1.0 gets no replay of the builtin words at the birth of its root (that renumbers
    the rungs and drifts), and no tool migrates it. Such a record opens when nothing hangs on its old acts, which a
    test proves on each host. docs/extensions.md and the release note say that a record of 0.1.0 whose later acts
    hang on an old read, write or bash drifts.

PLAN.md (by the lead) holds the plan of the remaining steps and more decisions of the lead (part 1, binding). Read it
with this ledger.

## Design (planned, write it in docs/extensions.md)

- Config: `{"extensions": {"<name>": false | true | "<path>" | {"path"} | {"git", "ref"?, "path"?} | {"npm", "version"?}}}`.
  Keys are extension names; builtins `files`, `bash`, `grant` are on unless `false`. A path resolves against the
  directory of that config file; `~` expands. The manifest name must equal the key.
- Manifest: `package.json` with a `furb` field: `{"name", "python", "world"?: {"ts"?, "py"?}, "tui"?, "requires"?:
  [...]}`. `python` names a file of form (a) or (b); the crate makes the word of it (decision 5).
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

## Done so far (committed and pushed)

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
- Step 1 of the old plan is done: the whole core suite is rewritten on core means alone and green on both engines
  (1180 passed, 4 skipped), with a new `test/test_ladder.py`; every sentence of engine.pyi has its one test. ruff
  and ty are clean on `test/test_*.py` and `test/conftest.py`. The four red hygiene tests are the builtin suites
  (step 2 below). Patterns used: a command became `wait(n)` or `job(root)` with a `finished` fact, a read became
  `clock()`, a door read or write became `engine.ask("ladder", chain, prompt[, word])`, an act that tells its open
  and its close became a prompt `to='operator'` that the operator closes, a showing is told from a rung with
  `tell('seen', 'x', (path, content, show))`, and the directory of a chain is read with `where(on)` (its standing
  while no cwd is bound). Monty has no `__call__` on an instance of a word's class, so a show is a def or a lambda.
- Core test files rewritten and green on both engines: ask, ask_shape, act, act_await, acts, act_shape, asked,
  actor_shape (window test moved to test/grant/test_window.py), boot, bound, cancel, cancel_shape, close,
  close_shape, control, covers, done, drift, drive, ear, ended, entry, fact, filter, gate_shape, get, headed, holds,
  keep, note, outcomes, pause, pause_shape, pausing, peek, peek_shape, prompt, question, question_shape, ready,
  refused, send, wake, wake_shape.

- The builtin suites are done (commits c252c1e to this one): the builtins are python modules of form (b), which
  import from `furb.engine` and `furb.builtin.files`; ruff reads them with the vocabulary ignores of the engine (the
  contract fixes their signatures, and a word types its verbs and nothing else), and ty reads them with
  `invalid-argument-type` and `invalid-return-type` off (the bus answers object, and a verb gives the act of its
  kind), and coverage omits them (they run as words). The crate has `src/extension.rs` with `word` (LF first,
  blank the top-level `from furb` imports, keep every other byte, refuse a file that does not parse with its line),
  `builtins`, `words`, `missing`, `Extension`, `Worlds` and `Error::Word`, with `src/extension.test.rs`. furb_monty
  gives `Extension`, `builtin_extensions`, `word_of` and `missing_words`. `conftest.BUILTIN` reads the words from
  the crate; the builtin suites run on both engines (`pytest_generate_tests` parametrizes every suite folder but
  `outside`); the no-shadow law reads the word the crate makes and accepts an import that binds a name the way the
  engine or another word binds it. `uv run pytest -q --no-cov test --ignore=test/outside`: 1430 passed, 4 skipped,
  hygiene green; ruff and ty clean on the suite. Test helpers added to conftest: `sown(words)`, `worded(word,
  words)` (runs a word on a fresh root), `texted(got)` and `exited(got)` (read a Text or an Exit on both engines:
  monty gives the fields of a word's class and no method), `made(on, name, *args)` (call a class or a show that a
  chain binds, as the operator). To pass HIDDEN itself (a verb compares it by identity), use
  `getattr(Bound(root), "HIDDEN")`. `unwire` keeps the mark of a class it does not know as plain data.

- Step "Crate and Python host", crate part done (commits f0ef8a5 and cbb1fc5), per PLAN.md part 4 with the
  amendments: `src/extension.rs` has `Error` (Config, Entry, Fetch, Manifest, Requires, Cycle, Word), `Places`
  (`here()` and `of(var, home, windows)`, with `FURB_CONFIG_DIR` and `FURB_CACHE_DIR`), `Source`, `Setting`,
  `Entry`, `settings`, `read_settings`, `merged`, `config`, `fetched` (path; git with `-c core.autocrlf=false`;
  npm pack then tar and flate2 in rust), `npm_dir`, `installed`, `Worlds`, `Manifest` (every part optional, one at
  least; `life` is a word), `manifest_of`, `manifest`, `Extension` (`word` and `life` are options), `builtins`,
  `loaded`, `ordered`, `extensions`, `words`, `lives`, `missing`, `word`. 49 tests in `src/extension.test.rs`
  (git and npm proved offline with a local bare repo and a local folder). The World trait lost read, write, run,
  Command, Running, Voice::out and Voice::exited, and gained `kinds`, `hears` and `answered`; the stand-in
  `worldly` keeps the core, closes a start of an unknown kind with `Refused("the World does no <kind>")` (read off
  `acts[about][0]`), and hands every other fact to `world.hears`. The play rule is in the preamble (`WORDS`,
  `LIVES`, `plays`, `playing`, `played`) and `Opening::words` and `Opening::lives` feed it; `Life::awaited(id)`
  awaits an act by name. `Life` lost the typed read, write, cd, cwd, grant and bash; value.rs lost Text and Exit;
  ts/wire.rs lost its Text branch (ts.rs still has its typed methods, which call verbs by name; step 4 removes
  them). furb_monty gives `extensions(project, refresh=False)`, `places()`, `builtin_extensions()`, `word_of`,
  `missing_words`, and `Extension` with `word` and `life`. `cargo test` (89), clippy (no feature, python,
  typescript) and fmt are green. Always run the crate tests under `timeout`: a test that awaits an act nobody
  ends hangs (an act a World closes must be wrapped in `ending`).
- Python host done (commit fd95163): `src/furb/world.py` keeps the core in `Live.hears` and hands every other fact
  to parts (`Part` protocol: `kinds` and `hears(a)` yielding sayings; `Files` and `Bash`, table `BUILTINS`); a
  start of a kind no part does is closed with `Refused("the World does no <kind>")`; `Live.words`, `lives`, `parts`,
  `booted`, `start()`, `where(on)`, `plays(chain)`, `play()`; `world.verb(on, name)` says a verb a chain binds.
  `unwire` keeps an unknown mark as plain data. `cli.py` loads `furb_monty.extensions(cwd)`, plays after boot,
  gains `furb update`, and ends with `furb: <why>` on a `Refused` of the config. The provider has its own operator
  `WINDOW`. test/outside is green (FURB_CONFIG_DIR and FURB_CACHE_DIR set per test), with new tests: grant off by
  config, a project extension with a life word, a config error, update, an unknown start refused, words played at
  birth, a 0.1.0 record opens (fixture `test/outside/record-0.1.0.jsonl`, made by the code of b104b8c; the crate
  test opens it too). `uv run pytest -q`: 1565 passed, 4 skipped, coverage 100%. ruff, ty, cargo test (90), clippy
  and fmt are green.
- TypeScript host, bind part (commit 4fcfd5c): `src/binding/ts/extension.rs` gives `configDirectory`,
  `cacheDirectory`, `builtinExtensions`, `resolveExtensions(project, {refresh, install})`, `wordOf` and
  `missingWords`; the places come from `process.env` of JavaScript (bun keeps it apart from the environment of the
  system, so the preload of the tests, `bind/typescript/test/preload.ts` from the root `bunfig.toml`, reaches the
  crate). `Life.boot(callback, names, record, words, lives)`; ts.rs lost the typed builtins. `extension.ts` has
  the shapes (`Call`, `Saying`, `Hearing`, `Fault`, `WorldPart {kinds, hears, live, dispose}`, `WorldContext`,
  `WorldExtension`, `Instance`, `isInstance`, `unwrapped`, `remade`); `ears.ts` has one dispatch (`WorldAdapter`
  with `parts`, `send`, `close`, `worldContext`); `builtin/files.ts` and `builtin/bash.ts` are the builtin parts;
  `extensions.ts` loads the parts (`builtinWorldParts`, `loadWorldParts`, `imported`); `World.load` and an async
  `boot`; `Activity` takes the live views of the parts and marks `started`; `furbDirectory` writes its rule
  whenever it is missing. The bind tests are green (72), with `test/extension.test.ts` and a test that opens a
  record of 0.1.0.
- Decisions of the lead in this step (the owner handed every decision to the lead):
  - A part of a World has the same names in every language: `kinds` and `hears`, as the Python `Part` and the rust
    `World` trait have them (not `acts` and `hear` as PLAN part 2 wrote), and the start of an act reaches it
    through `hears` like any fact. Only the TypeScript host adds `live` and `dispose`.
  - A verb called by the operator with no chain is said on the chain of who speaks (decision 4 of PLAN); a name
    bound nowhere raises `NameError` with the chain, or with "no chain was said".
  - An act is `started` when a start said so, or when the record showed it begun: an act that the World does asks
    the journal at its birth whether the record holds it (`holds@<act>.N`), and a record that holds it shows it
    begun. A later life says no start for such an act before a wake, so the start fact alone cannot say it.
  - Engine: a chain holds none of the answers it gives (a new sentence of `chain`, tested). Before, the done of a
    transcript query stood in the transcript it answered, so each read of the transcript held every read before it;
    a host that facts cross to (napi, the monty engine of python) copied that nesting and hit the recursion limit
    after ten reads of `cwd`, which the World asks at every read and write.
  - A path of a config reads as the directory it is (`extension::tidy`), so `../x` names no `.furb/../x`.

## Next steps, in order

1. TypeScript, the TUI (PLAN part 2, sections 4 and 5, commits 2 to 4): the switch of the TUI to `life.call` and
   `unwrapped`, hidden world rungs, the loader in the worker and the bridge, `tui/src/builtin/{files,bash,grant}.ts`,
   `/extensions`, the general commands, prefixes, views and sidebar; tests; `bind/typescript/README.md`,
   `tui/README.md`, `docs/tui.md`, screenshots and animation (bun 1.4.2 is at `~/.bun/bin`).
2. `extensions/skills/`: package.json manifest, skills.py + skills.pyi + tests, world.ts (finds SKILL.md under
   `.furb/skills`, the config dir `skills/`, and `.claude/skills`), tui.ts (`/skills`, `/skill <name>`); prove path,
   git (local bare repo) and npm (local tarball) loading in tests.
3. docs/extensions.md, CLAUDE.md, src/furb/CLAUDE.md (technical names: door, text, command, merged, show, grant,
   ledger move to the extensions; add ladder, extension), developer guide, READMEs; every gate green
   (`uv run pytest -q`, hygiene, ruff format/check, `uv run ty check --error-on-warning`, cargo fmt/clippy/test,
   `uv run pre-commit run --all-files`, bun check/lint/test). Delete this ledger. Tell the owner a release of `furb`
   (pypi, tag `v*`) and a publish of `@furb/skills` (npm) are theirs to do.

## Questions for the owner

None open. The owner handed every decision to the lead: decide by the bar of CLAUDE.md, write the decision and why
here, and go on.

## Environment notes

- Use the uv at `~/.local/bin/uv` (0.12.17) and Python 3.14.7; the preinstalled one only had 3.14.0rc2.
- Run a file on both engines: `uv run pytest -q -p no:cacheprovider --no-cov -n0 -o timeout=20 test/test_x.py`.
- Prose in Simplified Technical English, no em dash or en dash. Commit trailer:
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` and
  `Claude-Session: https://claude.ai/code/session_01Luoyf6crfHuvNmZs94como`.
