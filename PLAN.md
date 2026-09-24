# Plan: the rest of the extension system (delete with LEDGER.md when the work is done)

LEDGER.md holds the request, the decisions of the owner and the state. This file holds the plan of the steps that
remain, as the lead checked it against the code. Part 1 is binding: where a plan below says otherwise, part 1 wins.
Part 2 is the plan of the TypeScript host and the TUI, part 3 the plan of the skills extension, the proofs, the
documents and the releases. Part 4 is the plan of the crate and the Python host.

## Part 1: the decisions of the lead

1. One dispatch in every World adapter. Each adapter (the stand-in `worldly` in `src/preamble.py` for a World in
   rust, `WorldAdapter` in `bind/typescript/src/ears.ts`, `Live.hears` in `src/furb/world.py`) keeps only the core:
   stand, clock, chance, keep, ask, and the start of a wait or a prompt. A start of any other kind goes to the part
   that names that kind, and is closed with `Refused("the World does no <kind>")` when no part names it. Every
   other fact goes to each part in order. The `World` trait of the crate loses read, write, run, `Command`,
   `Running`, `Voice::out` and `Voice::exited`; a host in rust writes parts as a host in TypeScript does.
2. The npm build copies no builtin word. The crate carries the words (`include_str!`) and napi gives them: one home.
3. Every part of a manifest is optional (`python`, `world`, `tui`), and a manifest names one at least. A TUI-only
   extension is valid.
4. A verb said without `on` resolves its chain as the engine resolves it, from who speaks, and not always the root.
5. Pending is derived from the `start` fact: an act is pending when the record shows it started and not done and no
   pause holds it (`LiveAct.started`), and a prompt or a rung as today. The fixed lists of pending kinds go
   (`world.ts:49`, `session.ts:57`, `activity.ts:36`, `activity.ts:136`), and a record is inspected with no code of
   an extension.
6. The rungs the World plays (`by == "world"`) stand in the TUI as one compact line that names the extensions. They
   are no point of rewind, and they count in no total, so a session opens with nothing unread.
7. `extension::word` turns CRLF into LF before anything else, so a word is the same on every machine and the play
   rule never plays it again for a line end. It changes nothing but the lines of an import it blanks: no trim of
   the end of other lines, since a string literal may hold that space.
8. The no-shadow law reads the word the crate makes of a module, after the blanking, not the file on disk. It
   accepts an import that binds a name the same way in two words (`from dataclasses import dataclass`).
9. The extension API of the crate takes the config directory, the project directory and the cache directory as
   parameters. One function of its own computes the defaults from `FURB_CONFIG_DIR`, `XDG_CONFIG_HOME`,
   `XDG_CACHE_HOME`, `APPDATA` and `LOCALAPPDATA`. No test sets an environment variable.
10. The fetch runs on Windows: `npm.cmd` there, the npm tarball extracted in rust (`tar` and `flate2`), a local git
    remote as a `file:///` URL, and every git call with `-c core.autocrlf=false`.
11. The TUI gives no `{is: "name", name: "HIDDEN"}` any more (`app.ts:2439`, `app.ts:2454`): a read of the operator
    outside an act tells nothing already, so it gives no show.
12. `/edit` reads and writes a door through the core `ladder` query, so it needs no extension.
13. The owner chose (answered): the skills extension refreshes what lives outside the record.
    - A manifest may name a `life` word beside `python`: `"furb": {"python": "skills.py", "life": "skills()"}`.
      The module of `python` defines names alone and is played once per chain, by the play rule. The `life` word is
      played as the World in every life: after boot, on each live chain without a source, and at the birth of each
      chain without a source right after the words of the modules. Never while the record replays. The crate
      carries the rule, so every host plays it the same way. A chain with a source gets nothing of its own.
    - `skills.py` has no last line `skills()`. `skills()` compares what the World answers now with what the
      transcript of its chain told before (as `cwd` reads the transcript), and tells one line for each skill that
      is new or whose description changed, one line for each skill that is gone, and nothing when nothing changed.
      The first life tells every skill; a resume tells what changed.
    - The TUI part of skills gives `/reload-skills`, which runs the rung `skills()` on the chain on screen.

14. Part 4 amends. A World in rust says the kinds of act it does (`fn kinds(&self) -> Vec<String>`, empty by
    default), as a World part in TypeScript says `acts`. The stand-in closes a start of a kind that is no wait, no
    prompt and no kind of the World with `Refused("the World does no <kind>")`, reading the kind from `acts[about][0]`.
    Drop `world::refused`, which read the kind from the name of the act.
15. Part 4 amends. `extension::word` gives `Result<String, Error>`: a python part that does not parse is refused
    when the extension loads, with its file and its line, so the host says it once and plays nothing, and no chain
    plays a broken word at each birth. It turns CRLF into LF (decision 7), where part 4 keeps CRLF. An import that
    shares its line with another statement goes with its semicolon, as the code does now, where part 4 refuses it.
16. The owner chose (answered): `Text`, `read` and `write` keep their signatures and meanings. Only what the word
    asks the World and what the World answers are plain data. A record of 0.1.0 still opens: the reader of a record
    (`unwire` in `src/furb/world.py`, `again` in `src/preamble.py`, the reader of `bind/typescript`) keeps the mark of
    a class it does not know, `{"is": "Text", ...}`, as its plain fields, and `landed()` makes the Text of it. On
    replay a query takes its answer by its name and nothing compares its words, so the old shape of the write
    question does not drift. A test opens a real record of 0.1.0 on each host.
17. The owner chose (answered): a requirement that is off refuses the extension that needs it, with a message that
    says to turn that one off too. An extension whose word changed (an upgrade) is played as one more rung on a chain
    whose record holds the old word, and the later word binds last.

## Part 2: the TypeScript host and the TUI

This is a plan only. I edited no file. Line numbers are from the tree at 94263c5, with the uncommitted `test/test_rung.py` edit in place.

### 0. What must land first, and what I found while reading

**Prerequisite.** LEDGER step 3 (a) and (b) must land before step 4 can build: `src/extension.rs`, `Life` playing the words itself, and the crate cleanup (`life.rs:591-760` loses the typed read, write, cd, cwd, grant and bash; `value.rs` and `wire.rs` lose Text and Exit). `src/binding/ts.rs` calls those typed methods, so the napi change in section 3 is forced by step 3 (b) anyway. Only commit 1 below can land before the crate work.

**Findings to act on:**

1. **The TS World is already out of date with the new words.**
   - `ears.ts:171` reads a write as `words[1] = {path, content}`. The new fact is `("write", qid, by, on, path, content)` (`files.pyi:116`).
   - `ears.ts:187` answers with `{is:"Text", ...}`. Decision 1 wants plain `{path, content}`.
   - `ears.ts:166` and `ears.ts:225` ask `cwd`, which the engine no longer binds. Ear calls reach the chain module only through `preamble.py:493-503` then `called()` (`preamble.py:465-475`), so `on` must always be given.
2. **Instances cross in a shape that is not symmetric.**
   - Out: `{is:"instance", class:{is:"class", id, name, base}, value:{fields}}` (`preamble.py:144-145`).
   - In: `{is:"instance", class:<id>, fields:{...}}` (`preamble.py:181-184`).
   - The helper must give both directions.
3. **`{is:"name", name:"HIDDEN"}`, used at `app.ts:2439` and `app.ts:2454`, will no longer resolve.** `again()` resolves a name only among the engine names (`preamble.py:174`), and HIDDEN is now a files word.
   - It is not needed. A read the operator asks outside an act tells nothing (`files.pyi:43`), so the show can simply be dropped.
4. **Played words take rung ids and stand in the turns.**
   - Every new life and every new chain without a source gets three rungs by `world` (files, bash, grant). Ids shift by 3 per such chain, for example `world.test.ts:774-777`, `world.test.ts:800` and `snapshots.test.ts:31`.
   - The feed would show three large python cards unless the TUI hides rungs whose `by` is `"world"`.
5. **The LEDGER line "npm bundles the builtin words (build.ts, like system.json)" gives the words a second home.**
   - Decision 6 puts the words in the crate (`include_str!`), and napi carries them.
   - I recommend that build.ts writes nothing new. See open question 1.

### 1. `bind/typescript/src/extension.ts` (new): types and helpers

This file makes no runtime import of `../index.cjs`, only `import type`. Export it both from `index.ts` and as a new subpath `"./extension"` in `bind/typescript/package.json`, so a `tui.ts` or `world.ts` can import it without loading the native addon.

Extension code in the cache cannot resolve `@furb/engine` at run time, so every helper also comes through the context. Extension files use only `import type`.

### Shared shapes

```ts
import type { Life, Act } from "../index.cjs";
import type { Fact, Turn } from "./types.js";
import type { LiveAct } from "./activity.js";
import type { FileChange } from "./changes.js";

export interface Call { verb: string; args?: unknown[]; kwargs?: Record<string, unknown> } // moves from ears.ts:22-26
export type Saying = [kind: string, about: string, ...words: unknown[]];               // moves from ears.ts:21
/** What a part does while it hears: the calls it yields are answered before it goes on, and it ends with a saying or nothing. */
export type Hearing<T = Saying | undefined> = Generator<Call, T, unknown>;
export interface Fault { is: string; args: unknown[] }
```

### World part (language neutral; a Python part matches it by the same names)

```ts
export interface WorldPart {
  /** The kinds of act this part does the work of. The World hands it the start of one; a start of a kind that no
   *  part names is closed with Refused("the World does no <kind>"). */
  readonly acts?: readonly string[];
  /** Every fact the core World does not consume (all but stand, clock, chance, keep, ask, start), in order. It may
   *  yield calls, and it may end with a saying: a done that answers a query ends the round for that fact. */
  hear?(fact: Fact): Hearing | undefined;
  /** The start of an act of its kinds, with the act as `get` gives it. */
  start?(act: Fact): Hearing | undefined;
  // Hooks of the TypeScript host alone; a Python part has none of these.
  /** The live value of its acts in Activity: at birth, and as a fact changes it. Pure and synchronous. */
  live?: {
    born?(act: LiveAct): unknown;
    hear?(fact: Fact, acts: ReadonlyMap<string, LiveAct>): LiveAct | undefined; // the act it changed
  };
  dispose?(): void | Promise<void>;
}
export type WorldExtension = (context: WorldContext) => WorldPart; // the default export of world.ts
```

- **Pending kinds.** The part's `acts` is the declaration. I recommend that the World also derives pending in a general way: add `LiveAct.started`, set when Activity hears a `start` fact. Then pending is an act that is not done, not paused, and is a prompt, a rung, or started.
  - `inspectRecord` then needs no part code.
  - A pending act whose part is gone is refused at resume. This is the new law of `engine.pyi:61`.

### World context

```ts
export interface WorldContext {
  readonly directory: string;          // the directory of the World
  readonly readOnly: boolean;          // an inspection: the part is heard by nothing and must start nothing
  readonly signal: AbortSignal;        // aborts at dispose
  /** yield* context.cwd(on): the cwd verb of that chain, or "" when the chain binds none. */
  cwd(on: string): Hearing<string>;
  resolve(here: string, path: string): string;           // against directory, then the cwd; refuses "://"
  speak(kind: string, about: string, ...words: unknown[]): void; // later, as the World, in order (world.ts:595-603)
  close(value: unknown, id: string): void;               // later, as the World
  change(change: FileChange): void;    // one file change for the Changes view (world.ts:310-312)
  refused(message: string): Fault;
  unwrapped: typeof unwrapped;
  files: { read(path: string): { path: string; content: string };      // world.ts:282-299 rules
           write(path: string, content: string): { path: string; content: string; before: string } };
  shell: { path: string;                                                 // shell.ts
           spawn(command: string, options: { cwd: string; merged: boolean; fed: boolean; timeout: number | null }):
             { child: ChildProcessWithoutNullStreams; stop(): void; late(): boolean } }; // world.ts:605-658
}
```

`cwd(on)` is `try { return String(yield {verb:"cwd", kwargs:{on}}) } catch { return "" }`. A raised call is thrown into the ear (`ears.ts:66-69`).

### TUI part (data first, no OpenTUI import)

```ts
export type Remote<T> = { [K in keyof T]: T[K] extends (...a: infer A) => infer R
  ? (...a: A) => Promise<R extends Act ? string : Awaited<R>> : T[K] };  // moves from tui/src/bridge.ts:26-30
export interface TuiContext {
  life: Remote<Life>;
  chain: string; directory: string;          // selected chain, its working directory
  acts: readonly LiveAct[];                  // the acts of the session
  call(verb: string, args?: unknown[], kwargs?: Record<string, unknown>): Promise<unknown>; // on = chain by default
  path(typed: string): string;               // ~ and relative, as Session.path (session.ts:348)
  projectFiles(): string[] | undefined;      // known once read
  notify(message: string): void; submit(message: string): Promise<void>;
  track(id: string): void; show(view: "feed" | "transcript" | "changes"): void;
  unwrapped: typeof unwrapped;
}
export interface TuiCommand {
  label: string; argument?: string; detail: string;           // as commands.ts rows
  keys?: string;                                              // shortcut label, as "!" for bash
  paths?: boolean;                                            // its value is a project path (app.ts:181)
  values?(context: TuiContext): { value: string; detail: string; more?: boolean }[];
  run(argument: string, context: TuiContext): void | Promise<void>;
}
export interface ActView {
  title?(act: LiveAct): string;             // heading name, the kind by default
  subject?(act: LiveAct): string;           // default String(act.words[0])
  preview?(act: LiveAct): { text: string; tail?: boolean } | undefined;
  work?: boolean;                           // default: act.started
  runsPaused?: boolean;                     // a pause does not hold it (bash)
  standing?: boolean;                       // a dot while it lives, "ended" once done (grant)
  hidden?: boolean;                         // no card point, no rewind point, no chain status (grant)
  ends?: string[];                          // header words that end it (bash: exited)
  fields?: string[];                        // names of its words in its details (grant)
  details?(act: LiveAct): { streams?: { name: string; content: string; failure?: boolean }[];
                            notes?: { text: string; tone?: "faint" | "success" | "danger" }[] } | undefined;
}
export interface SidebarPart { rows?: { name: string; value: string; tone?: "muted" | "text" }[];
                               meter?: { mark: number; tip: string }; show?: boolean }
export interface TuiPart {
  commands?: Record<string, TuiCommand>;
  prefixes?: Record<string, string>;        // "!" -> "bash"
  acts?: Record<string, ActView>;           // by kind
  quiet?: string[];                         // header words of paragraphs no card shows (grant: ledger)
  paths?: string[];                         // note header words whose detail is a path (files: read, write)
  sidebar?(view: { acts: readonly LiveAct[]; chain: string }): SidebarPart | undefined;
  prompting?(message: string, context: TuiContext): Promise<void>; // files: read each @ reference first
  dispose?(): void | Promise<void>;
}
export type TuiExtension = () => TuiPart | Promise<TuiPart>;       // the default export of tui.ts
```

### Instance helpers

```ts
export interface Instance { is: "instance"; class: { is: "class"; id: number; name: string; base: unknown };
                            value: unknown }
export function isInstance(value: unknown, name?: string): value is Instance;
export function unwrapped<T = unknown>(value: unknown): T;   // every instance, deep, as its fields
export function remade(instance: Instance, fields: Record<string, unknown>):
  { is: "instance"; class: number; fields: Record<string, unknown> };  // the way in (preamble.py:181)
```

- `types.ts:115` `display` shows `unmarked(unwrapped(value))`.
- Remove the unused `TextValue` at `types.ts:39-43`.

### 2. The generic World: what stays, what moves, what is refused

### Ear core (`ears.ts`)

**Stays:**
- `Ears`, at lines 39-82. `boot` passes the words through (see section 3).
- `speak` and `later`, at 127-156.
- stand, clock and chance with float marks, at 162-191, less read and write.
- keep, at 192.
- ask, with the mute and pause of two failures, at 193-219.
- `started()` for wait and prompt, at 254-274.

**Change:**
- `WorldRequest.kind` (4-19) drops Read, Write, Run, Feed and Slay.
- `WorldAdapter` takes `parts: WorldPart[]` and gets `send()` and the delivery chain from `world.ts:595-603`, so the handler path of `boot({world})` hosts parts too.

**New world ear dispatch** (replaces 157-251):
- stand, clock, chance, keep and ask are core.
- For `start`: `act = yield {verb:"get", args:[id]}`.
  - A wait or a prompt goes to `this.started(act)`.
  - Otherwise, the part whose `acts` names the kind handles it: `yield* part.start(act)`. A throw becomes `{verb:"close", args:[fault(e), id]}`.
  - Otherwise: `yield {verb:"close", args:[{is:"Refused", args:[`the World does no ${act[0]}`]}, id]}`.
- Any other fact goes to each part's `hear` in order. A saying that answers a query ends the round. In readOnly, parts hear nothing.

**Move to `bind/typescript/src/builtin/bash.ts`:**
- The bash branch at 220-238. It asks `cwd` through `context.cwd`, asks `merged` by `{verb:"ask", args:["merged", on, id]}`, and spawns.
- feed at 239, exited at 240, cancel and close with `covers` at 241-249, and `running` at 90.

**Move to `bind/typescript/src/builtin/files.ts`:**
- read and write at 162-191, with the new fact shape and plain answers.

### World class (`world.ts`)

**Stays:**
- Models, roster, actors and standing (159-229).
- Images (230-238) and `open` (240-263).
- Clock, chance, keep, ask and prompt in `handle`.
- Wait with deadlines (317-339).
- `learnKinds`, `momentary` and `hear` (366-405).
- `ask` for pi-ai (407-517), `prompt` and `answer` (519-569), `save` (571-581), `resume` (585-593), the rest of `dispose` (661-683), `inspectRecord` (694-706) and `Session`.

**Moves to `builtin/files.ts`** (using `context.files`, `context.resolve` and `context.change`):
- `handle` Read and Write (282-314) and `path()` (361-364).
- Their imports: `statSync`, `mkdirSync`, `writeFileSync` and `dirname` at lines 4-5.

**Moves to `builtin/bash.ts`:**
- `Command` and `Running` (107-119), `commands` (146), `feeds` (154).
- `handle` Run, Feed and Slay (340-355).
- `run()` (605-658). Its process-group kill goes to `shell.ts` as `spawnShell`, which the context exposes.
- The feed cleanup on done (393) becomes the bash part hearing `done`.
- The command loop in dispose (672-676) becomes `part.dispose()`.
- The imports `spawn`, `spawnSync` (1) and `shell` (25).

**Changes:**
- `PENDING` (49) goes. Pending is derived as in section 1. `open` (250-251) uses `act.started || prompt || rung`.
- The readOnly list (267-271) drops Read, Write and Run.
- `WorldOptions` gains `extensions?: Extension[]`: when unsaid, the builtins only. It is sync, deterministic, and keeps the 59 `new World(...)` sites of tests and scripts working.
- `WorldOptions` also gains `parts?: Record<string, WorldExtension>`. `open()` throws, and names the extension, when a resolved extension has a TS World part that neither the builtins nor `parts` hold.
- New `static async World.load(options)`: it resolves through napi when `extensions` is unsaid, runs `await loadWorldParts(...)`, and returns the World.
- `boot()` (716-735) becomes async over `World.load`, and hands `extensions.map(e => ({name, word}))` to the life.
- `activity` (132) becomes `new Activity(liveHooks, coreKinds + parts' acts)`.

**Activity (`activity.ts`):**
- kinds (36): `["chain","prompt","rung","wait"]` plus the parts' `acts`.
- The bash value at birth (107-113) becomes `live.born`.
- `merged` (54, 69, 133-134) and `out` (156-161) move to the bash `live.hear`. The bash part keeps merged in a `WeakMap<LiveAct, boolean>`, which a derive-again drops by itself.
- completed (136): an act that is done, is a prompt, a rung or started, and whose `by` is not `"world"`.
- Add `started` on a `start` fact.

**Extension answers are plain data:**
- files answers read and write with `{path, content}` and no `is`. `landed()` (`files.py:27-31`) makes the Text.
- bash answers no query. The bash act ear answers `merged` and the stdin and stdout doors (`bash.py:19-23`). The part only speaks `out` and `exited`. The word makes `Exit`.
- A refusal is a core Fault `{is, args}`.

### 3. napi surface after the crate exposes the extension API

**Goes from `index.d.cts`** (with their Rust in `ts.rs`):

| What goes | `index.d.cts` | `ts.rs` |
| --- | --- | --- |
| `TextValue` | 144-147 | 21-25 |
| `ExitValue` | 100-104 | 27-32 |
| `BashOptions` | 88-94 | 40-47 |
| `GrantOptions` | 106-110 | 49-54 |
| `Life.grant` | 45 | 328-338 |
| `Life.bash` | 46 | 340-369 |
| `Life.read` | 48 | 383-401 |
| `Life.write` | 49 | 403-414 |
| `Life.cwd` | 55 | 445-454 |
| `Life.cd` | 56 | 456-465 |
| `span`, `grep`, `differs` | 65-67 | 530-541 |

**Stays:** `Act`, root, raised, disposed, site, call, result, outcome, held, made, forget, prompt, rung, inspect, chain, wait, peek, get, turns, scope, clock, chance, gate, pause, wake, cancel, close, send, take, dispose, `RecordLock`, `decodeRecord`, `engineSource`, `onConsoleEnd`, `Inspection`, `Outcome`, `PromptOptions` and `RungOptions`.

**Comes** (new `src/binding/ts/extension.rs`, sync `#[napi]` functions over `crate::extension`):

```ts
export declare class Life {
  static boot(callback: (request: unknown[]) => unknown, names: string[], record?: unknown[] | null,
              words?: Array<{ name: string; word: string }> | null): Life   // ts.rs:131-153; the life plays them
}
export type ExtensionSpec = boolean | string | { path: string } | { git: string; ref?: string; path?: string }
                          | { npm: string; version?: string }
export interface Manifest { name: string; python?: string; world?: { ts?: string; py?: string }; tui?: string;
                            requires?: string[] }
export interface ConfigEntry { name: string; spec: ExtensionSpec; file: string }
export interface Extension { name: string; builtin: boolean; directory?: string; config?: string;
                             requires: string[]; word: string;            // the word the crate made (decision 5)
                             world?: { ts?: string; py?: string }; tui?: string }  // absolute paths
export interface ResolveOptions { home?: boolean; local?: boolean; update?: boolean }
export declare function configDirectory(): string
export declare function cacheDirectory(): string
export declare function readConfig(project?: string | null): ConfigEntry[]
export declare function fetchExtension(spec: ExtensionSpec, base: string, update?: boolean | null): string
export declare function readManifest(directory: string): Manifest
export declare function orderExtensions(extensions: Extension[]): Extension[]
export declare function wordOf(module: string): string
export declare function builtinExtensions(): Extension[]
export declare function resolveExtensions(project?: string | null, options?: ResolveOptions | null): Extension[]
```

- `Ears.boot` (`ears.ts:79-81`) forwards `words`. A readOnly World passes none, so an inspection plays nothing.
- `tui/src/preferences.ts:15-25` uses `configDirectory()`, so ui.json and config.json share one rule.
- `project.ts:6-17`: `furbDirectory` writes `.furb/.gitignore` as `*\n!config.json\n` whenever it is missing (decision 3).

### 4. The loader, and which process loads what

- **`bind/typescript/src/extensions.ts` (new):**
  - `loadWorldParts(extensions)` maps builtin names to static imports (`builtin/files.ts`, `builtin/bash.ts`; grant has no World part). For the others it runs `import(pathToFileURL(ext.world.ts).href)` and checks for a default function.
  - `loadTuiParts(extensions, builtin)` does the same with `ext.tui`.
  - It refuses two extensions that claim one act kind, command name or prefix, and it names both.
- **Worker (`tui/src/worker.ts:136-156`):** at `open`, it takes the extensions in this order:
  1. `EngineOptions.extensions` when given (screenshots).
  2. Else `builtinExtensions()` for the demo.
  3. Else `resolveExtensions(cwd)`.

  A git or npm fetch then blocks only the worker. It runs `await loadWorldParts`, then `new World({...given, extensions, parts})`, and replies `{root, extensions}`.
- **Main thread (`tui/src/bridge.ts:121-183`):** `openEngine` reads `{root, extensions}` from the open reply (164). It runs `world.parts = await loadTuiParts(extensions, builtinTui)`. `HostView` (48) gains `parts` and `extensions`. `Session` reads `world.parts`, so no `new Session(...)` call site changes. That matters: there are 12 of them across `demo.ts`, `workspaces.ts:307`, scripts and tests.
- **TUI parts load only in the main thread.** The worker must never import them (worker.ts:1).
- **Builtin TUI parts live in `tui/src/builtin/{files,bash,grant}.ts`**, since they use `tui/src/format.ts` (`dollars`, `share`). The builtin World parts live in `bind/typescript/src/builtin/`, since they serve any host.
- **npm package and the builtin words.** The crate carries the words (`include_str!`), and napi gives them through `builtinExtensions()`. build.ts (33-45 writes `system.json`) needs no second copy. If the owner still wants the files in the package, build.ts would copy `src/furb/builtin/*.py{,i}` into `bind/typescript/builtin/` and add that folder to `"files"`. That is a second home, so ask first (open question 1).

### 5. The TUI

### Remove

- `tui/src/extensions.ts` (whole file) and `tui/src/index.ts:1`, which re-exports its types.
- `cli.ts`: 10, `--extension` at 28, 74-88, 111, and `extensions` in the options at 131.
- `AppOptions.extensions` (`app.ts:157`) and its import (29).
- `/extension` in `commands.ts:40`, `app.ts:168`, `app.ts:181`, `app.ts:3262-3266` and `app.ts:3764-3767`.
- New `/extensions`: it lists each loaded extension, where it came from and its parts. `/extensions update` fetches again and says that a new session takes the change.

### Moves into the builtin TUI parts (with file:line)

**files** (`tui/src/builtin/files.ts`):
- `/read` (`commands.ts:21`; `session.ts:827-831`; values `app.ts:3758-3759`) runs the rung `read(...)`.
- `/cd` (`commands.ts:22`; `session.ts:832-834`; values `app.ts:3768-3775`).
- `paths: ["read","write"]` for the note references at `app.ts:1755`.
- `prompting` holds the @ reference reads from `session.ts:577-581`, which `session.ts:689` calls through the parts.

**bash** (`tui/src/builtin/bash.ts`):
- `/bash` (`commands.ts:20`; `session.ts:823-826`) calls `call("bash",[arg])`, then track and show feed.
- The `!` prefix (`session.ts:684`; key label `app.ts:3594`).
- `/feed` (`commands.ts:31`; `session.ts:853-861`, now `call("ask", ["write", chain, `${id}/stdin`, text])`; values `app.ts:3801-3804`).
- ActView for bash:
  - preview: the tail while running (`app.ts:2152-2156`).
  - details: streams, exit code, "input open", and a timeout that is not 600 (`app.ts:2321-2324`, 2348-2394).
  - `runsPaused` (`app.ts:2198`) and `ends: ["exited"]` (`conversation.ts:21`).

**grant** (`tui/src/builtin/grant.ts`):
- `/grant` and `/context` (`commands.ts:26-27`; `session.ts:758-774`; values `app.ts:3809-3818`) call `call("grant", [usd])` and `call("grant", [null, share])`, then check `outcome`.
- ActView for grant:
  - standing (`app.ts:2173-2176`), subject (`app.ts:2217-2223`) and fields (`app.ts:2329`).
  - hidden: `session.ts:359`, 618; `app.ts:2556`, 4516; `conversation.ts:60`.
- `quiet: ["ledger"]` (`conversation.ts:44`).
- sidebar: a Ceiling row, and a meter mark with the tip ", pauses at X%" (`app.ts:2586`, 2596, 2712-2716, 2730, 2738, 2772).

### Made general in core

- **Commands.** `commands.ts` loses rows 20-22, 26-27, 31 and 40. The palette, suggestions and help merge `commands` with the parts' commands (`app.ts:3613-3619`, 3658-3669, 3837-3844).
- **Values.** `valued` and `pathCommands` (`app.ts:161-181`) come from `TuiCommand.values` and `paths`. `argumentValues` (3727) asks the part for commands it does not know.
- **Palette.** The `beside` key `grant` (`app.ts:3643`) is re-keyed to a core command, for example `pause`.
- **Command dispatch.** `Session.command` (`session.ts:705`) falls back to the part commands before "Unknown command".
- **Prefixes.** `submit` checks the part prefixes where `session.ts:684` now checks `!`.
- **Working.** `working` (`session.ts:57`) is `!done && !paused && (kind prompt or rung, or view.work ?? act.started)`.
- **Refresh.** `factsChanged` (`session.ts:232-256`) refreshes on any fact whose name has no `@`, so it covers any kind an extension adds.
- **Doors and `/edit`.**
  - `/edit` reads the door through the core `ladder` query: `call("ask", ["ladder", chain, id])[1]` replaces `session.ts:847`.
  - Its submit writes by the rung `ask("ladder", "", id, content)` in place of `write(Text(...))` (`session.ts:676-683`). `/edit` then needs no extension.
  - The reads that follow a door (`app.ts:2436-2443`, 2453-2461, 4398-4402) use `call("read",[path])` and `unwrapped`, with no show. When the chain binds no `read`, they fall back to `ladder`.
- **Snapshots.**
  - `snapshots.ts:108` reads the directory by `call("cwd", [], {on})` and falls back to "".
  - The directory is invalidated with the turns; the cd and write cases at 50-55, 61 and 71-85 go.
  - A `ladder` fact with more than five items (a ladder given a word) invalidates turns, program and actor.
  - `row()` (18-31) cuts every string in `act.value` longer than TAIL, whatever the kind.
- **Played rungs.** Rungs with `by === "world"` stand as one compact "Extensions: files, bash, grant" note in `conversation.ts`, and are no point in `isPoint` (`app.ts:4513-4519`). They do not count in `completed`, so `workspaces.ts:279` marks no session unread at open.
- **Demo.** `demo.ts:60` becomes `life.call("grant", [2], {on: life.root})`. The demo World (`worker.ts:77-132`) is unchanged, but takes `builtinExtensions()` so the screenshots stay the same on every machine. Its first answer's `read` and `bash` work through the played words.
- **Scripts.** `tui/script/animation.ts:90` uses `working`. `tui/script/screenshots.ts` loses its `Extensions` instance (10, 71-83, 506). `life.bash` at 229 and 344 becomes `life.call`. Shot 42 (448-452) loads `tui/examples/project-summary/` (package.json `furb` manifest plus `tui.ts`) through `EngineOptions.extensions`. Replace `tui/examples/project-summary.ts` with that folder.

### Tests that change

**bind/typescript/test/life.test.ts:**
- The fake Read and Write handler at 44-50 goes. The fake World becomes a part in `open()`.
- Lines 113-129 and 131-143 (`write(Text)`, `cwd(fork)`), 146-154 (`cwd` becomes `clock`), 165-189 (`{'is':'name','name':'bash'}` becomes `wait`), 198 and 247.
- Each call goes through `life.call(verb, args, {on})`.
- `boot` gets the builtin words.

**bind/typescript/test/world.test.ts:**
- Lines 233-259, 441-460, 462-486, 520-541 (host ear: the new read shape and a plain answer), 543-575, 611-637, 734-761 (`Text(path, 'kept')` becomes a plain dict), 763-782 (ids plus 3), 784-800 (`rung3` plus 3), 821-858, 842-858, 860-879, 881-900 and 902-914 (`life.cwd()` becomes `life.clock()`), 1023-1036 and 1038-1062.
- `boot` is now awaited at 235, 316, 331, 419, 824, 844, 923, 944, 1002, 1026 and 1045.
- `test/smoke.ts:7` changes the same way.

**New `bind/typescript/test/extension.test.ts`:**
- The config merge: home, then local override, then `false`.
- A requirement refused when bash is on and files is off.
- A path extension with a `world.ts` that hears, yields a call, speaks later, and adds a live value.
- The refusal "the World does no x".
- `unwrapped` and `remade` over Text and a nested Exit.
- Words played once after boot, and at chain birth, and not again in a later life.
- `furbDirectory` writes its `.gitignore`.

**tui/test:**
- `app.test.ts`: 52 and 67, 82 ("/grant " comes from the grant part), 169, 252-285, 498-515 (the edit rung is now `ask("ladder", ...)`) and 517+.
- `session.test.ts`: 16-33 (prompting), 61-85, 194-215, 275-326 (drop `/extension`) and 328-343.
- `views.test.ts`: 65-99, 101-158, 160-180 and 182-207.
- `features.test.ts`: 274-360. `!` goes through the bash part. The extension is loaded from a temp `FURB_CONFIG_DIR` config plus a local `.furb/config.json`.
- `snapshots.test.ts`: 10-61 (`life.write` to stdin becomes an `ask("write", ...)` call), 92 and 31 (+3).
- `workspaces.test.ts`: 36 and 354-372 (config.json is tracked).
- `composer.test.ts`: 85-89 and 422-453.
- New `tui/test/extensions.test.ts`:
  - A part command shows in the palette and the suggestions, and a part prefix works.
  - A hidden kind is hidden.
  - With bash off in `.furb/config.json`, `!ls` is text and `/bash` is unknown.
  - A refused start shows its reason.

**Isolation:** a root `bunfig.toml` with `[test] preload` pointing to a file that sets `FURB_CONFIG_DIR` and `XDG_CACHE_HOME` to temp directories, so no developer config leaks in.

### Docs

- `tui/README.md`:
  - `script/docs.ts:13-19` also lists the builtin part commands, then run `bun run docs`.
  - Rewrite 150-163 (the regenerated command table), 202 (`/extension` in the `~` list) and 208-212 (extension loading).
- `bind/typescript/README.md`:
  - The operations table loses Read, Write, Run, Feed and Slay.
  - "Text and Exit carry `is`" becomes the instance form and `unwrapped`.
  - The example uses `await boot` and `life.clock()`.
  - New sections on the World and TUI parts and `resolveExtensions`.
- `docs/tui.md`: the "Extensions" section (237-241) says how to enable an extension in config.json.
- Screenshots: run `bun run screenshots` and `bun run animation` again, and read each capture. 03-transcript now holds the played words. 41-slash-suggestions shows `/extensions`. 42 is the example folder.

### 6. Risks, open questions, and the order of commits

### Risks

- **Node 22 cannot import `.ts` without a flag.** Allow `.js` in the `ts` field, or ask npm extensions to ship built JS.
- **External code importing `@furb/engine` at run time will not resolve from the cache.** Use `import type` and context helpers only. Say so in docs/extensions.md.
- **The crate's `extension.rs` needs `serde_json`,** which is optional under `typescript` and `python` (`Cargo.toml:21`). Also, `include_str!` of `src/furb/builtin/*.py` must be in the crate package.
- **A fetch blocks the worker.** The first open of a project with a new git extension is slow.
- **Turning an extension off leaves its verbs bound in old records.** The World then refuses their acts. This is coherent, but document it.
- **The record gains three rungs per new life and chain.**
  - "Every fact" counts in tests change.
  - `life.test.ts:141`: a second life keeps nothing only if the crate plays nothing that the program already holds.
- **Decision 2's known limit:** a chain made and prompted in one word gets its first ask before its rungs.

### Open questions

1. The build.ts line of the LEDGER (see section 0). Is it dropped?
2. May `python` be optional in a manifest, for TUI-only extensions such as project-summary?
3. Should the preamble `called` default `on` to the root, so that `life.call("bash", [...])` works without `on`?
4. Pending derived from `start` facts, in place of declared kinds. I recommend it.

### Order of commits

The gates are `bun run build`, `bun run check`, `bun run lint`, then `bun test bind/typescript/test tui/test`. `check` and `lint` run on bun 1.3.11. The tui tests need bun 1.4.2 or later: install it (`curl -fsSL https://bun.sh/install | bash -s bun-v1.4.2`, through the proxy), or leave tui/test to the CI in `gates.yml`, which pins 1.4.2 at line 149.

1. **`extension.ts` types, the instance helpers, and `display` over instances.** Additive; it can land before step 3.
   - Gate: check, lint, test bind and tui.
2. **After crate step 3:**
   - napi: `ts.rs` removals plus `ts/extension.rs`, then build.
   - The generic World, the builtin World parts, Activity live hooks and `started`, and `World.load` with async `boot`.
   - `furbDirectory` and `preferences.ts`.
   - The bind tests.
   - A mechanical TUI switch to `life.call` (session.ts, app.ts, snapshots.ts, demo.ts), the hidden world rungs, the tui tests that change ids, and the bunfig preload.
   - Gate: build, check, lint, test bind and tui.
3. **TUI parts:**
   - The loader in the worker and bridge, and `tui/src/builtin/*`.
   - Remove `extensions.ts`, `--extension` and `/extension`; add `/extensions`.
   - Make app.ts, session.ts, conversation.ts, snapshots.ts and commands.ts general.
   - New tui tests.
   - Gate: check, lint, test.
4. **Docs:**
   - Both READMEs with `bun run docs`, `docs/tui.md`, the example extension folder, screenshots and animation.
   - Gate: lint; `bun run docs` leaves no diff; read each capture.

### Critical Files for Implementation
- /home/user/furb/bind/typescript/src/ears.ts
- /home/user/furb/bind/typescript/src/world.ts
- /home/user/furb/src/binding/ts.rs
- /home/user/furb/tui/src/app.ts
- /home/user/furb/tui/src/session.ts
## Part 3: the skills extension, the proofs, the documents and the releases

### 0. What this plan needs first, and six risks it found

The plan depends on three earlier steps of LEDGER.md:
- **Step 2** (LEDGER.md:128-132): the builtins become modules of form (b), and the crate has the word rule.
- **Step 3** (LEDGER.md:133-148): the crate module `src/extension.rs` does the config, the cache, the fetch, the manifest, the order and the play rule, and `furb_monty` exposes it.
- **Step 4** (LEDGER.md:149-152): `@furb/engine` exports the types of a World part and of a TUI part. The TS host fetches through napi and imports the parts dynamically.

You can start `skills.py`, `skills.pyi` and their python suite as soon as step 2 lands. `world.ts`, `tui.ts` and the bun tests wait for step 4.

Today `src/furb/builtin/*.py` are still form (a), with no imports. `uv run ty check` fails on them now (for example, `pause` and `act` are unresolved in grant.py). Step 2 fixes that.

These six risks apply to the skills extension. The implementer must handle them.

**R1. The no-shadow law and the imports of form (b).**
- `test/test_hygiene.py:243-253` runs `shadows()` on the file as it is on disk. In form (b), `from furb.engine import ask` binds `ask` at module level, and the law flags it.
- Fix: the law must read the word that the crate makes, with the imports blanked, and not the module.
- `from dataclasses import dataclass` is not blanked. It is bound again in files.py, bash.py and skills.py, so the law flags it too. The law must accept an import that binds the same name from the same module.

**R2. ty does not narrow the answer of `ask`.** `ask` gives `tuple[Question, object]` (engine.pyi:97). You cannot iterate over `object` or subscript it. Narrow it with `isinstance(list)` and `isinstance(dict)`, as the sketch below does, or with `match` as `files.landed` does (files.py:27-31).

**R3. ruff without per-file ignores.** `raise Refused(f"...")` fails EM102 and TRY003. Use the fix that step 2 picks for `Text.edit` (files.py:83), and use it in skills.py too.

**R4. Windows fetch.**
- Rust `Command::new("npm")` does not find `npm.cmd`. Call `npm.cmd` on Windows.
- The Git Bash PATH puts GNU tar first, and GNU tar reads `C:` as a remote host. Extract the npm tarball in Rust instead. `flate2` is already in Cargo.lock:570; add the `tar` crate.
- A git URL to a local folder must be `file:///C:/...`.
- A clone with `core.autocrlf=true` gives CRLF. The word rule must turn CRLF into LF. Otherwise the word differs from machine to machine, the play rule plays it again on a record, and the record drifts.

**R5. Rust tests cannot change the environment safely.** `std::env::set_var` is unsafe in edition 2024 (Cargo.toml:4), and the tests run in parallel. The extension API must take the config dir, the project dir and the cache dir as parameters. A separate function computes the defaults from XDG, `FURB_CONFIG_DIR`, `APPDATA` and `LOCALAPPDATA`.

**R6. The harness covers only `test/` itself.**
- `pytest_generate_tests` runs a test on both engines only when `path.parent == HERE` (test/conftest.py:640).
- `swapped()` changes only modules whose file starts with `HERE` (test/conftest.py:627).
- Both must also cover `test/<builtin>/` (step 2 needs this too) and `extensions/*/test/`.

### A. The official skills extension

### A.1 File tree

```
extensions/skills/
  package.json      manifest and npm package
  README.md         what it does, how to load it, the commands table (written by bun run docs)
  skills.py         the python part, form (b)
  skills.pyi        its contract
  world.ts          the World part
  tui.ts            the TUI part
  tsconfig.json     extends ../../tsconfig.json, adds "erasableSyntaxOnly": true
  test/
    conftest.py         re-exports the harness for this folder
    test_skills.py      sentences of skills
    test_skills_shape.py  sentences of type Skills (the file_of rule, test_hygiene.py:97-105)
    test_skill.py       sentences of skill
    test_skill_shape.py sentences of class Skill
    world.test.ts       the finder and the parser
    tui.test.ts         the two commands
```

### A.2 package.json

```json
{
  "name": "@furb/skills",
  "version": "0.2.0",
  "description": "Skills for furb: find SKILL.md files, and read one into a chain.",
  "type": "module",
  "license": "AGPL-3.0-only",
  "repository": { "type": "git", "url": "git+https://github.com/uael/furb.git", "directory": "extensions/skills" },
  "keywords": ["furb", "furb-extension", "skills"],
  "exports": { "./world": "./world.ts", "./tui": "./tui.ts", "./package.json": "./package.json" },
  "files": ["skills.py", "skills.pyi", "world.ts", "tui.ts", "README.md"],
  "furb": {
    "name": "skills",
    "python": "skills.py",
    "world": { "ts": "world.ts" },
    "tui": "tui.ts",
    "requires": ["files"]
  },
  "peerDependencies": { "@furb/engine": ">=0.2.0 <0.3.0" },
  "peerDependenciesMeta": { "@furb/engine": { "optional": true } },
  "devDependencies": { "@furb/engine": "workspace:*" },
  "publishConfig": { "access": "public" },
  "engines": { "bun": ">=1.4.2", "node": ">=22.18" }
}
```

Why each part:
- **No `dependencies` and no scripts.** The crate then runs no `npm install` in the cache (LEDGER.md:68-70). `npm pack` runs no lifecycle script, and the host passes `--ignore-scripts` anyway.
- **The peer dependency is only for types.** `world.ts` and `tui.ts` use only `import type` from `@furb/engine`. The cache folder has no `node_modules`, so every runtime helper comes through the context argument. The peer is optional, so npm 7+ does not install it by itself.
- **The devDependency is needed.** The bun lockfile uses the isolated linker (`tui/node_modules/@furb` is a link; there is no root `node_modules/@furb`), so the package needs `workspace:*` to resolve the types. Publish with `bun pm pack`, which rewrites `workspace:*`, then `npm publish <tgz>`.
- **Erasable syntax only.** Node 22.18 and later imports `.ts` files with type stripping only outside `node_modules`, and the cache is outside `node_modules`. Bun imports them in any case.

Root package.json changes:
- Workspaces (package.json:4-7): add `"extensions/*"`.
- `test` (package.json:16): `bun test bind/typescript/test tui/test extensions`.
- `format` and `lint` (package.json:17-18): add `extensions`.
- `check` (package.json:15): `tsc --noEmit && tsc --noEmit -p extensions/skills`.
- tsconfig.json:14 `include`: add `"extensions/**/*.ts"`.
- Commit `bun.lock` again, because CI runs `bun install --frozen-lockfile` (gates.yml:152).

### A.3 skills.py (form (b), about 25 lines)

This word stands in the turns of every chain without a source, so keep it short. The host blanks each `from furb...` line into an empty line, which keeps every line number (decision 5, LEDGER.md:42-51).

```python
from dataclasses import dataclass

from furb.builtin.files import HEAD, HIDDEN, Text, read
from furb.engine import Refused, Show, ask, tell


def skills(show: Show = HEAD, on: str = "") -> list[Skill]:
  heard = ask("skills", on)[1]
  found = [Skill(x["name"], x["description"], x["path"]) for x in heard if isinstance(x, dict)] if isinstance(heard, list) else []
  if show is not HIDDEN:
    tell("skills", "", ("skills", "".join(f"{x.name}: {x.description}\n" for x in found), show))
  return found


def skill(name: str, show: Show = HEAD, on: str = "") -> Text:
  found = next((x for x in skills(HIDDEN, on) if x.name == name), None)
  if found is None:
    raise Refused(f"no skill {name}")   # R3: the form that step 2 picks for Text.edit
  return read(found.path, show, on)


@dataclass
class Skill:
  name: str
  description: str
  path: str


skills()
```

Design choices:
- **The World answers with plain data.** The `skills` query is answered with a list of `{name, description, path}` tables, and the verb makes the `Skill` objects. So the record holds no class of an extension (decision 1, LEDGER.md:16-19; engine.pyi:62).
- **With no World part, there are no skills.** A question that nobody answers is answered with nothing (engine.pyi:102), so `skills()` gives `[]`. That is what the Python host gives, because it loads no external World part yet (decision 4).
- **`skills()` tells the list as a Showing** (engine.pyi:468-471). `shown()` (engine.py:355-362) tells only lines that the model has not seen, which keeps the law "Nothing is told twice". `HIDDEN` tells nothing.
- **`skill(name)` goes through `read`.** The body gets the seen-line logic, the show, a door check and the `#read` paragraph at no extra cost. The World part answers only `skills`, and the files World part serves the read. That is why the manifest requires `files`.
- **The last line `skills()` runs when the host plays the word.** Every new chain without a source then tells its skills before its first ask, as a system prompt lists skills.
  - The answer of a query asked from a run is kept in the record, so a later life replays it.
  - Limit (decision 2, LEDGER.md:25-26): a model that opens a chain and prompts it in the same word gets its first ask before this paragraph.
  - **Owner choice:** keep this line, or drop it and let the model call `skills()` itself.
- **Names are safe.** None of these names are module names of engine.py (I listed them with ast) or names of the builtin words: `skills`, `skill`, `Skill`, `heard`, `found`, `x`, `name`, `show`, `on`.

### A.4 skills.pyi (same style as the builtins: one sentence per line, Simplified Technical English, empty module docstring)

```python
from dataclasses import dataclass
from typing import Literal

from furb.builtin.files import HEAD, Text
from furb.engine import Show

def skills(show: Show = HEAD, on: str = "") -> list[Skill]:
  """The skills that the World finds for a chain: the name, the description and the path of each, which a model reads before it chooses one.
  skills asks the World a skills question on its chain, and gives a Skill for each table of the answer, in the order of the answer.
  A World that answers no skills question gives no skill.
  A skills question that the World refuses raises Refused in the caller.
  skills tells one line for each skill, with its name and its description, by the lines the model has not seen.
  A skills call with a hidden show tells nothing.
  The word of the extension calls skills once, so a chain without a source tells its skills before its first ask.
  """

def skill(name: str, show: Show = HEAD, on: str = "") -> Text:
  """A skill read into a chain: the text of its SKILL.md file, which the chain tells as a read of that file.
  skill finds the skill of that name among the skills that the World finds, and tells no list of them.
  skill reads the path of that skill with its show, so its lines stand in the turns as the lines of any read.
  skill gives the Text of the SKILL.md file.
  A skill of a name that no skill has raises Refused in the caller.
  """

@dataclass
class Skill:
  """A skill that the World found: its name, what it is for, and the path of its SKILL.md file.
  The World answers each skill as a table of plain data, and skills makes the Skill of it, so the record holds no Skill.
  """
  name: str
  description: str
  path: str

type Skills = tuple[Literal["skills"], str, str, str]
"""A skills is the question of the skills of a chain, which carries no word, and which the World answers with a list of tables, each with a name, a description and a path."""
```

That is 15 sentences, so the suite has 15 tests. The rules of the World part (the roots, the frontmatter, the order) are not python sentences. They go in README.md and docs/extensions.md, and the bun tests prove them.

### A.5 The python suite, the harness, and the hygiene laws

**Where the tests live:** `extensions/skills/test/`, outside the npm `files`. This keeps the extension self-contained.

**pyproject.toml changes:**
- `testpaths` (pyproject.toml:133): `["test", "extensions"]`. Keep `test` first, so the root conftest loads first and its `sys.modules.setdefault("conftest", ...)` (test/conftest.py:669) wins.
- `[tool.ty.src] include` (pyproject.toml:110): add `"extensions"`.
- per-file-ignores: add `"extensions/*/test/**"` with the same list as `"test/**"` (pyproject.toml:74). Give the parts themselves (`extensions/*/*.py`) no ignores.
- Coverage needs no change. `--cov=furb --cov=furb_monty` (pyproject.toml:136) does not measure `extensions/`, and a word runs through exec anyway.

**`extensions/skills/test/conftest.py`** is one statement:

```python
from conftest import engine_of, py, pytest_generate_tests, sand  # noqa: F401  the harness of test/conftest.py
```

- With `pythonpath = [".", "test"]` (pyproject.toml:134), `import conftest` gives test/conftest.py. ty resolves `conftest` the same way today: test/outside/test_monty.py:18 imports `Sand` from it, although test/outside has its own conftest.py.
- A hook or fixture of a conftest reaches only its own folder, so the extension needs this re-export.

**test/conftest.py changes:**
1. **A list of suite folders.** Add `ROOT = HERE.parent` and `SUITES = {HERE, *(HERE / n for n in BUILTIN), *(ROOT / "extensions").glob("*/test")}`.
   - `pytest_generate_tests` (test/conftest.py:637-641) parametrizes when `path.parent in SUITES and path.name != "test_hygiene.py"`.
   - `swapped()` (test/conftest.py:627) changes a module whose file is under any folder in `SUITES`.
2. **A generic answer for extension questions.** Add a `Sand` field `answers: dict[str, Callable[[tuple], object]]`. Add an arm before the `clock` arm (test/conftest.py:216): `case (kind, qid, *_) if kind in self.answers and engine.question(a): yield "done", qid, self.answers[kind](a)`.
   - It is generic: it serves any extension that the World answers with plain data (engine.pyi:62).
   - `Dead` (test/conftest.py:224-242) already refuses it, which serves the "refuses" sentence.
3. **Make the words with the real loader.** Add `def words(*names)`, which gives the words in `requires` order. It goes through the crate loader of `furb_monty` with a config of `{"skills": str(ROOT / "extensions/skills")}`. The suite then plays exactly what a host plays. Step 2 already routes `BUILTIN` (test/conftest.py:35-37) through the crate's word rule.

**Test patterns:**
- `sand = Sand(words=words("skills"), answers={"skills": lambda a: [{"name": "x", "description": "d", "path": "/p/.furb/skills/x/SKILL.md"}]}, files={"/p/.furb/skills/x/SKILL.md": "---\nname: x\n---\nbody\n"})`, then `log, root = life(sand)`.
- Call through `Bound(root).skill("x")` or `verb("skill", root)("x")` (test/conftest.py:283-321).
- Assert with `of(engine.turns(on=root), "skills")`, `heads`, `engine.peek`, and `sand.calls` (a read of the absolute path).
- Rung ids: the extension rungs take `rung1..` in the order `files`, then `skills`.

**test/test_hygiene.py changes:**
- CONTRACTS (test_hygiene.py:28) grows:
  ```python
  ROOT = PYI.parents[2]
  CONTRACTS = {PYI: TESTS, **{b: TESTS / b.stem for b in sorted(BUILTIN.glob("*.pyi"))},
               **{e: e.parent / "test" for e in sorted((ROOT / "extensions").glob("*/*.pyi"))}}
  ```
  Also update the module docstring (test_hygiene.py:1-12).
- Laws 1 to 4 then apply as they are. `file_of` already sends `Skill` and `Skills` to `_shape` files, because the lower-case names are taken by the functions.
- The no-shadow law (test_hygiene.py:243-253) grows to cover `extensions/*/<manifest python>`. Its outer set is the engine names, the builtin words and the words of `requires`. Apply it to the word, not the file (R1).
- A new law, `test_every_extension_has_a_manifest_a_contract_and_a_suite`: each `extensions/*/package.json` names `furb.python`, a `.pyi` stands beside it, and a `test/` folder exists.

### A.6 world.ts (answers the skills question with plain data)

The planned part shape is in LEDGER.md:71-75 and decision 4. The file:
- `import type { Fact, WorldContext, WorldPart } from "@furb/engine/extension"`, and runtime imports from `node:fs` and `node:path` only.
- `export function roots(directory, configDirectory)`, in this order:
  1. `<project>/.furb/skills`
  2. `<project>/.claude/skills`
  3. `<configDirectory>/skills`

  This follows decision 3: local wins over home. The project is the World's directory (`context.directory`), not the cwd of a chain, because `.furb` stands there. The config dir is the one that the crate resolves: `FURB_CONFIG_DIR`, then `$XDG_CONFIG_HOME/furb`, then `%APPDATA%\furb`.
- `export function frontmatter(text): { name?: string; description?: string }`:
  - The file must start with a `---` line. Read up to the next `---` line, and accept CRLF.
  - Keys at column 0. The values of `name` and `description` may be plain, in single or double quotes, or a block `|` or `>` (folded joins lines with a space).
  - Ignore other keys, for example `metadata:` and `license:` (seen in .claude/skills/*/SKILL.md).
  - Take no YAML dependency, so the package keeps no runtime deps.
- `export function found(roots): { name; description; path }[]`:
  - Each direct subfolder that holds `SKILL.md` gives one skill. Follow symlinks. Skip a missing or unreadable root and a folder with no SKILL.md.
  - The name is the frontmatter name, or else the folder name. The description is the frontmatter description, or else `""`. The path is the absolute path of SKILL.md.
  - When one name appears twice, the first root wins. Sort the answer by name.
  - Read the disk with synchronous calls, because a query is answered now (engine.pyi:97-104). Keep no cache, so a new skill shows at the next question.
- `export default function world(context): WorldPart`: a part that hears a fact. For `["skills", qid, by, on]` it gives the saying `["done", qid, found(roots(...))]`. It passes every other fact on.

### A.7 tui.ts (the /skills and /skill commands)

The default export gives `{ commands: { skills: {...}, skill: {...} } }`, typed with the TUI part types of `@furb/engine/extension`.
- **`/skills`** (label "List skills", no argument):
  - `await context.ask("skills", context.chain)`. A query of the host outside a rung enters no record and tells nothing (bind/typescript/README.md:130-131).
  - Show a picker of `name: description`. A pick runs the same thing as `/skill <name>`. A context with no picker gets a notice with the list.
- **`/skill <name>`** (label "Load skill"): `await context.rung(\`skill(${JSON.stringify(name)})\`)` on the chain on screen. It is an operator rung, so the gate reads it, and the SKILL.md lines stand in the transcript for the next ask. A JSON string is a valid python string literal.
- Optional: completion of the name from the same `ask`.

Step 4 must remove the old extension notion:
- tui/src/extensions.ts, tui/examples/project-summary.ts, `extension` in commands.ts:40.
- In app.ts: lines 29, 157, 168, 181, 3261-3270, 3613, 3764, 3839.
- In cli.ts: lines 10, 28, 74-88, 111, 131; tui/src/index.ts:1.
- Tests: features.test.ts:274-357, session.test.ts:275-298.
- Screenshots: screenshots.ts:10, 71-83, 448-452, 506.

The gallery capture `42-extension-command` becomes `/skills`, with skills loaded by path from the gallery's config.

### A.8 The bun tests of the extension

**`extensions/skills/test/world.test.ts`** makes temp folders with `mkdtemp` and tests:
- each root alone, and the three roots together (the first root wins);
- a folder with no SKILL.md, and a root that is missing;
- the frontmatter: plain, quoted, folded `>`, literal `|`, CRLF, no frontmatter (folder name), no description;
- the sort by name;
- that the part answers `skills` and nothing else.

**`extensions/skills/test/tui.test.ts`** uses a fake context and tests that `/skill x` writes the rung `skill("x")`, and that `/skills` asks `skills` on the chain and offers each skill.

### B. Proving loading by path, git and npm

### B.1 Rust: `src/extension.test.rs` (the crate owns the fetch, decision 6)

Every test passes its dirs explicitly (R5) and uses the real `concat!(env!("CARGO_MANIFEST_DIR"), "/extensions/skills")`.
- **Config:**
  - a path entry resolves against the folder of its config file, and `~` expands;
  - a local `.furb/config.json` overrides the home config by name;
  - a manifest name that is not equal to its key is refused.
- **Order:** `files`, `bash` and `grant` come first. `skills` comes after `files`. With `"files": false`, `skills` is refused because its requirement is off.
- **Word rule:**
  - applied to skills.py, it keeps the line count, and each line of a `from furb...` statement (a parenthesized import over several lines too) is empty;
  - a word of form (a) comes back as it is;
  - CRLF becomes LF (R4).
- **Path:** `{"skills": "<repo>/extensions/skills"}` gives the manifest, the word, and the absolute paths of world.ts and tui.ts.
- **Git (local bare repo):**
  - Set up in a temp dir: `git init --bare remote.git`, then `git init work`, copy the skills files to `work/extensions/skills`, commit, tag `v1`, push.
  - Load `{"git": "file:///<tmp>/remote.git", "path": "extensions/skills"}` and `{..., "ref": "v1"}`.
  - Assert the clone lands in `<cache>/extensions/git/<sha256(url#ref)>/`.
  - Delete `remote.git` and load again: it must still load, which proves that it is fetched once.
  - Run every git call with `GIT_CONFIG_NOSYSTEM=1`, `GIT_CONFIG_GLOBAL=<empty temp file>` and `-c init.defaultBranch=main -c user.name=t -c user.email=t@t -c commit.gpgsign=false -c core.autocrlf=false`. Runners differ in identity and in signing.
- **npm (local tarball):**
  - `npm pack <repo>/extensions/skills --pack-destination <tmp> --json --ignore-scripts` gives `furb-skills-0.2.0.tgz`.
  - Load `{"npm": "<tmp>/furb-skills-0.2.0.tgz"}`. The crate runs `npm pack <spec> --json --ignore-scripts` and extracts in Rust (R4).
  - It lands in `<cache>/extensions/npm/@furb/skills@0.2.0/` (the scope is a nested folder), without the `package/` prefix.
  - Set `npm_config_cache=<tmp>/npm`, `npm_config_update_notifier=false` and `npm_config_fund=false`. Nothing goes to the network.
- **Cross-check:** path, git and npm give the same manifest and byte-identical words.
- **Play rule (in life.test.rs):** the `Life` plays skills once on the root after boot and at the birth of each chain without a source. It plays nothing during replay.

### B.2 Bun: `bind/typescript/test/extensions.test.ts` (the part that is truly per language)

- For each of path, bare git repo and npm tarball (made as above):
  - write `<configDir>/config.json` and boot a World with that config dir and a temp cache dir;
  - assert that the host imports world.ts and tui.ts from the resolved folder, that the root plays the skills word, that `life.ask("skills", root)` answers the SKILL.md of a temp `.furb/skills/x/`, and that the TUI part lists `skills` and `skill`.
- A `.claude/skills` fixture under the project proves the second root end to end.
- Also: `furbDirectory` writes `.furb/.gitignore` with `*` and `!config.json`, whenever that file is missing (decision 3).

### B.3 Python

- The skills suite in `extensions/skills/test/` runs on both engines (A.5).
- `test/outside/test_extensions.py` tests the furb_monty extension API from python. By path: the words come in order (files, bash, grant, skills) and the skills word is blanked. One git test is optional, since the crate proves the fetch. The Python host plays skills, and with no World part `skills()` gives `[]`. Document that.

### B.4 CI (.github/workflows/gates.yml)

- Where git and npm are used:
  - The hooks job (gates.yml:23-56, ubuntu) runs `cargo test` through pre-commit (.pre-commit-config.yaml:55-60).
  - The typescript job (gates.yml:110-162) runs `cargo test --features typescript` (gates.yml:155) and bun on Linux, macOS and Windows.
- All three runner images carry git and Node with npm. Pin Node anyway: add `actions/setup-node` (pinned by SHA, `node-version: 22`) before `cargo test` in the hooks job and the typescript job, so the npm version, and so `npm pack --json`, stay fixed.
- The suite job (gates.yml:81-108) needs nothing new, unless a python test calls git or npm.
- gates.yml:162: `bun test bind/typescript/test tui/test extensions`.
- gates.yml:159: `git diff --exit-code -- tui/README.md extensions/*/README.md`, because `bun run docs` now also writes the commands table of each extension README from its tui.ts.
- .pre-commit-config.yaml:59, the cargo-test `types_or`: add `json` and `ts`, so a change of extensions/skills runs the fetch tests.
- Windows: see R4 (`npm.cmd`, extraction in Rust, `file:///C:/` URLs, LF words). The job shell is bash (gates.yml:121-123), but a Rust `Command` does not use it.

### C. Step 6: documents and gates

### C.1 docs/extensions.md (new), section by section

1. **What an extension is.** Three parts: a python word played as a rung, a World part, a TUI part. The builtins `files`, `bash` and `grant` are extensions that are on unless you turn them off. The engine knows no extension.
2. **Where furb looks.**
   - Home config: `$XDG_CONFIG_HOME/furb/config.json`, or the `FURB_CONFIG_DIR` override, or `%APPDATA%\furb` on Windows.
   - Local config: `<project>/.furb/config.json` overrides the home config by name.
   - `.furb/.gitignore` holds `*` and `!config.json`.
   - The cache: `$XDG_CACHE_HOME/furb`, `~/.cache/furb`, or `%LOCALAPPDATA%\furb`.
3. **The config format.** The entries are `false`, `true`, a path, `{path}`, `{git, ref?, path?}` and `{npm, version?}`. Give examples:
   - `{"skills": "../furb/extensions/skills"}`
   - `{"skills": {"git": "https://github.com/uael/furb", "path": "extensions/skills"}}`
   - `{"skills": {"npm": "@furb/skills"}}`
   - `{"bash": false}`
4. **The cache.** The layout of git (`sha256(url#ref)`, subfolder by `path`) and of npm (`<name>@<version>`). Fetched once. Say how to refresh it: the update command that step 3 or 4 names.
5. **The manifest.** The `furb` field (name, python, world {ts, py later}, tui, requires). The name must equal the key. The order: builtins, then home, then local, sorted by `requires`, and an extension whose requirement is off is refused.
6. **The python part.**
   - The two forms, (a) and (b).
   - The word rule, with the full reason from decision 5 (LEDGER.md:42-51).
   - When it plays: after boot on every chain without a source whose program lacks it, at each chain birth, as the World, and never during replay. State the known limit.
   - What a word may use. The plain-data rule.
   - Its `.pyi` contract and its suite folder. The hygiene laws that apply.
7. **The World part.**
   - The TS shape: a context gives a part; it hears a fact, yields calls, returns a saying, and speaks later as the World.
   - Pending kinds, `Activity`, `world.change`.
   - An unknown start is closed with `Refused("the World does no <kind>")` (engine.pyi:61).
   - World parts in python come later (decision 4).
   - Only erasable TypeScript, and only `import type` from `@furb/engine`.
8. **The TUI part.** Commands, how acts show, sidebar rows, a prefix.
9. **Walkthrough: the skills extension.** The tree, the manifest, the word, the World part, the TUI part, the tests.
10. **Publishing an extension.** npm (`files`, peer `@furb/engine`, `publishConfig.access`, `bun pm pack` then `npm publish`) or git.
11. **The names of the extension system.** extension, builtin, manifest, word (of a module), play, config, cache.
12. **Trust.** A World part and a TUI part run with the rights of the user. The python part runs in the sandbox. Fetches run with `--ignore-scripts`.

### C.2 CLAUDE.md (root)

- :3-6: add that the builtins live in `src/furb/builtin/`, official extensions in `extensions/`, the loader in `src/extension.rs`, and the guide in `docs/extensions.md`.
- :11-13: `Life` no longer carries `read` or `bash`. Name the extension API of the crate.
- :24-31: the workspace has three packages; add `extensions/skills` (`@furb/skills`).
- :72-77: add `words`, `answers`, `plays`, `Bound`, `verb` and `job` to the harness text. :78-80: a builtin's tests are in `test/<name>/`, an extension's in `extensions/<name>/test/`.
- :93-104: the hygiene laws cover every contract (engine.pyi, `builtin/*.pyi`, `extensions/*/*.pyi`). Law 7 also covers the words of builtins and extensions. Add the new manifest law.
- :110-135: ruff over `src test script extensions`; `bun test ... extensions`; `cargo test` needs git and npm on PATH.
- :137-139: CI text.

### C.3 src/furb/CLAUDE.md

**Move out** (to a new `src/furb/builtin/CLAUDE.md`, "The names of the builtin extensions"):
- command (:96), merged (:97), door (:98-99), text (:100), grant (:109), ledger (:110);
- the working directory of a chain (:59-60);
- the makers span, grep and differs (in :101-102).

**Keep and rewrite:**
- show (:101-102): "a callable given the lines of the content of a showing, which gives the numbers of the lines the chain tells".
- World (:26-27): the disk and the machine are reached through the World parts of extensions.
- program (:88-90): "which the ladder query answers", not "which its door shows".
- replay (:91-92): "how a ladder given a word edits the program".
- pending (:117-118): "a wait, a prompt to the operator, or an act of an extension that the World started".

**Add:**
- ladder: the query that a chain answers for its prompts;
- showing: a note of a path, a content and a show;
- extension: a word played on a chain, with a World part and a TUI part of the host, which the engine does not know.

**Laws:**
- :143-144: "nothing of the engine is a class but the name of an act and the two exceptions".
- :145-146: "The verbs carry their signatures".
- Add: "The engine knows no extension: a host plays the word of each one as a rung, as the World, on every chain without a source."
- Add: "The word of a module is the module with each top-level `from furb` import made empty lines."

### C.4 Other documents

**docs/developer-guide.md:**
- Tools table (:19-25): add git, and Node.js 22 with npm, for `cargo test`.
- Map (:37-49): add `src/furb/builtin/`, `extensions/`, `src/extension.rs`, `docs/extensions.md`.
- :57-58: `read` and `bash` now come from builtin extensions.
- :72: the path is now `test/bash/test_bash.py`. :73: ruff over `extensions`. :83: bun test over `extensions`.
- Add a section "Changing or writing an extension". Add docs/extensions.md to :134-139.

**READMEs:**
- README.md: a line on extensions, and a link to docs/extensions.md in "Learn more".
- bind/typescript/README.md: the operations table (:92-106) and "Text and Exit carry is" (:115) change with step 4. Add an "Extensions" section on the napi extension API and `@furb/engine/extension`.
- bind/python/README.md: the extension API of `furb_monty`.
- extensions/skills/README.md (new).

**TUI docs:**
- tui/README.md: remove the `/extension` text (:202, :208-212). Say that extensions come from config.json, with a link to docs/extensions.md. Run `bun run docs` again: the table at :123-166 loses `/extension`, and the builtin commands (`/bash`, `/read`, `/cd`, `/grant`, `/context`) come from the TUI parts of the builtins once docs.ts reads them.
- docs/tui.md:237-241: the Extensions section and the new capture 42.

Last: delete LEDGER.md.

### C.5 Gates to run, all green

1. `uv sync --reinstall-package furb-monty`
2. `uv run pytest -q` (both engines, 100% coverage, now also `extensions/*/test`)
3. `uv run pytest -q test/test_hygiene.py`
4. `uv run ruff format src test script extensions`, then `uv run ruff check src test script extensions`
5. `uv run ty check --error-on-warning`
6. `cargo fmt --check`
7. `cargo clippy --all-targets -- -D warnings` and `cargo clippy --all-targets --features typescript -- -D warnings`
8. `cargo test` and `cargo test --features typescript`
9. `uv run pre-commit run --all-files` (includes uv-lock)
10. `bun install --frozen-lockfile` and `bun run build`, then `git diff --exit-code -- bind/typescript/index.cjs bind/typescript/index.d.cts`
11. `bun run docs`, then `git diff --exit-code -- tui/README.md extensions/*/README.md`
12. `bun run check`, `bun run lint`, `bun test bind/typescript/test tui/test extensions`
13. `bun run screenshots` and `bun run animation`, then read each changed capture. This needs bun 1.4.2 or later; this host has 1.3.11.
14. `npm pack --dry-run` in `extensions/skills`, to check the file list. It writes nothing.
15. A search for em and en dashes in the changed prose.
16. Optional: `uv run python script/play.py`, which spends prompts.

### D. What the owner must do for releases

### PyPI (furb and furb-monty)

- **A workflow exists:** `.github/workflows/publish.yml`. A `v*` tag (publish.yml:8-10) builds furb with `uv build`, and furb-monty as wheels on ubuntu and macos (:46) plus an sdist. It publishes through the environment `pypi` (:90) with OIDC, `uv publish --trusted-publishing always` (:101). No token is kept. A manual run builds and publishes nothing, which is a rehearsal.
- **Present state:**
  - PyPI has only `furb 0.1.0`, uploaded 2026-09-19 from tag `v0.1.0`. Its requires_dist does not name furb-monty.
  - `furb-monty` is not on PyPI (404).
  - Today `furb` pins `furb-monty==0.1.0` (pyproject.toml:20), so the next release must publish both.
- **Missing, for the owner:**
  1. On pypi.org, add a pending trusted publisher for the new project `furb-monty`: owner `uael`, repo `furb`, workflow `publish.yml`, environment `pypi`.
  2. Bump the versions, because PyPI refuses a second 0.1.0: pyproject.toml:3 and :20, bind/python/pyproject.toml:3 and :20, Cargo.toml:3, bind/typescript/package.json:3, extensions/skills/package.json. Then run `uv lock`.
  3. Run the rehearsal from Actions, and check that the furb wheel holds `furb/builtin/*.py` and `*.pyi`.
  4. Push tag `v0.2.0` on main.
- **Why a release is needed:** users need it to get the builtins as extensions, and extension authors need it to type-check `from furb.builtin.files import ...` against the published stubs.
- **Note:** there is no Windows wheel, no macOS x86_64 wheel and no Linux arm64 wheel, so those machines build from the sdist with Rust.

### npm (@furb/engine and @furb/skills)

- **No npm workflow exists.** `@furb/engine` is not private but was never published. `@furb/tui` is private.
- **Scope:** `/-/org/furb/package` returns 200 `{}`, where an unknown scope gives 404 "Scope not found". So the scope `furb` already exists, with no public packages. The owner must confirm that they own it, or rename the packages.
- **Missing, for the owner:**
  1. A workflow, for example `.github/workflows/npm.yml`, on the same `v*` tag.
  2. Auth for the first publish of each new package: an `NPM_TOKEN` secret (a granular token with publish rights on `@furb`), or a manual publish. Trusted publishing is set up per package that already exists. After that, configure the trusted publisher on npmjs.com (uael/furb and the workflow file) and remove the token. It needs npm CLI 11.5.1 or later and `id-token: write`.
  3. `publishConfig.access: "public"` on each scoped package.
  4. For `@furb/skills`: `bun pm pack` (it rewrites `workspace:*`), then `npm publish furb-skills-<v>.tgz --access public --provenance`.
  5. For `@furb/engine`, which holds a native `.node`:
     - add `napi.targets` to bind/typescript/package.json:6-8;
     - build on a matrix of Linux x64, macOS arm64 and Windows x64, with uv and python for `system.json` (build.ts:33-45);
     - publish the platform packages that the generated index.cjs already requires (`@furb/engine-linux-x64-gnu`, `@furb/engine-darwin-arm64`, `@furb/engine-win32-x64-msvc`) through `napi create-npm-dirs`, `napi artifacts` and `napi prepublish`;
     - then publish `@furb/engine`.
- **None of this blocks the proofs.** The tests use a local tarball and a local bare repo. After the merge to main, `{"git": "https://github.com/uael/furb", "path": "extensions/skills"}` works with no release. Only `{"npm": "@furb/skills"}` for real users needs the npm publish.

### Critical files for implementation
- /home/user/furb/test/conftest.py
- /home/user/furb/test/test_hygiene.py
- /home/user/furb/src/furb/builtin/files.py (the model for skills.py, and the `read` that skill uses)
- /home/user/furb/pyproject.toml
- /home/user/furb/.github/workflows/gates.yml
- /home/user/furb/.github/workflows/publish.yml
- /home/user/furb/package.json
- /home/user/furb/tsconfig.json
- /home/user/furb/bind/typescript/package.json
- /home/user/furb/bind/typescript/src/project.ts
- /home/user/furb/tui/src/extensions.ts (to remove)
- /home/user/furb/tui/script/docs.ts
- /home/user/furb/src/furb/CLAUDE.md
- /home/user/furb/CLAUDE.md
- /home/user/furb/docs/developer-guide.md
- /home/user/furb/LEDGER.md (delete at the end)

## Part 4: the crate and the Python host

### 0. What I found at HEAD (read this first)

- `cargo test` is red at HEAD. `src/preamble.py:224` binds `verb(names, "cwd")`, and `cwd` has left the engine. So every typed Rust World raises KeyError when it is primed: boot catches it, the root is "" and `raised` is set. The gate tests that use `read`, `HEAD`, `TIMEOUT`, `bash` or `Exit` (`src/gate.rs:128,139,166,180,214-216,221`) are red too. So commit 1 below can gate only `cargo test extension`.
- `src/furb/world.py:41` imports `Text` from `furb.engine`, which no longer holds it, so the Python host is broken until commit 6.
- The rule the owner chose is already in the suite: `test/conftest.py:343-350` (`plays`), `:490-506` (`life()`), and `Sand.hears` `:173-174`. The crate must do exactly the same thing.
- `ruff_python_parser`, `ruff_python_ast`, `ruff_text_size` 0.0.9 (crates.io) and `sha2` 0.10.9 are already in `Cargo.lock` through monty (`Cargo.lock:1164-1199, 1915`). You can add them as direct dependencies with no new download. Their `rust-version` is 1.95, the same as the crate. You parse with ruff; you do not need a parser by lines.
- `git`, `npm` and `tar` are on this host. `cargo test --features typescript` runs on ubuntu, macOS and Windows in CI (`.github/workflows/gates.yml:153-155`). So the fetch tests must also pass on Windows, where npm is `npm.cmd`.

### 1. The new module `src/extension.rs` (with `src/extension.test.rs`)

`src/lib.rs`: add `pub mod extension;` after `pub mod ear;` (line 23). At the end of `extension.rs`, add `#[cfg(test)] #[path = "extension.test.rs"] mod test;`, as `src/life.rs:836-838` does.

`Cargo.toml` (lines 95-122):
- `serde_json = { version = "1", features = ["preserve_order", "raw_value"] }` stops being optional. Remove `"dep:serde_json"` from the `typescript` feature (line 93). `preserve_order` keeps the order of the keys in a file.
- Add `sha2 = "0.10"`, `ruff_python_parser = "=0.0.9"`, `ruff_python_ast = "=0.0.9"` and `ruff_text_size = "=0.0.9"`. The versions are pinned to the ones monty pulls, so cargo builds one copy. Add a comment that says so.

### Error type: `extension::Error`

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
  Config { file: PathBuf, why: String },          // not json, or extensions is no object
  Entry { name: String, file: PathBuf, why: String }, // an entry of no known form, or true with no source
  Fetch { name: String, why: String },            // git or npm or tar failed (their stderr), or no directory at a path
  Manifest { root: PathBuf, why: String },        // no package.json, no furb field, no python, a name that is not the key, a path out of the root
  Requires { name: String, needs: String, off: bool },
  Cycle { names: Vec<String> },
  Word { name: String, line: usize, why: String }, // name is "" for word() alone
}
```

Also add `impl Display`, `impl std::error::Error`, and `impl From<Error> for Fault` that gives `Fault::refused(error.to_string())`. That is how both bindings raise `Refused`.

### Places: the home config and the cache

```rust
pub struct Places { pub config: PathBuf, pub cache: PathBuf, pub home: Option<PathBuf> }
impl Places {
  pub fn here() -> Places; // of(|k| std::env::var_os(k), std::env::home_dir(), cfg!(windows))
  pub fn of(var: impl Fn(&str) -> Option<OsString>, home: Option<PathBuf>, windows: bool) -> Places;
}
```

- **config:** `FURB_CONFIG_DIR`, else `(XDG_CONFIG_HOME | windows? APPDATA | home/.config)/furb`. This is the same order as `tui/src/preferences.ts:16-24`.
- **cache:** `FURB_CACHE_DIR` (new; tests and hosts need it), else `(XDG_CACHE_HOME | windows? LOCALAPPDATA | home/.cache)/furb`.
- An empty variable counts as unset.
- The home is `std::env::home_dir()`. If clippy calls it deprecated on the toolchain, read `HOME`, or `USERPROFILE` on Windows.
- The lookup is injected so that tests never call `set_var`, which is `unsafe` in edition 2024.

### Config

```rust
pub enum Source { Builtin, Path(PathBuf), Git { url: String, reference: Option<String>, path: Option<PathBuf> }, Npm { package: String, version: Option<String> } }
pub enum Setting { Off, On, From(Source) }
pub struct Entry { pub name: String, pub on: bool, pub source: Option<Source>, pub file: Option<PathBuf> }

pub fn settings(text: &str, file: &Path, home: Option<&Path>) -> Result<Vec<(String, Setting)>, Error>;
pub fn read_settings(file: &Path, home: Option<&Path>) -> Result<Vec<(String, Setting)>, Error>; // missing file: empty
pub fn merged(layers: &[Vec<(String, Setting)>]) -> Result<Vec<Entry>, Error>;
pub fn config(places: &Places, project: &Path) -> Result<Vec<Entry>, Error>;
```

- `config` reads `places.config/config.json`, then `project/.furb/config.json`.
- The file is `{"extensions": {name: form}}`. Other top-level keys are ignored.

The forms of an entry:
- `false` gives Off. `true` gives On.
- `"p"` and `{"path": "p"}` give `Path`.
- `{"git": url, "ref"?: s, "path"?: sub}` gives `Git`.
- `{"npm": package, "version"?: v}` gives `Npm`.
- Any other form, or a field of the wrong type, gives `Error::Entry`.

Path resolution:
- A path of `Path`, and a `git` or `npm` value that starts with `./`, `../` or `~`, resolve against the parent of the config file.
- `~` alone, `~/x` and `~\x` expand to the home. `~user` is not expanded.

Merge by name:
- Start from the builtins in the order files, bash, grant, each On with `Source::Builtin`.
- Then read the home layer, then the local layer. A new name is appended in file order. A known name keeps its place.
- `false` turns an entry off and keeps its source.
- `true` turns it on and keeps the earlier source. For a name with no earlier source that is no builtin, this is `Error::Entry`.
- A source form replaces the source and turns the entry on.
- A builtin name may take an external source. The manifest name must still equal the key.

### Cache and fetch

```rust
pub fn fetched(name: &str, source: &Source, places: &Places, refresh: bool) -> Result<PathBuf, Error>;
pub fn installed(root: &Path) -> Result<(), Error>;
```

- **Path:** the directory itself, fetched in place. If there is no directory, give `Error::Fetch`.
- **Git:**
  - The cache directory is `places.cache/extensions/git/<sha256hex("<url>#<ref or empty>")>`.
  - When the directory exists and `refresh` is false, reuse it.
  - Otherwise run `git -c advice.detachedHead=false clone --depth 1 [--branch <ref>] <url> <tmp>` with `GIT_TERMINAL_PROMPT=0`, where tmp is `places.cache/extensions/.tmp/<key>-<pid>`.
  - Then remove the old directory (on a refresh) and `fs::rename` tmp into place. If the rename fails because another process won, keep the directory that stands and remove tmp.
  - The root is that directory joined with `path`.
  - A ref is a branch or a tag, as `--branch` takes (see open question 13).
- **Npm:**
  - The cache directory is `places.cache/extensions/npm/<package>@<version or "latest">` when `package` is a registry name. The pattern is `^(@[a-z0-9-~][a-z0-9-._~]*/)?[a-z0-9-~][a-z0-9-._~]*$`, and a scope makes a nested directory.
  - For a tarball path or a URL, the directory is `npm/<sha256hex(spec)>`.
  - Run `npm pack <package>[@<version>] --pack-destination <tmp> --json --ignore-scripts`, with the program `npm.cmd` on Windows. Read `[0].filename` from the JSON on stdout.
  - Then run `tar -xzf <tmp>/<filename> -C <tmp>` and rename `<tmp>/package` into place.
  - Fetch once. A refresh fetches again.
- **`installed(root)`:** runs `npm install --omit=dev --ignore-scripts` in the root when `package.json` has a non-empty `dependencies` and there is no `node_modules`. Only a host that loads a world part calls it (TypeScript), through the `install` flag below.
- One private helper runs a program and gives stdout, or `Error::Fetch` with its stderr.

### Manifest and builtins

```rust
pub struct Worlds { pub ts: Option<PathBuf>, pub py: Option<PathBuf> }
pub struct Manifest { pub name: String, pub python: PathBuf, pub world: Worlds, pub tui: Option<PathBuf>, pub requires: Vec<String> }
pub fn manifest_of(text: &str, root: &Path) -> Result<Manifest, Error>;
pub fn manifest(root: &Path) -> Result<Manifest, Error>; // root/package.json, field "furb"

pub struct Extension { pub name: String, pub root: Option<PathBuf>, pub word: String, pub requires: Vec<String>, pub world: Worlds, pub tui: Option<PathBuf> }
pub fn builtins() -> Vec<Extension>;
```

- Manifest fields: `name` and `python` are required. `world {ts?, py?}`, `tui?` and `requires? []` are optional.
- Every path is joined to the root. A path whose normal form leaves the root is `Error::Manifest`.
- `builtins()`: files (requires none), bash (requires files), grant (requires none).
- Each builtin word is `word(include_str!("furb/builtin/<x>.py"))`. The path is relative to `src/extension.rs`, so it is `src/furb/builtin/`.
- A builtin has `root: None`, no world and no tui. Each host knows its own builtin parts.

### Load and order

```rust
pub fn loaded(name: &str, root: &Path) -> Result<Extension, Error>;
pub fn ordered(entries: &[Entry], extensions: Vec<Extension>) -> Result<Vec<Extension>, Error>;
pub fn extensions(places: &Places, project: &Path, refresh: bool, install: bool) -> Result<Vec<Extension>, Error>;
pub fn words(extensions: &[Extension]) -> Vec<String>;
pub fn missing<'a>(program: &[&str], words: &'a [String]) -> Vec<&'a str>;
```

- `loaded(name, root)` reads the manifest, checks that the name equals the key, reads the python file, and makes the word. It maps a word error to `Error::Word { name, .. }`.
- `ordered` does two things:
  - It checks every requirement. A requirement that is off gives `Requires { off: true }`. One that is not configured gives `Requires { off: false }`.
  - It sorts in a stable order: repeatedly take the first extension, in the entry order, whose requirements are all placed. If some remain and none can be taken, give `Cycle`.
- `extensions` does all of it:
  - config, then for each entry that is on: the builtin, or `fetched`, then `installed` if `install` and the extension has `world.ts`, then `loaded`;
  - then `ordered`.
- `missing` gives the words that are not among the program values, in their order, by string equality. This is the same rule as `conftest.plays`.

### The word of a module: `pub fn word(source: &str) -> Result<String, Error>`

1. Parse with `ruff_python_parser::parse_module(source)`. On `Err(e)`, give `Error::Word { line: the line of e.location().start(), why: e.to_string() }`.
2. For each statement of `parsed.syntax().body` (top level only): when it is `Stmt::ImportFrom(StmtImportFrom { module: Some(m), level: 0, .. })` and `m.id.as_str()` is `"furb"` or starts with `"furb."`, take its `range` (`ruff_text_size::Ranged`). This gives the lines from the line of `start` to the line of `end`.
   - This covers `furb.engine`, `furb.builtin.*`, `furb.extensions.*` and `from furb import engine`.
   - It also covers parenthesized imports over several lines and imports continued with a backslash, since the range spans them.
3. If another top-level statement starts or ends on one of those lines (`;`), give `Error::Word`: "an import of furb shares line N with another statement".
4. Rebuild the text with `split_inclusive('\n')`. Each blanked line keeps only its ending (`"\n"`, `"\r\n"`, or `""` at the end of the file). So the number of lines is the same, and a finding of the gate points at the line of the file.
5. Leave everything else alone:
   - `import furb` stays, and the gate refuses it (`src/gate.rs:204-210`).
   - A relative import stays, and so does `from furbish import x`.
   - An import inside a function or a block stays.
   - A module with no such import is given back unchanged. That is form (a).

### Tests in `src/extension.test.rs`, one by one

Each test works in `std::env::temp_dir().join("furb-extension-<test>")`, emptied first, as `src/life.test.rs:193-198` does.

Places:
1. `the_config_directory_is_furb_config_dir_when_it_is_set`
2. `the_config_directory_is_under_xdg_config_home_and_then_under_the_home`
3. `the_config_directory_on_windows_is_under_appdata_when_xdg_is_unset`
4. `the_cache_directory_is_furb_cache_dir_then_xdg_cache_home_then_the_home`
5. `the_cache_directory_on_windows_is_under_localappdata`
6. `an_empty_variable_counts_as_unset`

Config:
7. `a_missing_config_file_holds_no_setting`
8. `each_form_of_an_entry_is_read` (false, true, "p", {path}, {git,ref,path}, {npm,version})
9. `an_entry_of_no_known_form_is_refused_with_its_file_and_its_name` (3, {"url"}, {"git": 1})
10. `a_config_that_is_no_json_is_refused_with_its_file`
11. `a_path_resolves_against_the_directory_of_its_config_file`
12. `a_tilde_expands_to_the_home`
13. `the_local_config_overrides_the_home_config_by_name`
14. `the_builtins_come_first_then_the_home_names_then_the_local_names_in_file_order`
15. `a_builtin_is_on_unless_false`
16. `true_keeps_the_source_an_earlier_file_gave`
17. `true_for_a_name_that_no_file_sources_and_no_builtin_is_refused`

Manifest:
18. `a_manifest_is_read_from_the_furb_field_of_package_json`
19. `a_manifest_without_a_furb_field_or_a_python_is_refused`
20. `a_manifest_whose_name_is_not_its_key_is_refused`
21. `a_manifest_path_that_leaves_its_root_is_refused`

Builtins:
22. `the_builtins_are_files_bash_and_grant_and_bash_requires_files`
23. `the_word_of_each_builtin_is_the_word_of_the_file_the_package_ships` (compare with `read_to_string(CARGO_MANIFEST_DIR/src/furb/builtin/x.py)` through `word`)

Order:
24. `an_extension_stands_after_what_it_requires_and_otherwise_keeps_its_place`
25. `an_extension_whose_requirement_is_off_is_refused` (files false, bash on)
26. `an_extension_whose_requirement_is_missing_is_refused`
27. `a_cycle_of_requirements_is_refused`

Word:
28. `a_word_without_an_import_of_furb_is_itself`
29. `a_top_level_import_from_furb_becomes_an_empty_line_so_every_line_keeps_its_number` (engine, builtin.files, extensions.x, `from furb import engine`)
30. `a_parenthesized_import_over_several_lines_becomes_as_many_empty_lines`
31. `a_backslash_continued_import_becomes_as_many_empty_lines`
32. `an_import_of_another_package_a_relative_import_and_import_furb_stay`
33. `an_import_of_furb_inside_a_function_or_a_block_stays`
34. `a_line_ending_of_crlf_is_kept`
35. `an_import_of_furb_that_shares_its_line_with_another_statement_is_refused`
36. `a_module_that_does_not_parse_is_refused_with_its_line`

Missing:
37. `the_missing_words_are_the_words_the_program_lacks_in_their_order`

Fetch:
38. `a_path_entry_is_fetched_in_place_and_a_missing_directory_is_refused`
39. `a_git_entry_is_cloned_once_into_the_cache_under_the_hash_of_its_url_and_ref`
    - Use a local bare repository made with `git -c init.defaultBranch=main -c user.name=furb -c user.email=furb@example.com -c commit.gpgsign=false`.
    - Check that a second fetch runs no clone: mark a file in the cache directory and see that it stays.
40. `a_git_entry_takes_its_ref_and_its_path`
41. `a_git_entry_is_cloned_again_on_a_refresh`
42. `a_git_fetch_that_fails_is_refused_with_what_git_said`
43. `an_npm_entry_is_packed_and_unpacked_once_into_the_cache` (`npm pack` of a local folder, which is offline)
44. `an_npm_entry_of_a_registry_name_is_kept_under_its_name_and_version` (pure function of the directory, with no npm run)

Load:
45. `extensions_gives_the_builtins_and_the_path_extensions_of_both_configs_in_order_with_their_words`
46. `extensions_refuses_an_extension_whose_manifest_name_is_not_its_key`

In `src/gate.rs` tests, also add:
47. `the_word_of_every_builtin_passes_the_gate_after_the_words_before_it`. Use `sheet(&[files, bash, grant], "close(1)")` and `found()` must be empty.
48. `a_word_reads_a_name_an_extension_bound_in_the_program`. Use `said(&[files], "close(read('a').lines)")` and expect no finding.

### 2. The play rule inside Life

Where the rule lives: the sandbox side is `src/preamble.py`, and the Rust side is `Opening` and `Opening::boot`.

### Preamble

Add a global near `MADE` (lines 87-90):

```python
WORDS: list[str] = []
"""WORDS are the words of the extensions the host plays, which stand empty while boot says the record again."""
```

Add three functions after `opened` (line 462), before `called`:

```python
def plays(engine: Names, chain: str) -> None:
  """Each word the program of a chain lacks, played on it as a rung, in order, by whoever speaks."""
  _, program = verb(engine, "ask")("program", chain)
  held = list(program.values()) if isinstance(program, dict) else []
  for word in [w for w in WORDS if w not in held]:
    verb(engine, "rung")(word, on=chain)

def playing(world: Ear, engine: Names) -> Ear:
  """The World, which plays the words on each chain born without a source once the life stands on its record,
  and passes every fact to the World it wraps and every saying of that World to the bus."""
  a = None
  while True:
    match a:
      case ("chain", str(id), _, _, _, "") if WORDS:
        plays(engine, id)
    try:
      said = world.send(a)
    except StopIteration:
      return
    a = yield said

def played(engine: Names, words: list[str]) -> None:
  """The words, played as the World on every chain without a source, once boot stands on the record."""
  WORDS[:] = words
  site = engine["site"]; acts = engine["acts"]
  assert isinstance(site, ContextVar) and isinstance(acts, dict)
  token = site.set(str(engine["WORLD"]))
  try:
    for one in [a[1] for a in list(acts.values()) if a[0] == "chain" and not a[5]]:
      plays(engine, one)
  finally:
    site.reset(token)
```

In `opened` (lines 435-462):
- At line 446 add `WORDS.clear()` beside `MADE.clear()`.
- After the `outside` dict (lines 450-453) add `if "world" in outside: outside["world"] = playing(outside["world"], engine)`.

This wraps both the typed `worldly` and the heard `crossing`. The site is `world`, because `hears` sets the site to the name of the ear (`src/furb/engine.py:565`). The journal keeps a rung made by `world` (engine.py:504-505), and a later life makes it again with `site.set(by)` (engine.py:498-499).

### Rust side (`src/life.rs`)

- `Opening` (lines 347-352): add `words: Vec<String>`. Set it to `Vec::new()` in `Life::open` (433-440) and in `Life::open_on` (447-452).
- After `limits` (line 373), add a builder:
  ```rust
  pub fn words(mut self, words: impl IntoIterator<Item = impl Into<String>>) -> Self
  ```
  Its doc: "The words of the extensions, which the life plays as the World on every chain without a source: once boot stands on its record, on each chain whose program lacks one, and at the birth of each such chain after."
- `Opening::boot` (lines 380-417): destructure `words` at 381. After `raised` (line 415), before `Ok(Life ...)` at 416, add:
  ```rust
  if raised.is_none() && !words.is_empty() {
    inner.ran("played(__engine, __words)", vec![("__words", Object::list(words.into_iter().map(Object::string)))])?;
  }
  ```

### How it compares with the program query

It asks `ask("program", chain)`, which the chain itself answers (engine.py:176-177) with `{rung: unquoted(word)}`. A word is missing when it is not among the values, by equality. An extension word holds no `<Sn>` quote, so `unquoted` of it is itself.

### What it must not do during replay

- `WORDS` is empty from `opened` until `played`, so a `chain` fact that the journal says again during boot plays nothing. The whole record is said again before `boot` returns: the journal loop is driven from `dispatch` (engine.py:476-499, 551-560).
- Chains replayed from the record already hold the rungs of the world in their program, so the sweep adds only new words.
- When boot raised (a drift), it plays nothing, since the journal keeps nothing more.
- A chain with a source is never played: it takes the program of its origin through replay (engine.py:201-210).

### Also add to `Life`

`pub fn awaited(&mut self, id: &str) -> Act<'_, Object>`, "One act of the life by its name, to await". A Rust host that says an extension verb through `verb()` gets only the name, and life.test.rs needs it. The private `act<T>` stays at lines 575-583.

### 3. The crate cleanup (file:line)

### `src/world.rs`

- Remove `Text` from the imports (line 23). Import `ear::Reply`.
- Remove `Command` (63-78) and `Running` (80-86).
- Remove `Voice::out` and `Voice::exited` (134-142). Hosts use `Voice::say(Fact::says("out", ..))`.
- In the trait (179-220): remove `read` (194-195), `write` (197-198) and `run` (212-213). Add:
  ```rust
  /// Every other fact of the life: a question of an extension, a start of an act the World does not know by kind,
  /// a feed, a control. It is answered as an ear of the host answers.
  fn hears(&mut self, fact: &Fact) -> Reply { refused(fact) }
  /// The value of the verb the World said, as ("value", v) or ("raised", fault), and what it says now.
  fn answered(&mut self, got: ObjectRef<'_>) -> Reply { let _ = got; Reply::Nothing }
  ```
- Add `pub fn refused(fact: &Fact) -> Reply`. For a `start` fact it gives `Reply::Calls { name: "close", args: [Fault::refused(format!("the World does no {kind}")).object(), about], kwargs: [] }`, where kind is `about` without its trailing digits. For any other fact it gives `Reply::Nothing`. This is the contract law of the Ear.
- Update the module docs (1-11) and the trait docs (179-184): the Voice is for whatever an act of an extension says later, and not "a command".

### `src/preamble.py`

- Module docstring (lines 5-8): the World is asked by method for what the engine asks of every World, and hears every other fact.
- `World` Protocol (45-59): remove `read`, `write`, `run`, `feed` and `slay`. Add `def hears(self, fact: object) -> object: ...` and `def answered(self, got: object) -> object: ...`.
- Add `settled` before `crossing`. It holds the loop of `crossing` lines 283-290:
  ```python
  def settled(reply, answered, names, ears):
    while True:
      match reply:
        case ("calls", str(which), list(args), dict(kwargs)):
          try: got = ("value", outward(asked(names, ears, which, args, kwargs), names))
          except BaseException as no: got = ("raised", no)
          reply = answered(got)
        case ("raised", no):
          raise fault(no, names, ears)
        case _:
          return reply
  ```
- `crossing` (270-302): use `reply = settled(ears.hears(name, outward(a, names)), lambda got: ears.answered(name, got), names, ears)`, then match `say`, `over` or other. The behavior stays the same.
- `worldly` (216-267): rewrite. Drop `cwd`, `ask`, `covers`, `running` and `at()`.
  ```python
  acts = names["acts"]
  while True:
    a = yield
    said = None
    match a:
      case ("stand", qid, *_): said = ("done", qid, again(world.stand(), names, ears))
      case ("clock", qid, *_): said = ("done", qid, world.clock())
      case ("chance", qid, *_): said = ("done", qid, world.chance())
      case ("ask", rung, _, on, actor, turns): world.ask(rung, on, actor, turns)
      case ("start", about, _) if acts[about][0] == "wait": world.wait(about, acts[about][4])
      case ("start", about, _) if acts[about][0] == "prompt": world.prompt(about, acts[about][4], acts[about][5])
      case ("keep", _, _, entry): world.keep(entry)
      case _:
        match settled(world.hears(a), world.answered, names, ears):
          case ("say", tuple(saying)): said = again(saying, names, ears)
    if said is not None:
      yield said
  ```
  - Use `yield said` as a statement, as the old code does. The fact handed back is dropped here, and the bus says it to every ear again.
  - Pass the raw fact, as `keep` and `ask` already do, and not `outward(a)`. `outward` would hold every show a tell carries in `MADE` for ever.
- Play functions and `opened`: as in section 2.

### `src/life.rs`

- Imports: line 30 becomes `value::{Fault, Object, ObjectRef, entry, inward}`, and line 31 becomes `world::{Later, Said, Voice, World}`. Add `fact::Fact`, which is already imported (line 27).
- `Hosting.running` (line 79) goes, and so does its init at line 389.
- In `worldly` (145-210):
  - The `running` parameter (148) goes.
  - The `read` and `write` arms (158-159) go, and so do `run` (172-185) and `feed`/`slay` (196-207).
  - Add `"hears" => at(0).and_then(Fact::of).map_or_else(Object::none, |fact| replied(world.hears(&fact)))` and `"answered" => replied(world.answered(at(0).unwrap_or(Object::none().as_ref())))`. For the second, hold an owned `Object::none()` first to satisfy borrowing.
- The call at line 137 loses `running`. `fn answered` (212-218) goes.
- Typed verbs to remove: `read` (590-597), `write` (599-602), `cd` (629-632), `cwd` (634-637), `grant` (719-735), `bash` (737-752).
- `impl Came for Exit` (782-787) and `impl Came for Text` (789-794) go.
- Keep: peek, turns, clock, chance, gate, get, pause, wake, cancel, close, debug, wait, rung, prompt and chain. Also keep `Life::on`.

### `src/value.rs`

- Remove `Text` (240-273) and `Exit` (275-307).
- Rewrite the module docs (1-12): the crate reads one class, `Fault`. A class a word defined crosses out as monty carries it, and in through the preamble mark.
- The test `an_instance_of_the_engine_goes_in_as_its_name_and_its_fields` (429-437) uses `marked("Point", [("x", Object::int(1))])` and `Fault::refused`.

### `src/lib.rs`

- Docs: lines 8-9 name `rung` and `prompt` in place of `read` and `bash`. Lines 19-21 say "read as [`Fault`]".
- Line 37 becomes `value::{Fault, Object, ObjectRef}`. Line 38 becomes `world::{Actor, Later, Said, Standing, Voice, World}`.
- Add `pub use extension::Extension;`.

### `src/binding/ts/wire.rs`

- Line 1 drops `Text`.
- Remove lines 61-63, the `Text::of` branch. A Text is now a class of a word, which the preamble sends out as `{is: "instance", ...}`.

### `src/binding/py.rs`

- Line 6 of the comment: "an instance of a class a word defined as a type of this interpreter" in place of "a `Text` as a `Text`". There is no code change for Text.

### `bind/python/furb_monty/engine.py`

- PURE (35-38): remove `"span", "grep", "differs", "HEAD", "TAIL", "HIDDEN", "showing"`.
- ACTS (42): remove `"grant", "bash"`.

### `src/gate.rs` tests

| Line | Now | Becomes |
|---|---|---|
| 128 | `x = read('a')` | `x = clock()` |
| 139 | `close(len(read('a.txt').lines))` | `close(len(turns()))` |
| 166 | `close((await bash('ls')).code)` | `close(await wait(0))` |
| 180 | `setattr(Exit, 'a', 1)` | `setattr(Refused, 'a', 1)` |
| 205 | `from furb.engine import read` | `from furb.engine import clock` |
| 214 | `read = 1\nclose(read)` | `clock = 1\nclose(clock)` |
| 215 | `old = HEAD\nHEAD = old\nclose(TIMEOUT + 1)` | `old = OPERATOR\nOPERATOR = old\nclose(len(OPERATOR) + 1)` |
| 216 | `x = read('a')\nread = 1\nclose(x)` | `x = clock()\nclock = 1\nclose(x)` |
| 221 | `said(&["read = 1"], "close(read('a'))")` | `said(&["clock = 1"], "close(clock())")` (call-non-callable) |

Also add tests 47 and 48 from section 1.

### `src/life.test.rs`

- Imports (23-26): drop `Command`, `Exit`, `Running` and `Text`. Add `Fact`, `Reply`, `extension`, and `value::{field, entry}`.
- `Yard` (29-52): add `pending: Option<Pending>` (`Read { qid, path }`, `Write { qid, path, content }`, `Start { about, command }`, `Covers { fact, left: Vec<String> }`), `commands: HashMap<String, (String, String)>` (act to command and chain), and `running: Vec<String>`. Remove `impl Running for Ran` (54-65). The Yard only notes `slain`, as before.
- `impl World for Yard` (67-159): remove `read`, `write` and `run` (83-94, 124-145). Add `hears`:
  - A `bash` question fact: note the command.
  - `read` or `write`: `Reply::Calls { name: "cwd", args: [], kwargs: [("on", on)] }`, and remember what is pending.
  - `start` of a known bash: `Calls cwd`. On the answer, spawn the thread of lines 127-143, which speaks through `voice.say(Fact::says("out", ...))` and `"exited"`.
  - `cancel` or `close` while commands run: `Calls { name: "covers", args: [fact.0, id] }` for each running id in turn. On `("value", True)`, slay and note it.
  - `exited`: drop the id from `running`.
  - Otherwise: `world::refused(fact)`.
- `answered` finishes what is pending. A read or a write gives `Reply::Say(Fact::says("done", qid, [dict path/content, or Fault::refused("no file at ..").object()]))`.
- `Lived::new` (193-204): `Life::open(world).words(extension::words(&extension::builtins())).boot(record)`.

Tests:
- 238-244: `life.verb("cwd", vec![], vec![("on", Object::string(root))])`.
- 246-257: write with `block_on(life.rung("write(Text('a.txt', 'one\\ntwo\\n'))", "", "", &root))`. Read with `life.verb("read", [a.txt], [on])`, then `field(value, "content")`. The refusal stays the same.
- 259-269 and 365-376: `fs::write` in place of `life.write`.
- 290-299 and 301-315: `life.verb("bash", ..)`, then `lived.settled(id)`, then read `code`, `stdout.content` and `stderr.content` through `field(.., "value")`.
- 356-363: `get("wait9")`.

New tests:
- `the_life_plays_the_words_of_the_extensions_on_the_root_as_the_world`: the program of the root holds the three words; `get("rung1").by() == "world"`.
- `the_life_plays_the_words_on_a_chain_born_without_a_source`.
- `a_chain_with_a_source_is_played_no_word_of_its_own`.
- `a_later_life_plays_only_the_words_its_record_lacks`: the same words give no new rung, and one more word gives one rung.
- `a_life_opened_with_no_words_plays_nothing`.
- `a_start_the_world_does_not_do_is_closed_with_its_refusal`: `life.word("act('job', __on, started(idle))", ..)`, then the outcome is `Refused("the World does no job")`.
- `the_world_answers_a_question_of_an_extension_through_a_verb_it_says`: the read through cwd above.

### 4. Exposure

### py.rs (the `python` feature)

```rust
#[pyclass(module = "furb_monty._monty", name = "Extension", frozen, get_all)]
pub struct PyExtension { name: String, root: Option<String>, word: String, requires: Vec<String>, world_ts: Option<String>, world_py: Option<String>, tui: Option<String> }
#[pyfunction] #[pyo3(signature = (project, refresh = false))] fn extensions(py, project: &str, refresh: bool) -> PyResult<Vec<PyExtension>> // Places::here(), install=false
#[pyfunction] fn builtin_extensions() -> Vec<PyExtension>
#[pyfunction] fn word_of(py, source: &str) -> PyResult<String>
#[pyfunction] fn missing_words(program: Vec<String>, words: Vec<String>) -> Vec<String>
#[pyfunction] fn places() -> (String, String) // config dir, cache dir
```

- Errors use `raised(py, &Made::new(py)?, &Fault::from(error))`, as `gate` does (lines 355-363). They raise `furb.python.Refused`.
- Register everything in `_monty` (lines 653-659).
- `pyo3::Life::new` (lines 285-300) is unchanged: the Python host plays the words itself on both engines, so nothing plays twice.
- `bind/python/furb_monty/__init__.py`: import and `__all__` get `Extension`, `builtin_extensions`, `extensions`, `gate`, `missing_words`, `places`, `word_of`.
- `_monty.pyi`: add stubs with docstrings.

### ts.rs (the `typescript` feature)

Add:

```rust
#[napi(object)] pub struct ExtensionWorld { pub ts: Option<String>, pub py: Option<String> }
#[napi(object, js_name = "Extension")] pub struct JsExtension { pub name: String, pub root: Option<String>, pub word: String, pub requires: Vec<String>, pub world: ExtensionWorld, pub tui: Option<String> }
#[napi(object)] pub struct Places { pub config: String, pub cache: String }
#[napi] pub fn extensions(project: String, refresh: Option<bool>) -> napi::Result<Vec<JsExtension>> // install = true
#[napi] pub fn builtin_extensions() -> Vec<JsExtension>
#[napi] pub fn word_of(source: String) -> napi::Result<String>
#[napi] pub fn missing_words(program: Vec<String>, words: Vec<String>) -> Vec<String>
#[napi] pub fn places() -> Places
```

- The JS names are `extensions`, `builtinExtensions`, `wordOf`, `missingWords` and `places`.
- `JsLife::boot` (lines 131-153) gains `words: Option<Vec<String>>` (ts_args `..., record?: unknown[] | null, words?: string[] | null`) and calls `.words(words.unwrap_or_default())`.

Also remove these typed methods (the step 4 side of the same regeneration):
- `TextValue` and `ExitValue` (21-32), `BashOptions` and `GrantOptions` (40-54);
- `grant` (328-338), `bash` (340-369), `read` and `write` (383-414), `cwd` and `cd` (445-465);
- `span`, `grep` and `differs` (530-541). These are broken now: they call the engine with no chain. TS reaches them by `held("modules", [on, "span"], "at")` and `made(id, ...)`.

Keep `take`. Removing these breaks 10 TS files: `bind/typescript/src/types.ts`, its tests, and `tui/src/{session,demo,snapshots,app}.ts` with their tests. So do the removal in step 4 with the TS rewrite. In step 3, commit only the additions.

Every change of ts.rs needs `bun run build`, which runs `napi build --dts index.d.cts`. Commit `bind/typescript/index.cjs` and `index.d.cts` in the same commit, because CI runs `git diff --exit-code` on them (gates.yml:158).

In `index.d.cts`:
- Added: `Extension`, `ExtensionWorld`, `Places`, `extensions`, `builtinExtensions`, `wordOf`, `missingWords`, `places`, and the `words` parameter of `Life.boot`.
- Removed later (step 4): `TextValue`, `ExitValue`, `BashOptions`, `GrantOptions`, and the methods listed above.

### 5. The Python host

### `src/furb/world.py`

- **Stays in `Live` (the generic core):** SYSTEM, PARTS, wire and unwire, worded, truth, LINES, kept, and the Live fields, plus `buys`, `answer`, `line`, `keep`, `asked` and `show`.
- **Drops:** `Text` from the import (41) and the `Text()` arm of `wire` (85-86). Records hold plain data now.
- **New fields:** `words: list[str]`, `parts: tuple[str, ...] = ("files", "bash")` and `booted: bool = False`.
- **New methods:**
  - `plays(chain)`: `_, program = engine.ask("program", chain)`, then `for w in missing_words(list(program.values()), self.words): engine.rung(w, on=chain)`.
  - `play()`: set the site to WORLD with a token; call `plays` on every `engine.acts` value that is a chain with no source; reset the site; set `booted = True`. This is the same as `conftest.life`, lines 499-505.
  - `where(on)`: `engine.modules[on]["cwd"](on=on)` when the chain binds `cwd`, else the directory of `engine.ask("stand", on)[1]`. It works on both engines through the made handle, as `conftest.where` (lines 267-276) does.
- **`hears`** (428-483) keeps stand, ask, clock, chance and keep, and the starts of wait and prompt. Add:
  - `("chain", id, _, _, _, "") if self.booted`: `self.plays(id)`.
  - A start of an act whose kind is no wait, no prompt and no kind of a part: `engine.close(Refused(f"the World does no {kind}"), about)`.
  - After the match: `for part in parts: yield from part.hears(a)`.
- **`class Files`** takes the read, write, `serves`, `door`, `at` and `CAP` logic of 263-317. It hears `("read", qid, _, on, path)` and `("write", qid, _, on, path, content)` (the new fact shape of `files.py:49`). It answers the plain `{"path", "content"}` or a `Refused`, and resolves against `live.where(on)`. `kinds = frozenset()`.
- **`class Bash`** takes `Command` (159-203) and `ran`/`told` (330-382). It hears the start of `bash` (merged through `engine.ask("merged", on, about)[1]`, the directory through `live.where(on)`), `feed`, `cancel`/`close` (slay and yield `exited`), and `exited`. `kinds = frozenset({"bash"})`.
- `PARTS = {"files": Files, "bash": Bash}`. A part is made only when its name is among `parts`.

### `src/furb/cli.py`

- `lived()` (30-41):
  - `loaded = furb_monty.extensions(str(cwd.absolute()))`;
  - `world = Live(..., words=[e.word for e in loaded], parts=tuple(e.name for e in loaded))`;
  - `boot` as now, then `world.play()`.
- `main()` (109-117) catches `Refused` from the config and says `SystemExit(f"furb: {no}")`.
- Optional: a subcommand `update` that calls `extensions(..., refresh=True)` and prints the names, as the explicit refresh the design names.
- The CLI loads the python words of external extensions and no world part (decision 4).

### test/outside

The core suite belongs to the other agent, so coordinate with them before you edit `test/outside`.
- An autouse fixture in `test/outside/conftest.py` sets `FURB_CONFIG_DIR` and `FURB_CACHE_DIR` to `tmp_path`, so a home config of the developer changes no test.
- `doubles.life` (137) calls `world.play()` after boot. `test_world.world()` (36-38) gives `words=[e.word for e in builtin_extensions()]`.
- `test_world.py` stops importing `TIMEOUT`, `Exit` and `Text` from `furb.engine`. It calls the verbs through `Bound(root)` from `conftest` and compares `.path` and `.content`, not `==` of a `Text`: the class in the life is not the class of the module.
- `test_world.commanded` moves to `furb.world.Bash`.
- New tests:
  - a home config with `grant: false` gives a root that binds no `grant`;
  - a start of an unknown kind is refused;
  - a chain born after boot is played;
  - a config error exits the CLI.
- `test_monty.py:39,60,197-202` use `engine.read`, `span` and `read = 1`. They are the other agent's.
- The suite agent switches `conftest.BUILTIN` (line 35) to `{e.name: e.word for e in furb_monty.builtin_extensions()}` once commit 2 lands.

### 6. Risks and open questions

1. **Parser pin:** the pins of `ruff_python_*` must move with every bump of monty's `rev`, or cargo builds a second copy.
2. **Changed words:** a changed extension word (an upgrade) is a new string. A later life plays it as one more rung on chains whose record holds the old one. Both stand in the turns, and the later one binds last. Ask the owner.
3. **Refused words:** a word the gate refuses never enters the program, so it is played again at every boot and every chain birth. Nobody reports it. Should the host log it?
4. **Paused chains:** a word played on a paused chain waits for the wake (engine.py:219-220).
5. **Cost of forwarding:** the typed Rust World now hears every other fact, one host call each. A `kinds()` filter could come later.
6. **Raw facts:** raw facts go to the typed World. An instance of a class of a word that comes back in the args of a `Reply::Calls` goes through `again` and becomes a plain dict. `covers` reads only `a[0]` and `a[1]`, but it needs a test.
7. **Two copies of one rule:** the missing-words rule has two copies, the preamble and `extension::missing`. The life tests and the extension tests pin each.
8. **An off requirement is an error, not a quiet drop:** `{"files": false}` alone refuses bash. The message must say to turn bash off too. This follows "refuse one whose requirement is off". Confirm with the owner.
9. **No upward search:** the local config is `<cwd>/.furb/config.json`, with no search up the tree.
10. **`FURB_CACHE_DIR` is new:** the ledger does not name it.
11. **npm `latest` goes stale:** an npm entry with no version is kept as `<name>@latest` until a refresh.
12. **Blocking napi call:** the napi `extensions` call is synchronous (git and npm on the JS thread). Step 4 may wrap it in an `AsyncTask`.
13. **Git ref:** it is a branch or a tag (`clone --branch`). A commit sha fails.
14. **Old records do not open:** they hold `{"is": "Text"}` and the old write shape, so they drift or raise KeyError in `unwire`. Say so in the release note.
15. **Tokens in the first turn:** the builtin words stand in the turns of every root. That costs tokens in the first turn, but hygiene law 5 counts only engine.py.
16. **Windows tests:** the npm and git tests run on windows-latest (`npm.cmd`, bsdtar). The tests must use `Path::join` only.

### 7. Commits, each with its gate

1. **"Add the extension module of the crate"**: Cargo.toml, Cargo.lock, `src/extension.rs`, `src/extension.test.rs`, and the `pub mod` line in lib.rs.
   - Gate: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo clippy --all-targets --features typescript -- -D warnings`, `cargo test extension`.
   - The rest of `cargo test` is red at HEAD (section 0).
2. **"Give the extension API to python"**: py.rs functions and the class, `furb_monty/__init__.py`, `_monty.pyi`, and PURE/ACTS in `furb_monty/engine.py`.
   - Gate: `uv sync --reinstall-package furb-monty`, `uv run python -c "import furb_monty as m; print([e.name for e in m.builtin_extensions()])"`, `uv run ruff format --check src test script bind`, `uv run ruff check ...`, `uv run ty check --error-on-warning`.
   - After this commit, tell the suite agent that `word_of` and `builtin_extensions` exist.
3. **"Let the World of a Rust host hear every other fact, and play the words of the extensions in the life"**: preamble, world.rs, life.rs (worldly, Hosting, Opening.words, boot hook, `awaited`), life.test.rs, and the gate.rs tests.
   - Gate: `cargo fmt --check`, both clippy runs, `cargo test` (all of it now), `cargo test --features typescript`, `uv sync --reinstall-package furb-monty`.
   - Also run `uv run pytest -q -n0 --no-cov test/outside/test_monty.py`: no new failure compared with before.
4. **"Remove the typed builtins from the crate"**: the Life typed methods and Came impls, value.rs Text/Exit, lib.rs, ts/wire.rs, the py.rs comment.
   - Gate: the same as commit 3, plus `cargo check --features typescript`.
5. **"Give the extension API to TypeScript"**: the additions to ts.rs, the `words` of boot, and the regenerated `index.cjs` and `index.d.cts`.
   - Gate: `cargo clippy --all-targets --features typescript -- -D warnings`, `cargo test --features typescript`, `bun run build`, then `git diff` of `index.d.cts` shows only the additions, then `bun run check` and `bun test bind/typescript/test` show no new failure.
6. **"Play the extensions in the python host"**: world.py, cli.py and test/outside (coordinated).
   - Gate: `uv run ruff format/check`, `uv run ty check --error-on-warning`, `uv run pytest -q test/outside` on both engines with coverage of `furb.world` and `furb.cli` whole.
   - Then, once the suite agent is done: `uv run pytest -q`, `uv run pre-commit run --all-files`.

### Critical Files for Implementation
- /home/user/furb/src/extension.rs (new) and /home/user/furb/src/extension.test.rs (new)
- /home/user/furb/src/preamble.py
- /home/user/furb/src/life.rs (with /home/user/furb/src/life.test.rs)
- /home/user/furb/src/world.rs
- /home/user/furb/src/binding/py.rs (with /home/user/furb/bind/python/furb_monty/__init__.py, _monty.pyi, engine.py) and /home/user/furb/src/furb/world.py
