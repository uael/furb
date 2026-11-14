# Config

furb reads `config.json` in the config directory of the user (`~/.config/furb` by default), then `.furb/config.json`
and `.furb/config.local.json` of the project, which git ignores. A later config wins for each key that it gives.
[`models.md`](models.md) says the keys of the models and the providers, and [`skills.md`](skills.md) the key
`skills`. The verb `config` of the daemon, as `docs/protocol.md` says, reads each key with its value and the layer
that gives it, and changes it.

## The schema

A schema names the keys, as the settings of VS Code do. Each key has a property: its `type` (`boolean`, `integer`,
`number`, `string`, `array` or `object`), its `default`, which holds when no config gives the key, its
`description`, and under `enum` the values that it takes when it takes only some. furb names the keys that it reads:
`max_thinking_tokens`, `automatic`, `classes`, `providers` and `skills`. A change of a key through
`/config` or the protocol refuses a value that its property does not take, and writes nothing. A key that no schema
names stays as the config gives it.

An extension adds keys of its own: its manifest names a JSON file under `furb-config`, whose `properties` give each
key by its name, as the `configuration` of an extension of VS Code does:

```json
{"properties": {"folder": {"type": "string", "default": "notes", "description": "The folder of the notes."}}}
```

The configs hold these keys under the name of the extension, as `{"notes": {"folder": "journal"}}`. An extension
that takes a name of a key of furb does not load, and the root is told why, so it names no key and no read of
the configs fails. Its word reads its keys as `read("settings://notes")`, a JSON object of each key
with the value that a config gives, else its default, as `{"folder": "journal"}`; it reads them where it runs, so a
change through `/config` holds from the next read. `read("settings://")` gives each key of the configs, and the name
of a key of furb its value, as `read("settings://automatic")`. The process of an extension reads its keys through the verb
`config` of the [protocol](protocol.md). A file of keys that is missing or is no schema names no key, so
`/config` still reads and changes the configs and can turn that extension off, and `read("settings://notes")`
says why.
