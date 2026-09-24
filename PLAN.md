# Plan: the rest of the extension system (delete with LEDGER.md when the work is done)

LEDGER.md holds the request, the decisions of the owner and the state. This file holds the plan of the steps that
remain, as the lead checked it against the code. Part 1 is binding: where a plan below says otherwise, part 1 wins.
Part 2 is the plan of the TypeScript host and the TUI, part 3 the plan of the skills extension, the proofs, the
documents and the releases. The plan of the crate and the Python host comes as part 4 when it is done.

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
13. The last line `skills()` of `extensions/skills/skills.py` stays for now, so a chain tells its skills before its
    first ask. The owner may drop it; the lead asks.

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