# Extensions

An extension adds to what a model can do in furb. It is Python, as a word of a model is: a life runs the word of each
extension on each chain where it plays, before any other word, so the functions that it defines are there for the model,
and the model reads the extension in its turns as it reads its own work. In the setup of the root, which every new chain
holds, each extension is two turns: a turn of the model whose word is `rung(read('<file>', HIDDEN))`, and the turn of
the rung that this word made, which holds the code of the file alone, since that rung runs it. A rung of a text runs the
code in the module of the extensions of its chain, which holds the names of the engine, and binds each name that the
code defines in the module of the chain too, so the model calls each function of the extension, and a name that the
model binds, as `picked`, never reaches the code of an extension. A project changes a default of an extension with a
skill of its own, whose word runs in the same module. The engine crafts these turns itself from the extensions that the
World gives in the standing. The file is a copy of the word of the extension in `words/<name>/<digest>/` of the config
directory of the user, which holds no import of furb. The digest is the start of the sha256 of the word, so a life of
another build in the same config directory writes its own word beside this one, and never over the file that this life
reads.

An extension never goes into the system prompt. So a chain that takes one only grows at its end, and what a provider
caches of the chain stays valid.

## The official extensions

Ten extensions come with furb, each a skill with code. `catchup`, `memory`, `skills`, `compaction` and `naming` play
by default, as their manifests say, and `furb-kit`, `web`, `mcp` and `grant` play on need, as the rule below
says.

### Which skill plays on which chain

One rule says it. A skill with code plays in one of three ways:

- By default: an official skill whose manifest says `furb-default: "true"` plays from the setup of the root, unless a
  config turns it off.
- By a config: a config that turns a skill on, by `true` for an official one or by the path of its folder for any
  other, plays it from the setup, or from the write of that config in the middle of a life.
- On need: `await skill(name)` plays it from then on, as a word loads `web` or `mcp` when its work needs them.

A skill that plays, plays on every chain of the life, and one whose manifest says `furb-plays: root` plays on the root
alone and on a chain whose source had it. The index, `skills://`, marks `on` each skill that plays on the chain that
reads it.

- **`catchup`** keeps a chain up to date with the machine. At each stand, as when a later life opens on a record, it
  reads again each file that the chain read or wrote, and tells the chain in one paragraph headed caught up the lines
  that changed, or their count when more than 40 changed, `gone` for a file that it can no longer read, and the hours
  since the life before when they are more than one. Its life word is `catchup()`. A chain where nothing changed hears
  nothing.
- **`memory`** tells a chain the memory files of its project. The memory of a folder is its `AGENTS.md`, or its
  `CLAUDE.md` when it has no `AGENTS.md`, then its `AGENTS.local.md`, or its `CLAUDE.local.md` when it has no
  `AGENTS.local.md`. A local file holds what is true on one machine alone, and git ignores it. When a chain starts, it
  reads the memory of the user, in the config directory that a word reads as `config://`, then the memory of each
  folder from the root down to the working directory. It finds each file with reads of folders, in the word alone.
  Each time the chain reads a file in a folder below, it reads the memory of each folder on the way. A memory file
  comes whole the first time, and again only when it changed. Its life word, `await recall()`, reads each memory file
  in the setup of the root, which every new chain holds, and then `remember()` does the rest as the chain works.
  `memory(path)` tells the memory of one path.
- **`skills`** gives a chain the skills that it can read. `skills()` lists each skill that it finds in `.furb/skills`,
  `.agents/skills` and `.claude/skills` of the working directory and of each folder above it, then in `skills` of the
  config directory, then each extension of the project and of the configs and each official one, which is a skill with
  code: its line marks `code`, `on` when the life runs it on the chain, and `default` when it plays by default. The
  first line of the list says how to load and read a skill. `await skill(name)` loads a skill on the chain: it tells
  the `SKILL.md` of a skill with no code, and it tells the prose and the interface of a skill with code, or its word
  when it ships no interface, after the life enables it, once for each word, so it plays as an extension that a config
  turns on. `await skill()` plays each skill with code that says `furb-default: "true"`. `read("skills://<name>")`
  gives the word of a skill with code, and `read("skills://<name>/<file>")` a file of its folder.
  `defined(*names)` is a `Show` that selects the definitions of those names in a text, so
  `read("skills://<name>", defined("f"))` tells those definitions and gives the whole word to edit and write.
  `read("settings://<name>")` gives the keys of a skill in the configs as a JSON object, each with the value that wins
  for it or the default of its `furb-config`, and `read("settings://")` each key of the configs, as
  [the schema](config.md#the-schema) says. A write of
  `skills://<name>` writes the word in `.furb/skills/<name>` of the project, after a copy of the folder there
  when it stands elsewhere, and reloads the skill when the life runs it. A word that binds a name of the engine at its top is refused, and the write changes no file. Its life word, `await listed()`, reads
  `skills://`
  in the setup of the root, which every new chain holds.
- **`compaction`** jumps a chain to a chain with the work that it still needs, and the work goes on there, while the
  turns of the chain stay as its model read them. `compact()` keeps what `FILTERS` keep, binds a summary in the first
  word there, and reads again with `reground()` each file that it kept,
  showing only what changed. The filters are `latest()`, the last read or write of each path, `quiet()`, which drops the
  commands, `recent(n)`, the acts of the last `n` replies, and `await chosen()`, which asks the model of the chain.
  `together(*filters)` joins them, and `supersede(keep)` applies one. `await condense()` asks the model of the chain what to keep and
  compacts at once. `compacted()` does the same for one chain each time an answer fills more than `SHARE`, 0.6, of the
  window, and stops and pauses nothing. It compacts again only once an answer reads `AGAIN`, 0.15, more of the window
  than the first answer after the compaction before, so a compaction that leaves the chain near `SHARE` is no loop. `fitted()` watches a chain: when the model refuses a reply because its
  prompt passes the window, it compacts the chain at once to its open prompts, with a summary that says why, so the next
  reply reads less and the chain goes on with no pause. Its life word, `compacted(); fitted()`, wraps each chain with
  both, so a chain compacts to what a model chose, with its summary, before its prompt passes the window, and keeps
  its reads and its findings. The compaction of `fitted()` comes only as the last resort, when one reply grows past the
  window before the compaction. Each compaction costs one turn more on the chain, to the model that wrote its last
  reply, which reads the chain from its cache.
  A project changes `SHARE`, `AGAIN`, `FILTERS` or `BRIEF` in an extension of its own.
- **`naming`** gives the session a name from the first message of the operator. Its life word, `name_session()`, plays
  on the root alone. At the first prompt that the operator makes in the life, even one made before the word plays, it asks the actor that its `automatic()`
  gives for a name of a few words, on a chain of its own labelled `naming`. With no such actor, or with no
  name from it, the name is the first line of the message. The act is done with the name, and the record keeps the
  prompt and its answer, so a later life gives the same name and asks no model. `rename_session(name)` says the
  name that the operator chose, as an act of the root, and it wins over the name of `name_session()`. Each client
  shows the name that the record holds, and the sidebar hides the chain `naming`. A
  config that turns `naming` off, as `{"skills": {"naming": false}}`, saves the call of that model. A rename in a
  client is the same act, which the client makes with an ear of its own, so it lasts whatever extension plays.
- **`furb-kit`** gives `extend(name, *rungs)`, which writes the words of those rungs as an extension of the project
  and turns it on, so every later chain runs it. `extend(name, *rungs, session=True)` keeps the kit in the record alone: it writes no
  file and says the fact `enable` with the word, so each chain of this session, and of each later life on its record,
  plays it. `view="name"` names in that fact the value of the root that holds the tree of the view of the kit, a node
  of the protocol as a dict: the daemon serves it to each window that mounts the view of the kit, and again each time
  that the value changes. How to extend furb stands in one home, the text of its
  [`SKILL.md`](../.agents/skills/furb-kit/SKILL.md), which a model reads with `skill("furb-kit")`, or the operator with
  `/furb-kit` and a request: the three ways to extend furb and when to take each, a kit of the session with a view, and
  the parts of a kit. The skill `furb`, with no code, says how to drive furb from its command line and points at
  `furb-kit` and `furb-develop`. `enabled()` gives the extensions that the life runs. `bound_by()` gives each name that a word of the chain bound, with the rung of that word.
- **`web`** searches the web and reads its pages. `web_search(query)` uses the search of the provider of the model of
  the chain, or of the actor that `to` names: the web search of the APIs of Anthropic, OpenAI and xAI, Gemini grounded
  in Google Search, and the search of DeepSeek. For a provider with no search, it uses the free search of Parallel for
  agents, which needs no key. `web_fetch(url)` reads a page, as Markdown when it is HTML. Each is an act to await, as a
  command is, since a page takes seconds. The extension has no life word: `web_search` asks the question `search`,
  which the provider answers when it has a search of its own, and the process of the kit answers the rest and each
  question `fetch` of `web_fetch`.
- **`mcp`** connects the chain to the servers of the Model Context Protocol. A server comes from three configs, and the
  later wins for a name: `mcp.json` in the config directory of the user, `.mcp.json` of the project, which has the form
  of Claude Code, and `.furb/mcp.json`. Each holds `{"mcpServers": {name: server}}`: a command, its `args`, `env` and
  `cwd` for a server over stdio, or a `type` of `http` or `sse` with a `url` and `headers` for a remote one. `${VAR}`
  and `${VAR:-default}` expand in each of them, `"timeout"` bounds a call in seconds, `"deny"` hides tools, and
  `"disabled": true` turns a server off. A server of `.mcp.json` runs only once the operator approves it. The process
  of the kit connects each server, ends each server over stdio with the life, connects a lost server again with a
  backoff, and signs in to a remote server with OAuth 2.1 when it asks for a login: the token stays in the Keychain on
  macOS, else in a file that only the user reads, which the key `tokens` of the kit, `{"mcp": {"tokens": "file"}}`,
  names on macOS too, as on a Mac reached over SSH, whose Keychain is locked. Its life word, `await mcp_index()`, tells each chain one line for each
  server, with its state and its tools, and binds the tools of each short server as a class of typed functions, so a
  model writes `await github.create_issue(owner="o", repo="r", title="t")` and reads the name and the type of each
  argument in the signature. `mcp_use(server)` binds a larger server, `mcp_tools(server)` and `mcp_tool(server, name)` tell signatures and
  schemas, `mcp_call(server, tool, arguments)` calls a tool by its original names, `mcp_read(server, uri)` reads a
  resource, `mcp_prompt(server, name, arguments)` expands a prompt, and `mcp_control(action, server)` reconnects,
  enables, disables, approves, logs in or logs out. Each answer of a server stays in the record, so a later life asks no
  server again.
- **`grant`** puts a ceiling on a chain. `grant(usd, share)` holds the ledger of the chain: at each answer of a model
  it tells the dollars since the grant, and it pauses the chain when they reach `usd`, or when the answer read more than
  `share` of the window. A later grant on the chain closes the one before it, and a wake lets the chain go on. A word
  loads it with `await skill("grant")`, and the `grant` of each door, and the `Grant` door of the host, run that word on the chain.

`compaction`, `naming`, `web` and `mcp` each hold a short guide for the model, a quote named after the extension, which
the model reads on each chain where the extension plays.

## Turn an extension on or off

Three configs say which extensions a life runs, in this order: `config.json` in the config directory of the user,
`.furb/config.json` in the directory of the project, then `.furb/config.local.json` beside it. A later config wins
for a name. Git keeps `.furb/config.json`, and ignores `.furb/config.local.json`: a project keeps its local
extensions there, as the ones that serve this machine alone. The config directory is
`FURB_CONFIG_DIR`, else `furb` under `XDG_CONFIG_HOME`, else `furb` under `~/.config`, or under `%APPDATA%` on
Windows.

```json
{ "skills": { "memory": true, "skills": false, "notes": "~/src/furb-notes" } }
```

- `true` turns an extension on, and keeps the path that an earlier config gave.
- `false` turns an extension off.
- A path turns on the extension in that folder. A relative path starts at the folder of the config, and `~` is your
  home.

A config of another form, a folder with no manifest, a manifest of another name, and a missing word are each refused
with their path. The `.furb` of a project keeps all that it holds out of version control but `config.json` and
`skills/`, since the extensions of a project are code of the project, which its team shares.

## Write your own

The smallest extension is a function that a model wrote in a word. When a later chain needs it too, a model calls
`extend(name, ...)` of `furb-kit`, which writes the extension for you. You can also write one by hand: an extension
is a folder.

```text
.furb/skills/<name>/
  SKILL.md      the manifest
  <name>.py     the word
  <name>.pyi    the interface, which it can have
```

The manifest is the file of a skill, so Claude Code reads the folder as an ordinary skill. Its frontmatter gives the
name and the description, and the fields of furb under `metadata`, each with the prefix `furb-`. The [open format of
skills](https://agentskills.io/specification) puts there the fields of a client: "Arbitrary key-value mapping for
additional metadata (a map from string keys to string values)." Its body is the prose of the extension.

```markdown
---
name: notes
description: Keeps the notes of a chain.
metadata:
  furb-word: notes.py
  furb-life: notes()
  furb-interface: notes.pyi
  furb-default: "false"
  furb-plays: every
  furb-process: notes.ts
  furb-acts: notes_add
  furb-config: config.json
---
What the notes are for.
```

- `name` is the name of the extension: a lower case letter, then lower case letters, digits and `-`. It must be the
  key that the config gives it.
- `description` says what the extension does and when to use it, as for any skill.
- `furb-word` is the file of the word, relative to the folder. It is a Python module that imports what it uses from
  `furb.engine`, so an editor, ruff and ty read it. The word that a chain runs is that file less its imports of furb.
  It only defines: it holds no comment and binds no name of the engine. It can hold one quote of guidance for the
  model, which holds no law.
- `furb-interface` is the file of the interface of the extension, its contract. It can be absent.
- `furb-default` is `"true"` for an extension that plays by default, as `catchup`, `memory`, `skills`, `compaction`
  and `naming`. It can be absent, which is `"false"`: the extension then plays only when a config turns it on or a
  word loads it with `skill(name)`.
- `furb-life` is the life word, a line of Python that starts the work of the extension, such as `await recall()`. It
  awaits each act that it starts whose first answer comes later, as `await mcp_index()` does, so what that act tells
  stands in the turn of the life words of the setup. It can be absent.
- `furb-plays` says where the extension plays: `"every"`, on every chain, which is the default, or `"root"`, on the root
  chain alone. A chain with a source keeps what its source played, so a chain that goes on from the root has an
  extension of the root, and a new chain with no source has none. Give `"root"` to an extension whose names only the
  work of the root needs, as a loop that conducts workers: each other chain then reads none of its words, and a model
  there is free to bind each name that it defines.
- `furb-process` is the file of the process of the extension, its World part, relative to the folder: a module of
  Python, a module of TypeScript or of JavaScript, which bun runs, or any other program. A module of Python whose
  header names what it needs, as [PEP 723](https://peps.python.org/pep-0723/) says, runs on `uv run --script`, which
  makes that environment once. furb takes the uv that `FURB_UV` names, else the one beside the program of the life,
  else the one on PATH; with none, it fetches uv 0.13.1 for the system into its cache once, and checks it against the
  sha256 that furb holds for that release. A fetch or a check that fails refuses the act and says how to install uv by
  hand. Another module of Python runs on the Python that `FURB_PYTHON` names (the door of Python names its own), else
  the one beside the program of the life, else `python3`. An official extension in Python finds `furb.skill` of the
  build that hosts it beside the copy of its module.
  The life starts it in the folder of the skill at the first act of a kind that `furb-acts` names, never at the open,
  so a replay starts no process: the record answers each act before its tip. The life speaks to it on its stdin and its stdout, one frame of JSON for each line, and gives it the
  variables `FURB_LIFE` and `FURB_SKILL`, the record of the life and the name of the extension. Each life does so, the
  daemon of the machine, a door and the rig alike. The life ends the process with each process that it started when the
  life ends, and when a new fact `enable` of the extension reloads it, so the next act starts the new process. A
  process that ends before its life tells the operator once, with its exit and the last lines of its stderr, and each
  act that it held is refused with the same reason. A process keeps working on each later build, as
  [the protocol](protocol.md) says of a kit. It can be absent.
- `furb-acts` names the kinds of the acts that the process answers, separated by spaces, as `notes_add notes_drop`.
  The life takes each act of these kinds, says that it started, and gives it to the process, which closes it. It can be
  absent.
- `furb-config` is the JSON file of the keys that the extension adds to the configs, relative to the folder, as
  [the schema](config.md#the-schema) says. `/config` shows them under the name of the extension, and its word reads
  them as `json.loads(read("settings://<name>", HIDDEN).content)`. It can be absent.

The client `@uael/furb/skill` gives a process of TypeScript what it needs: it attaches the life on its stdin and its
stdout, answers the acts of its kinds, hears the facts of the kinds that it asks, and says verbs that the life takes.
A process draws nothing: the word of the kit draws its views through the life.

```typescript
import { Skill } from "@uael/furb/skill";

const skill = await Skill.attach({
  kinds: [],
  answering: () => ({ notes_add: (fact) => `added ${String(fact[4])}` }),
});
await skill.closed;
```

The life gives the acts that wait for the kit as soon as it attaches, so a kit gives its answers to `attach`, which
makes them from the welcome of the life, as its folders, before it attaches. `answers` adds or removes one later.

The module `furb.skill` gives a process of Python the same calls, on asyncio, and `furb.protocol` the types of the
protocol:

```python
import asyncio

from furb.skill import Skill


async def main() -> None:
  skill = await Skill.attach(kinds=[], answering=lambda _welcome: {"notes_add": lambda fact: f"added {fact[4]}"})
  await skill.closed


asyncio.run(main())
```

The word of the kit makes each act of such a kind as any act of the World, under a kind that starts with the name of
the kit, so no other kit answers it:

```python
def add(item: str) -> Act[str]:
  return act("notes_add", "", None, item)
```

The process closes the act with what its function gives, awaited when it is awaitable, or with a refusal that names
what it raised. A done of the act that comes first, as a cancel, cancels that function, and the process closes
nothing. A word that awaits `add("milk")` waits until the process answers, and a later life takes the answer from the
record. `skill.answers(kind, answer)` of TypeScript does the same, and gives `answer` a signal that the done aborts.

A field of furb that the manifest does not know is refused with its path, so a typo does not pass.

A word reaches the machine only through the acts that furb has already: a read, a write, a command, a wait, the
clock, a chance, and a prompt to the operator. It can also make acts that an ear of its own answers, written in
Python, as `remember()` does, and answer the reads of a scheme of its own. An extension that needs an ear in Rust is
a change of furb. The skill `furb-kit` gives an example of each.

A word sets the actor of a chain that it made, or of a chain that waits, with a rung on that chain: `rung("actor =
'claude-cli:sonnet/medium'", on=chain)` rebinds the default actor, and `wake(chain)` lets a paused chain go on. The next
prompt on the chain with no `to` goes to the new actor. The note that a model
limit tells a chain names the same two calls.

An official extension also has a contract, `<name>.pyi`, whose laws its suite in `test/` holds, as [the hygiene
laws](../AGENTS.md#the-hygiene-laws) say, and an ear of the World in `src/world/<name>.rs` when it reads the disk or
the network, as `skills` and `compaction` do. The search and the fetch of `web` are questions of the World, which the provider and the process of the kit answer, and the client of `mcp` is the process of its kit.

Turn on only the extensions that you trust. A word runs in the sandbox of monty with the engine of the crate, and with
the rights of your user with the engine of CPython, as a word of a model does there.

## The codes of a command

A word that awaits a command is refused when the command exits with a code that it does not accept, and `peek` gives
its `Exit`. `ok` names the codes of one command, as `ok=(0, 1)`, and `ok=True` accepts every code. A command with no
`ok` is judged by `accepts`, a name of the engine that is `None` until a word binds it: then the default policy accepts
the code 0, and the code 1 of a grep, an rg, a test, a diff, a cmp or a git check that ends the command and wrote no
error. A word binds `accepts` to a function of the command and its `Exit` for each later command of its chain, and an
extension that plays on every chain binds it for the whole project:

```python
def accepts(command, exit):
  return exit.code in (0, 1)
```

## Permissions

furb has no permissions, by design. Each act runs with the rights of the user who runs furb, and the record shows who
made each act. For isolation, run furb in a virtual machine.

An extension can add a check: its word can define a verb again, such as `bash`, so that the verb asks you before it
makes the act. A word of the model can define that verb again too, so such a check stops a mistake and not an
attack. furb has no such extension built in.

## How a life runs its extensions

A life enables each extension by a fact, `enable`, which the record keeps, so a later life on the record runs the
same words. [`src/furb/AGENTS.md`](../src/furb/AGENTS.md) says what the fact means and when a life says it, and the
ear `extensions` of [`src/skill.rs`](../src/skill.rs) says the fact and plays it.

The extensions that a life enables at its start are those that the configs turn on. When a write of
`.furb/config.json` or of `.furb/config.local.json` lands in the middle of a life, the life enables each extension that the config turns on by its
path and that it does not run yet. A change of `skills` through `/config`, `/extensions` or the protocol enables at
once the extension that it turns on. An extension that a config turns off runs on until the life ends.

`/extensions` in the window, or ⌃X X, lists each skill of the session: the extensions that play first, then those that
play on need, then the skills with no code, each with what it is for, how it plays and where it comes from. Enter
turns an extension on or off in `.furb/config.local.json`, and adds `/<name>` of a skill with no code to the input.

A life reloads an extension that it runs when its word changes: at its start, or at a later stand, for the word that the
host gives, as a new build gives a new word of an official extension, and when a write of the life lands in the folder
of an extension that a config turns on, as `extend()` does. The life enables it again with the new word, and each chain
plays the definitions of that word: its imports, its functions, its classes, its types and its names in capitals, and
each name of state that it assigns, as `queue: list = []`, in file order. So a function and a constant take their new
value, a new name of state takes its first value, a name of state that the chain holds, such as `queue`, keeps its
value, and the life word does not run again. A change of a file that no write of the life makes reaches the life at its
next start. A host that gives no extensions enables none.

Each thing that comes from the disk reaches a chain through a question that an ear of the World answers, and a later
life gets the same answer from the record. So a replay is exact, and the World is asked nothing twice.

## In each host

- The command line: `furb skills` prints each skill that a life on a record runs, with its life word. The
  `state` of the protocol names them.
- TypeScript: `Session` takes `skills`, false to enable nothing new, and `config`, the config directory of the
  user.
- Both doors: `opened` gives the record and the ears of the crate, and `skills` false enables nothing new.
  `enabled`, on the facts of the root, gives the extensions that a life runs, in the order that it enabled them.
