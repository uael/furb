# Handoff: the loop localfit

## The goal and its rulings

Make a local model good at furb. Only this lane runs real lives, and only on Strata:
`local:qwen3.8-flash-next-iq2_xs`, `FURB_LOCAL_URL=http://192.168.1.13:8080`, and `FURB_LOCAL_KEY` read by the
command from `env.ANTHROPIC_AUTH_TOKEN` of `~/.claude/strata-local.json`, never printed or written. Strata takes one
run at a time. No other model: Sonnet costs the owner money.

The loop: change one thing (the provider, the gate, a clause of the doctrine, an extension), run the bench, keep the
change only when the measure gets better, and ship each kept change as its own PR, with the numbers before and after
in its body. A change of `engine.py` must stay under its wall of 6000 tokens, and the lane replay changes the engine
now, so put a change of the engine in the report and do not make it. When the measure does not move in two turns,
ask chain4.

## Where the work lives

- Worktree: `/Users/uael/src/github.com/uael/furb-w-localfit`.
- Branch `w/localfit`: PR #234, the refusal of a cut turn says what the turn wrote. Green, shipped.
- Branch `w/localfit-efforts`: this file, the bench in `.localfit/bench.py`, and the efforts of a local model in
  `src/world/provider/local.rs`. Not green: the test in `src/world/provider/local.test.rs` that says that a model of a
  local server takes no effort fails, and the change did not make the measure better, so it is no PR.
- Bench outputs: `/Users/uael/src/github.com/uael/furb-localfit-bench/out/<run>/<task>/` (record, `furb.out`,
  `steps.json`).

## The bench

`python3 -I .localfit/bench.py <task> <furb> <out> [seconds]`, with `BENCH_MODEL` to name the actor, as
`local:qwen3.8-flash-next-iq2_xs/low`. Tasks: `count` (lines of a file, shape int), `calc` (fix add, mean and clamp
until `python3 test_calc.py` prints ok), `edit` (one replace in settings.ini), `ask` (a prompt of str to the operator
inside the word; the operator answers teal on stdin). It runs `furb prompt` on a fresh copy, reads the record with
`furb facts` every 15 seconds, stops the run on a big friction, and prints `RESULT` with the result, the replies, the
refused words by cause, the silent words, the cut turns, the tokens and the seconds.

Known faults of the bench:
- The findings of a word are read from the gate whose maker is the rung, but in the off run a word that is no python
  showed no findings, so the gate is likely made by the chain. Map each gate to its rung by order or by its word.
- The rule "the same word 3 times" misses a loop of words that differ only by a number, as `#rung1074\n<s:rung1074_word>`.
  Compare the words with their digits removed.

## Measures

| run | actor | count | calc | edit | ask |
| --- | --- | --- | --- | --- | --- |
| base-2 | no effort (template default xhigh) | ok, 15 s, 1 reply | failed: 2 cut turns of 65536 tokens | ok, 45 s, 2 replies | ok, 15 s, 1 reply |
| low-1 | /low | | failed: 2 cut turns of 65536 tokens, the chain paused | | |
| off-1 | /off | | failed: 1130 replies of 17 tokens each, a word that echoes `#rungN\n<s:rungN_word>` | | |

What the cut turns of calc show (PR #234 made them visible): about 250000 characters of thought and no text. The
model never reads calc.py. It guesses the bugs (mul, div, pow), and it doubts that a word may look and be asked again
("maybe the evaluation is interactive"). So the cause is the model, not the cap: it does not believe the doctrine
clause "A word that closes nothing ends its step".

Facts of Strata found on the way: it is llama.cpp, with a window of 262144 and one slot. Its chat template takes
`reasoning_effort` low, medium or xhigh (high is xhigh, the default is xhigh), and `chat_template_kwargs`
`{"enable_thinking": false}` turns the thought off. It reads no budget of thought per request
(`thinking_budget_tokens` and `reasoning_budget` do nothing). Output runs at about 100 tokens a second, so a cut
turn of 65536 tokens costs about 10 minutes.

## Next step

1. Fix the two faults of the bench above.
2. Try a thought budget in the provider: stream the thought, and when it passes a budget (say 8192 tokens), end the
   request and ask again with the thought closed (`</think>` then the start of the word), or refuse the turn with the
   end of its thought, so the model reads that it must act. Measure calc with it.
3. Try a clause of the doctrine, or a first word of the chain, that shows a look then close in two steps, since the
   model doubts that it may look first. A change of engine.py goes in the report only (the lane replay owns it).
4. Run the four tasks at each change; keep only what makes calc pass without breaking the other three.
