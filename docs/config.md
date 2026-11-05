# Config

furb reads `config.json` in the config directory of the user (`~/.config/furb` by default), then `.furb/config.json`
and `.furb/config.local.json` of the project, which git ignores. A later config wins for each key that it gives.
[`models.md`](models.md) says the keys of the models and the providers, and [`extensions.md`](extensions.md) the key
`extensions`.

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
