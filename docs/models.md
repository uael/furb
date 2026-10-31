# Models

furb asks a model for each step of a life. This page says which models it finds, how to choose one, and how to run
one on your own machine.

## Where the models come from

furb looks in two places, in this order:

1. **A provider of the network.** furb offers the models of each provider whose key stands in the environment, as
   `ANTHROPIC_API_KEY` or `OPENAI_API_KEY`. It knows the providers that [models.dev](https://models.dev) and pi-ai
   list.
2. **A local server.** When `FURB_LOCAL_URL` gives the address of a server, furb offers the models that it lists.
   [A local model](#a-local-model) says more.

The first model that furb offers is the default: a model of the network, else a local model. The TUI checks each place
on its first run, and says how to fix each one that is missing.

## Choose a model

A model has a name of the form `provider:id`, as `openai:<id>` or `local:qwen3`. When one model alone has an id, the id
alone names it, as `qwen3:8b` names a local model under A local model.

An **actor** is a model and an effort, which is how hard the model thinks: `off`, `minimal`, `low`, `medium`,
`high`, `xhigh` or `max`. Write the effort after a slash, as `local:qwen3/medium`. An actor with no effort takes
`high`, and an effort that a model does not take moves to the nearest one that it takes.

- In the TUI, `/model` and `/effort` choose the actor of the chain on screen. ⌃M, and ⇧Tab or ⌃T, do the same.
- On the command line, `--model` gives the default actor, as `furb --model local:qwen3/low`.
- `--roster` gives more models that a life can ask, one name each time. With no `--roster`, a life can ask each
  model that furb offers. A prompt to a model that the roster does not hold is refused.

## Provider settings

For a ChatGPT subscription, open `/providers` and choose `codex`. furb uses an existing ChatGPT sign-in when
one is available. Otherwise, it opens the browser for sign-in. When it connects, the Codex model picker opens.
Choose a model there. No API key, server, model id or context size is needed.

`codex` uses the limits of the ChatGPT subscription. `openai` remains the provider for an OpenAI API key.
Connecting Codex does not change a running model call.

The account service supplies the model list, the input window, image support and the supported reasoning efforts.
furb reserves the input headroom that the service specifies, or 5% when it leaves that value unsaid. For a model
with a 500,000-token default window and 5% headroom, the picker shows 475,000 input tokens.

To request a larger window, set `model_context_window` under the `codex` provider in `.furb/config.json`:

```json
{"providers": {"codex": {"model_context_window": 1000000}}}
```

As in Codex, the value is capped at the maximum window from the account service before the input reserve is
applied. With an 872,000-token maximum and a 95% usable share, this setting gives 828,400 input tokens. Without
this setting, furb uses the default window from the account service.

Model data is cached for five minutes and checked when a life opens, when the Codex model picker opens, and on
`/reload`. If a refresh fails for a temporary cause, the last model list stays available. A changed model list or window
updates the standing when the operator reloads it or opens the Codex picker.

The catalog of the other providers comes from two sources: models.dev, and the package pi-ai for the providers that
models.dev does not list. A life updates both in the background once a day, and `furb catalog --update` updates them
at once. An update replaces the copy in the cache only when the source answers with data that reads as a catalog, so an
offline or invalid source leaves the last valid catalog in use. `furb catalog` shows for each source its version, the
time of its last good fetch, and the error of its last failed one. A running life uses a new catalog at its next
stand, on `/reload`, with no restart. A scheduled job of the repository makes the bundled catalog again each day, with
the exact version of pi-ai, and opens a pull request only when the data changes.

furb keeps its sign-in in the user config directory, under `auth`, outside the project. On Unix, the credential
file is readable and writable only by its owner. furb refreshes its own credential when needed. It can read an
existing Codex access token, but it does not use the Codex refresh token or change the Codex credentials.

Disabling `codex` removes its models from this project and keeps the sign-in for a later connection.

Run `/providers` in the TUI to add or remove a provider for the current project. A built-in provider uses the
credential in the environment. To add a server that uses OpenAI chat completions, enter:

```text
/provider add office http://127.0.0.1:8080/v1 qwen3 262144 OFFICE_KEY
```

The arguments are the provider name, URL, model id, context window and, when needed, the name of the environment
variable that holds its key. The optional last argument selects the protocol, which is `chat` when unsaid. Use `openai`
for OpenAI Responses, `anthropic` for Anthropic Messages, or `gemini` for Google Gemini. Use `-` for the key name when
the server needs no key and a protocol follows it. `/provider remove office` disables the provider and keeps its
settings for a later add.

The same settings can be written in `config.json` in the user config directory, in `.furb/config.json` in the
project, or in `.furb/config.local.json` beside it, which git ignores: a project keeps its local models there, as
those of a server on this machine. Each config is read in that order, and a later one wins for each field that it
gives. Other config fields stay as they are. `/provider` and the dialog of the providers write `.furb/config.json`.

```json
{
  "providers": {
    "openai": false,
    "office": {
      "protocol": "chat",
      "url": "http://127.0.0.1:8080/v1",
      "key_env": "OFFICE_KEY",
      "models": {
        "qwen3": {
          "window": 262144,
          "output": 32768,
          "reasoning": true,
          "images": false,
          "efforts": ["off", "low", "medium", "high"]
        }
      }
    }
  }
}
```

A built-in provider can be `true`, `false`, or an object that overrides its URL, credential or models. A custom provider
needs a URL and at least one model. Each model needs a positive window. Its output limit is one quarter of that window
when unsaid. `reasoning` and `images` are false when unsaid. `efforts` lists the efforts that a model of the protocol
`chat` takes, from `off`, `low`, `medium` and `high`, as a local model takes them: `off` turns its thought off with
`enable_thinking` false, and each other level is `reasoning_effort`. A model that takes `off` thinks at most the tokens
of thought of one turn, as a local model does. With no `efforts`, it takes none. The optional `key` field gives a key
directly; `key_env` wins when both are present. A custom server with neither field sends no key. An object can set
`enabled` to false to disable a provider and keep its other fields. The name `local` can be false to remove the server
that `FURB_LOCAL_URL` names from the roster.

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

## The claude command line

The provider `claude-cli` runs the `claude` command line as one account by default. To run it as several accounts, list
them under `providers.claude-cli` in `.furb/config.json`:

```json
{
  "providers": {
    "claude-cli": {
      "accounts": [
        {"name": "perso", "token_file": "~/.config/furb/perso.token"},
        {"name": "team", "token_env": "FURB_CLAUDE_TEAM"},
        {"name": "pro", "config_dir": "~/.claude-pro"}
      ],
      "switch": "failover"
    }
  }
}
```

An account has a `name` and exactly one place for its credential:

- `token_file` is a file that holds a token of `claude setup-token`. furb reads the file each time it starts a process.
- `token_env` is the name of a variable of the environment of furb that holds such a token.
- `config_dir` is a config directory of the command line, where the account logged in.

A path is absolute or starts with `~`. furb never writes a token in a config, a message or a log. A token of
`claude setup-token` lives one year and never refreshes, so it cannot lose its sign in as a refreshed login can. The
user runs `claude setup-token` once for each account, since it opens a browser and shows the token once.

`switch` is `failover` or `spread`. With `failover`, which is the default, a new conversation takes the first account of
the list that is not parked. With `spread`, it takes the account that a conversation took least recently.

A turn that meets a usage limit parks its account. The park lasts until the reset that the message gives, or for
5 minutes when the message gives none. A turn whose sign in fails, or whose token cannot be read, parks its account until `/reload`. When
another account is free, the turn ends as a failure that passes, with the words `Switched account from a to b`. Then
furb asks again, as the section on a failed call says. The retry runs as the other account, in a new conversation
with the whole history. When every account is parked, the turn waits for the first reset. With one account or none,
furb parks nothing. The list of models and the web search run as the first account that is not parked, and they do not
switch.

`command` gives the program of the command line, as a word or as a list of words, such as `["cswap", "run", "3", "--"]`.
Its words come before each flag, for each account.

## Thinking

`MAX_THINKING_TOKENS` gives the most tokens that a model can think in one turn, and 0 turns thought off. The key
`max_thinking_tokens` of `.furb/config.json` gives it for one project, and the same key of `.furb/config.local.json`
wins over it. The environment wins over the config, and with
neither, it is 32768. A model with adaptive reasoning thinks as its effort says and cannot turn its thought off.

## A local model

A local server sends your work to no provider of the network, so it stays on your machine. furb talks to each server
that serves the chat completions of OpenAI under `/v1` and lists its models at `/v1/models`, such as llama.cpp, LM
Studio, Ollama, vLLM and Strata:

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
A usage limit of the claude command line is no attempt. furb asks again at the reset that the limit names. A failed sign in of that command line is no attempt too. furb asks again every minute, and a stand asks at once. So the user can sign in again outside furb, and the chain goes on with no pause.
A pause of the chain holds the next call of a reply that waits, and the wake makes it. A call that runs returns.

## The doctrine

Each chain reads [`DOCTRINE.md`](../DOCTRINE.md) in the first turn of its model, as a read of a file:
`.furb/DOCTRINE.md` of the project when it holds one, the `DOCTRINE.md` of the checkout in dev mode, else
`~/.furb/DOCTRINE.md`, which furb writes from the doctrine that it carries. It tells the model how to work in furb: that
its answer is Python, which verbs it has, and how to give its answer.
