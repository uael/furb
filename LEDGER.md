# Ledger: the extension system, second shape

Delete this file when the work is done. Commit and push at each milestone. Run every command under `timeout`
(at most 600 s; `-o timeout=60` for pytest, `--timeout 20000` for bun test).

## What the owner decided (in order)

1. Builtins are no rungs. The owner approved: definitions go in the system prompt, and only life words stay rungs.
2. Then: keep the builtins in `engine.py`. A disabled builtin is a region that is removed from the system prompt.
   Keep `engine.pyi` and the tests of main unchanged, or nearly.
3. Revert the ladder too. The engine code that answers `read`, `write`, `cd` and `cwd` stays where it was on main:
   it is only an answer, and it is inert when no verb asks. Revert most of `engine.{py,pyi}` and the tests, if not all.
4. Keep what was made generic in Rust and in the Python and TypeScript bindings: config, cache, fetch, manifests,
   order, word of a module, World parts, TUI parts, the generic World trait.

## The design

- `engine.py` is main's file, with top-level comment lines `# region <name>` and `# endregion` around the definitions
  of each builtin: files (span, grep, differs, HEAD, TAIL, HIDDEN, read, write, cd, cwd, Text, showing, the import of
  dataclass), bash (TIMEOUT, bash, Exit), grant (WINDOW, grant). A name may have more than one region. Code that only
  answers stays outside the regions. `engine.pyi` is main's file.
- The engine that runs is always whole. A region only leaves the system prompt.
- The system prompt is the engine with the regions of the builtins that are off cut, minified in layout alone, then
  the word of each external extension, as the crate makes it. Python makes the minified segments; the TypeScript
  build writes them to `system.json`; each host joins the segments of the builtins that are on and appends the words.
- An external extension word runs in the module of the engine before boot, so every chain module copies its names at
  birth and a replay finds its verbs. The engine source that the monty life runs and that the gate reads is the
  engine plus the words. The python host runs the words in the module `furb.engine`.
- A session keeps the words it started with: the World says `("extensions", root, words)` once, the record keeps it,
  and a later life takes the words from the record before boot. (Decide the exact shape when the host step starts.)
- Life words (`skills()`) stay rungs, played by the host after boot on every chain without a source, and at the
  birth of each such chain.
- skills reads its earlier answers from `asked` and `outcomes`, not from the transcript query (main's engine keeps
  the done of a transcript query in the transcript, so each read would nest the reads before it).

## Steps

1. Engine, contract and tests back to main; regions; hygiene laws for regions and for extension contracts.
2. Crate: builtins carry no word; the engine source with words; pinned words; no play of words, life words still
   played; life tests.
3. Python host: parts answer main's values (Text, Exit); system prompt of segments; words before boot; pin; tests.
4. TypeScript host: the same; build writes segments.
5. TUI: no fold of played words; life word rungs.
6. skills: imports from furb.engine, asked and outcomes, suite on the new harness.
7. Documents, every gate, screenshots, PR description.

## Done

- Step 1, part: `engine.py`, `engine.pyi` and every test outside `test/outside` equal main (`git diff origin/main --
  src/furb/engine.py src/furb/engine.pyi test/ ':!test/outside'` is empty). `src/furb/builtin/`, the builtin suites
  and the ladder are gone. Decision: no region comment in `engine.py` (src/furb/CLAUDE.md: engine.py holds no
  comment); the crate holds the table `BUILTINS` of the top-level names each builtin defines, and
  `extension::system(minified, taken, words)` cuts the statements that bind only names of a builtin that is off.
- Step 2, part: `extension::source(words)` is the engine then the words; the monty life runs it as the engine module
  and its gate reads it (`gate::checked(sheet, engine)`); the preamble plays life words only (`LIVES`); napi gives
  `systemPrompt(engine, taken, words)`, pyo3 gives `system_prompt` and `engine_source`, and `gate(sheet, engine=None)`.
  `cargo test --lib extension`: the extension tests pass; two life tests wait for the Yard to answer a read with
  `{"is": "Text", "path", "content"}` (the mark the preamble makes an engine Text from, as main's hosts did).

## Next

- Life tests (src/life.test.rs): Yard answers read and write with the Text mark; the bash part answers as main's
  engine asks (merged, out, exited, feed); drop the tests of played words, keep the life words tests; a test that a
  word of an extension is bound on every chain from birth and that the gate reads it.
