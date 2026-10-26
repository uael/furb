# Models

furb asks a model for each step of a life. This page says which models it finds, how to choose one, and how to run
one on your own machine.

## Where the models come from

furb looks in three places, in this order:

1. **The claude command line.** When furb finds the program `claude`, it offers its models, as `claude-cli:opus`,
   `claude-cli:sonnet`, `claude-cli:haiku` and `claude-cli:fable`. Furb asks the program for them: its aliases, the
   window of each and the efforts that each takes. The question costs no model prompt. Furb keeps the answer in its
   cache until the program changes, and `/reload` asks again, so a new model or a new window shows with no new build
   of furb. An older program that does not answer gives the four models above, with the windows that furb knows. They
   use the plan that you signed in to `claude` with. furb finds the program that `FURB_CLAUDE_BIN` names, else the `claude` on PATH or in your home, else, on
   macOS, the one that Claude Desktop installed.
2. **A provider of the network.** furb offers the models of each provider whose key stands in the environment, as
   `ANTHROPIC_API_KEY` or `OPENAI_API_KEY`. It knows the providers that [models.dev](https://models.dev) and pi-ai
   list.
3. **A local server.** When `FURB_LOCAL_URL` gives the address of a server, furb offers the models that it lists.
   [A local model](#a-local-model) says more.

The first model that furb offers is the default. So it is `claude-cli:opus` when you have the claude command line,
else a model of the network, else a local model. The TUI checks each place on its first run, and says how to fix
each one that is missing.

## Choose a model

A model has a name of the form `provider:id`, as `claude-cli:sonnet` or `local:qwen3`. When one model alone has an
id, the id alone names it, as `sonnet`.

An **actor** is a model and an effort, which is how hard the model thinks: `off`, `minimal`, `low`, `medium`,
`high`, `xhigh` or `max`. Write the effort after a slash, as `claude-cli:opus/medium`. An actor with no effort takes
`high`, and an effort that a model does not take moves to the nearest one that it takes.

- In the TUI, `/model` and `/effort` choose the actor of the chain on screen. ⌃M and ⇧Tab do the same.
- On the command line, `--model` gives the default actor, as `furb --model claude-cli:sonnet/low`.
- `--roster` gives more models that a life can ask, one name each time. With no `--roster`, a life can ask each
  model that furb offers. A prompt to a model that the roster does not hold is refused.

## Provider settings

For a ChatGPT subscription, open `/providers` and choose `codex`. Furb uses an existing ChatGPT sign-in when
one is available. Otherwise, it opens the browser for sign-in. When it connects, the Codex model picker opens.
Choose a model there. No API key, server, model id or context size is needed.

`codex` uses the limits of the ChatGPT subscription. `openai` remains the provider for an OpenAI API key.
Claude Code stays available by default. Connecting Codex does not change a running model call.

The account service supplies the model list, the input window, image support and the supported reasoning efforts.
Furb reserves the input headroom that the service specifies, or 5% when it leaves that value unsaid. For a model
with a 500,000-token default window and 5% headroom, the picker shows 475,000 input tokens.

To request a larger window, set `model_context_window` under the `codex` provider in `.furb/config.json`:

```json
{"providers": {"codex": {"model_context_window": 1000000}}}
```

As in Codex, the value is capped at the maximum window from the account service before the input reserve is
applied. With an 872,000-token maximum and a 95% usable share, this setting gives 828,400 input tokens. Without
this setting, furb uses the default window from the account service.

Model data is cached for five minutes and checked when a life opens, when the Codex model picker opens, and on `/reload`.
If a refresh fails for a temporary cause, the last model list stays available. A changed model list or window
updates the standing when the operator reloads it or opens the Codex picker.

Furb keeps its sign-in in the user config directory, under `auth`, outside the project. On Unix, the credential
file is readable and writable only by its owner. Furb refreshes its own credential when needed. It can read an
existing Codex access token, but it does not use the Codex refresh token or change the Codex credentials.

Disabling `codex` removes its models from this project and keeps the sign-in for a later connection.

Run `/providers` in the TUI to add or remove a provider for the current project. A built-in provider uses the
credential in the environment. To add a server that uses OpenAI chat completions, enter:

```text
/provider add office http://127.0.0.1:8080/v1 qwen3 262144 OFFICE_KEY
```

The arguments are the provider name, URL, model id, context window and, when needed, the name of the environment
variable that holds its key. The optional last argument selects the protocol, which is `chat` when unsaid.
Use `openai` for OpenAI Responses, `anthropic` for Anthropic Messages, or `gemini` for Google Gemini.
Use `-` for the key name when the server needs no key and a protocol follows it.
`/provider remove office` disables the provider and keeps its settings for a later add.

The same settings can be written in `config.json` in the user config directory, or in `.furb/config.json` in the
project. The project wins for each field that it gives. Other config fields stay as they are.

```json
{
  "providers": {
    "openai": false,
    "office": {
      "protocol": "chat",
      "url": "http://127.0.0.1:8080/v1",
      "key_env": "OFFICE_KEY",
      "models": {
        "qwen3": { "window": 262144, "output": 32768, "reasoning": true, "images": false }
      }
    }
  }
}
```

A built-in provider can be `true`, `false`, or an object that overrides its URL, credential or models. A custom
provider needs a URL and at least one model. Each model needs a positive window. Its output limit is one quarter
of that window when unsaid. `reasoning` and `images` are false when unsaid. The optional `key` field gives a key
directly; `key_env` wins when both are present. A custom server with neither field sends no key. An object can set
`enabled` to false to disable a provider and keep its other fields. The names `claude-cli` and `local` can be false
to remove the command line or the server that the environment names from the roster.

To start Claude through a wrapper, set `providers.claude-cli.command` in `.furb/config.json`:

```json
{
  "providers": {
    "claude-cli": {
      "command": ["cswap", "run", "3", "--"]
    }
  }
}
```

The first item is the executable. Each later item is one argument, before the flags that furb gives Claude for
completions and web search. Spaces stay within an item. No shell reads the command, so shell operators and variable
references have no special meaning. An executable name uses PATH; a path selects that file. The command can also
be a single executable string, such as `"command": "/opt/bin/claude"`, with no arguments.

The `--` in the `cswap` example separates its account options from the Claude flags that furb appends. Without
that separator, `cswap` rejects those flags.

A configured command replaces the default Claude executable, including `FURB_CLAUDE_BIN`. Without `command`, furb
keeps its default discovery. An empty array or executable is invalid. Set `"enabled": false` in the object to
disable Claude without removing its command. Project fields override user fields; a project `true` or `false`
changes only whether the provider is enabled when the user config holds an object.

An optional model `cost` gives `input`, `output`, `cache_read` and `cache_write`, in dollars for one million tokens.
A custom model has no dollar price when `cost` is unsaid, so a dollar ceiling cannot bound its cost.

Run `/reload` after an external edit. The running life reads the provider settings and its thinking limit again,
then stands on the new roster. Existing model calls finish with the settings they started with. New calls use the
new settings. If the default model was removed, the first remaining model becomes the default, or the operator
when no model remains. An invalid config shows its error and leaves the current roster in use.

Every host can request the same reload with `engine.stand()`. The model list functions `models`, `model` and
`setup` read the current settings and accept an optional project directory and user config directory. A host that
gave an explicit roster keeps that limit: it can choose an added model when it opens a life with that model in
its roster.

## Thinking

`MAX_THINKING_TOKENS` gives the most tokens that a model can think in one turn, as it does for Claude Code, and 0
turns thought off. The key `max_thinking_tokens` of `.furb/config.json` gives it for one project. The environment
wins over the config, and with neither, it is 32768. A model with adaptive reasoning, such as Opus 5.5, Sonnet 5.5,
Haiku 5.5 or a Fable model, thinks as its effort says and cannot turn its thought off.

## A local model

A local server costs nothing, and your work stays on your machine. furb talks to each server that serves the chat
completions of OpenAI under `/v1` and lists its models at `/v1/models`, such as llama.cpp, LM Studio, Ollama, vLLM
and Strata:

```sh
FURB_LOCAL_URL=http://localhost:11434 furb --model local:qwen3/medium
```

- Each model that the server lists for chat is named `local:` and its id. `local:` and the id is the sure name: a
  name such as `mistral:7b` names the provider `mistral`, and `local:mistral:7b` names the local model. An id alone
  names a local model only when it holds a colon, as `qwen3:8b`, and, when the part before the colon names a
  provider, only once the server listed it.
- A command that names a local model that furb did not list yet waits for the list, five seconds at most.
- `FURB_LOCAL_KEY` gives the key of the server, when it takes one.
- A local model takes the efforts `off`, `low`, `medium` and `high`. When a turn thinks past the most tokens of
  thought, furb asks the model again with its thought off, so it writes its word at once.
- A local model writes at most a quarter of its window in one turn, or the number that `FURB_LOCAL_TOKENS` gives.
- furb uses the window that the server gives, or the one that `FURB_LOCAL_WINDOW` gives. It offers no model whose
  window neither gives.

The system prompt of furb is about 9000 tokens, so a model needs a window of 32768 tokens or more. Ollama serves a
model in the `num_ctx` of its parameters, or in `OLLAMA_CONTEXT_LENGTH`: set it to 32768 or more. Ollama tells that
window only for a model that it loaded, so give the same number to `FURB_LOCAL_WINDOW`.

furb asks the server for its list when a host lists the models, as the TUI does, and when no other model can be the
default. It keeps the list while the process runs. It asks again after 30 seconds for a model that the list did not
hold, and for a server that did not answer.

## When a call fails

When a call to a model fails for a cause that passes, such as a lost connection or a provider that is too busy, furb
asks again after a wait that doubles each time, up to ten attempts. The TUI counts the wait down in its footer.

When the claude command line says that you hit its usage limit, furb does not stop the chain. It tells the chain that
it waits and until when, as `#chain1 waits on claude-cli:opus/high: usage limit until 2026-10-23 17:10 (Europe/Paris)`.
It asks again when the limit resets, and the work goes on with no act from you. This wait is no attempt, and it has no
end. A limit that still stands after its reset makes furb ask again every five minutes at most.

The claude command line can be missing for a moment while it updates itself. Then a turn tries it again for 60
seconds, or for the seconds that `FURB_CLAUDE_GONE` gives. A turn that makes no progress for 15 minutes fails, or
after the seconds that `FURB_CLAUDE_STALL` gives.

## The doctrine

Each model reads [`DOCTRINE.md`](../DOCTRINE.md) once on each chain, in the message that asks it for its first word.
It tells the model how to work in furb: that its answer is Python, which verbs it has, and how to give its answer.
