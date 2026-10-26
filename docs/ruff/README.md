# Ruff-100 display proof

These are headless captures from `bun run tui/script/ruff-proof.ts`, not a running user session.
The fixture is inert and local. No provider or model runs for these pictures.
`manifest.json` identifies the source checkpoint, source hashes, installed Ruff, and picture hashes.

The 160-column pictures show the running word, raw toggle, formatted view, selection, and finished threads.
The 80-column pictures show the same feed, a quote expanded through its last instruction, and a folded thread.
`copied.json` holds the exact original source selected by Copy its source, as a JSON string.
Text frames omit terminal padding; PNGs retain the exact terminal cells. The renderer test checks the exact copy,
including Unicode, quote bindings, raw strings, and multiline strings. A native scripted test checks the exact
original program in the editor. Neither action takes its source from the formatted display.

The raster font has no CJK or emoji glyphs. The pictures replace those fixture glyphs with “Unicode value”.
The renderer tests use the original Unicode fixture. These pictures are not Unicode font evidence.

`ruff.gif` is a reduced-motion state walkthrough: running, raw, formatted, finished.
Its four frames each hold for 1.4 seconds. It does not demonstrate an animated loader.
`animation-samples.json` records timing and hashes of PNGs decoded from that actual GIF.
The standard `docs/furb.gif` is a separate 129-picture demo capture.

Ruff targets 100 columns, but it cannot split every literal. The terminal can also be narrower than the target.
Folded strings expand in place, and original source copy and edit remain available.
The number beside a projected line is a display number. Engine refusal coordinates stay original.
Numbers continue through each thread and start at one in an independent thread.

Verified commands for this change:

- `bun run test ruff`: nine tests passed, with installed Ruff and controlled lifecycle fixtures.
- `bun run test`: all 367 tests passed after the queue-handoff fix, through the normal project wrapper.
- `bun run check`: root and GUI type checks passed.
- `bun run lint`: passed; informational style suggestions remain.
- `bun run screenshots`: 91 pictures captured to scratch with the permitted temporary picker adapter,
  restored exactly afterward. The broad gallery is not included here.
- `bun run animation`: captured the standard 129-picture animation.

The focused tests also check incomplete and invalid input, missing commands, command changes, disposal,
queue admission across cards, a completion callback's microtask handoff, a complex rewrite's raw fallback, cache eviction beyond 256 words with refreshed
step objects, preference persistence, live tails, continued numbers, and original ownership and refusal text.

The public completion-callback regression failed before the queue-handoff fix and passed afterward.
`queue-handoff-before.txt` and `queue-handoff-after.txt` record that focused run.
It checks the Formatter callback boundary; it does not claim that the App's delayed redraw hit that race.
