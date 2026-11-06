---
name: converge-tui
description: How an audit of the round judges the TUI of main against the rulings of the owner on it, and turns each gap into a row of its register.
---
An audit of the TUI judges what main draws against the rulings of the issues 14 to 17 and 28 of the private repository
of decisions, and against nothing else: no spec, no plan, no taste of your own. The walk of step 5 adds the defects
that a person meets as he uses the TUI, which need no ruling. The local file
`.furb/extensions/conductor/private.json` of the main checkout names that repository under the key `repo`: read each of
these issues whole, with its comments, as `gh issue view <n> --repo <repo> --comments`.

1. In your workspace, run `sh <this folder>/capture.sh <main checkout> $TMPDIR/audit/<key>
   $TMPDIR/audit/tui/records.json`. It checks out origin/main and writes its commit in `sha`, builds through
   `script/build-slot.sh`, and captures the gallery of the demo and the records of the project that the last whole
   capture pins, each a PNG with the cells of its frame, then builds the program furb and walks the TUI, as step 5
   says. It writes each step that fails in `fails.txt`.
2. Keep the rulings that you read beside your capture, as `rulings.md`. When they differ from `$TMPDIR/audit/tui/rulings.md`, or no
   last capture stands, judge each capture: give `diff.py` no last capture.
3. Run `python3 -I <this folder>/diff.py $TMPDIR/audit/<key> "" <main checkout> $TMPDIR/audit/<key>/head
   $TMPDIR/audit/tui/head 1 before now`. It writes `shots.json`: each capture that changed, as the text
   of its frame with each changed row marked, with an image when it is new, when its colors alone changed and in each
   pass, and each step of the capture that failed.
4. A failed step is a fault of the tool and no gap. Capture once more; when the step fails again, say so in your report
   and judge nothing.
5. The capture walks the TUI as a person does, with keys, clicks and sizes, on a life of a scripted model and on the
   records that it captured, which open read-only: `tui/scenarios/walk.ts` says each journey. Each step is a capture of
   `walk/`, with the action that led to it in its `.txt`, and its probes write what they found in
   `head/walk/walk.json`, under `findings`: a key that changed nothing, a view that Esc closed with a trace, a toggle
   that a second press did not undo, an input that no longer shows the keys, a value of the code on the screen, an
   error that the window logged, a life that says it is not open, and an answer that never came. A finding needs no
   ruling, since no ruling allows a broken key: read its capture and the capture before it, and make each one that holds
   a row of your register, with its step, its probe and its capture as evidence. Drop a finding that the capture shows
   to be a fault of the walk itself, and say why in your report. Read the frames of the walk in their order as well, as
   the owner reads his screen, and add a row for each defect that you see there that no probe names.
6. Judge with blind judges. For each batch of at most 16 changed captures, make a chain of its own, give it a grant of
   3 dollars, and prompt claude-cli:sonnet/high there with the rulings whole, the legend below, and the captures of the
   batch, each capture of the walk with its action. Each image of a capture stands in its tag as
   `![<label> <name>](furb-image://<digest>)`, which the judge sees. Ask for a list of the gaps that it sees, each a tuple (capture, gap, quote): the name of the capture, what it
   shows against the rulings in one or two sentences, and the words of the ruling that it fails, copied exactly, in
   twelve characters at least. A judge that gives no list gives no gap, and its batch counts as not judged.
7. Run `python3 -I <this folder>/gaps.py rulings.md <each capture of the batch>`, with the answers of the judges of the
   batch as a JSON list on stdin. It keeps each gap that names a capture of the batch and quotes the rulings exactly,
   once. Each gap that it keeps is a row of your register, the largest first, and its brief names its issue and holds
   its ruling.
8. Once the capture is whole and each batch was judged, put it in place of the last one, so that the next audit diffs
   against it: `rm -rf "$TMPDIR/audit/tui" && mv "$TMPDIR/audit/<key>" "$TMPDIR/audit/tui"`.

The legend that each judge reads: Each capture stands in a capture tag, under its name, and its change says why it is here. Its text is the frame that the renderer drew, one numbered line for each row of the screen, less the runs of rows that did not change, which it names. A row marked + changed: under it, the line before gives what before showed in the columns that changed, and colors gives the colors of those columns now. A row marked ~ changed in its colors alone, which it gives for both. A row marked - is a row of before that is gone, and a row marked + with no line before under it is new, with the colors of its whole row. A color is a key and its columns, and styles names each key: its foreground, its background and its attributes. A capture whose change is the same as the change of another capture names that capture. An image shows a capture as a terminal draws it, when the capture comes with one. A capture of the walk, under walk/, is a step of a person who uses the TUI: its action says what the person did just before, on the screen of the capture before it in its order. Look at each line, each color, each cut, each overlap and each empty place.
