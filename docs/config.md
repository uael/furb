# Config

furb reads `config.json` in the config directory of the user (`~/.config/furb` by default), then `.furb/config.json`
and `.furb/config.local.json` of the project, which git ignores. A later config wins for each key that it gives.
[`models.md`](models.md) says the keys of the models and the providers, and [`extensions.md`](extensions.md) the key
`extensions`. In the TUI, `/config` shows each key with its value and the layer that gives it, and changes it.
`/config <key> <value>` sets a key in the layer that gives it now, else in the config of the project, and
`/config <key>` with no value takes it out of that layer. A key in an object of furb or of a skill takes a dot, as
`/config providers.strata false`, and a true or a false on a provider whose settings are an object turns it on or
off and keeps its settings.

## The schema

A schema names the keys, as the settings of VS Code do. Each key has a property: its `type` (`boolean`, `integer`,
`number`, `string`, `array` or `object`), its `default`, which holds when no config gives the key, its
`description`, and under `enum` the values that it takes when it takes only some. furb names the keys that it reads:
`gate`, `max_thinking_tokens`, `automatic`, `classes`, `providers` and `extensions`. A change of a key through
`/config` or the protocol refuses a value that its property does not take, and writes nothing. A key that no schema
names stays as the config gives it.

An extension adds keys of its own: its manifest names a JSON file under `furb-config`, whose `properties` give each
key by its name, as the `configuration` of an extension of VS Code does:

```json
{"properties": {"folder": {"type": "string", "default": "notes", "description": "The folder of the notes."}}}
```

The configs hold these keys under the name of the extension, as `{"notes": {"folder": "journal"}}`. An extension
cannot take a name of a key of furb. Its word reads its keys as `read("settings://notes")`, a JSON object of each key
with the value that a config gives, else its default, as `{"folder": "journal"}`; it reads them where it runs, so a
change through `/config` holds from the next read. `read("settings://")` gives each key of the configs, and the name
of a key of furb its value, as `read("settings://gate")`. The process of an extension reads its keys through the verb
`config` of the [protocol](protocol.md).

## The gate

The gate reads each word of a model before it runs, against the program of its chain, and refuses a word in which it
finds an error: nothing of that word runs, and the chain is told each finding. `"gate": false` turns it off:

```json
{"gate": false}
```

With the gate off, its ear answers each word with no finding, so each word runs one statement at a time and stops at
the first statement that raises, and the provider no longer reads a word against the gate while a model writes it. A
value that is no bool is as unsaid, and the gate is on when no config names it. A life reads the key when it opens.

The record keeps each gate and its answer, so a replay gives the answer that the record holds, whatever the config
says then, and no record drifts when the key changes.

To compare lives with the gate and without it, count in the facts of each record, as `furb facts <record>` prints
them:

- the words that the gate refused: each `done` of a `gate` whose answer is not `[]`, which the chain is told as a
  `tell` that starts with `#rungN refused`;
- the words that raised or stopped at a statement: each `tell` that starts with `#rungN raised`, and each `ran` of a
  part of a word that raised.
