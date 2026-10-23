# Models

furb asks a model for each step of a life. This page says which models it finds, how to choose one, and how to run
one on your own machine.

## Where the models come from

furb looks in three places, in this order:

1. **The claude command line.** When furb finds the program `claude`, it offers its models: `claude-cli:opus`,
   `claude-cli:sonnet`, `claude-cli:haiku` and `claude-cli:fable`. They use the plan that you signed in to `claude`
   with. furb finds the program that `FURB_CLAUDE_BIN` names, else the `claude` on PATH or in your home, else, on
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
