//! The scenarios of the captures: the 27 daily paths on which the window and the other programs in
//! a terminal are judged, and the walk of the README. Each step of a scenario is one frame. A
//! scenario that the window has is a walk: the words that its scripted model answers, turn by turn,
//! and its steps. Another stands at a size on the steps of other scenarios. A scenario that the
//! window has not names its gap.

use std::time::Duration;

use crate::capture::READY;

/// A size of the terminal of a capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Size {
  pub(crate) cols: usize,
  pub(crate) rows: usize,
}

impl Size {
  pub(crate) fn name(self) -> String {
    format!("{}x{}", self.cols, self.rows)
  }
}

/// The walk of the still image of the cards of a machine: a thread that works, one that answered
/// and that he did not read, one that asks him, and sessions that rest.
pub(crate) const CARDS: Walk = Walk {
  sessions: &["Measure the open of a large record.", "Write the docs of the protocol."],
  words: &[
    "await bash('sleep 60')\nclose('slept')",
    "close('The boot reads the record once, then each tail.')",
    ASKS,
  ],
  pace: &[],
  reads: false,
  far: None,
  steps: &[
    step("a", &[Act::Type("Run the bench of the replay."), Act::Key("Enter")], "⎋ stop"),
    step(
      "b",
      &[Act::Key("C-n"), Act::Type("Read the boot of the host."), Act::Key("Enter")],
      "each tail",
    ),
    step(
      "c",
      &[Act::Key("C-n"), Act::Type("Place the list of cards."), Act::Key("Enter")],
      "Where does the list",
    ),
  ],
};

/// The sizes of the captures of the other programs: the common one first, then narrow and wide.
pub(crate) const SIZES: [Size; 3] =
  [Size { cols: 120, rows: 40 }, Size { cols: 80, rows: 24 }, Size { cols: 200, rows: 60 }];

/// What a step does in the terminal before it waits for its screen.
#[derive(Debug)]
pub(crate) enum Act {
  /// A text typed as it stands.
  Type(&'static str),
  /// A key, as tmux names it: `Enter`, `C-x`, `PageUp`, `Escape`.
  Key(&'static str),
  /// A text pasted, as a terminal pastes it.
  Paste(&'static str),
  /// The pointer on the first cell of a text: button 0 clicks it, 35 moves there, 64 and 65 turn
  /// the wheel up and down.
  Pointer { on: &'static str, button: u8 },
  /// A wait until the screen shows a text, as a person reads an answer before the next message, so
  /// the life reads each message at the same turn in each capture.
  Until(&'static str),
  /// A message that another window of the operator says in a new session of the project, which
  /// that window holds until the walk ends, so the window of the walk sees a life that it did not
  /// open.
  Said(&'static str),
}

impl Act {
  /// What the window reads of the act, as a recording keeps a key: the text typed or pasted, a
  /// key by its name in brackets, the pointer by the text it stands on, and nothing for a wait.
  pub(crate) fn read(&self) -> Option<String> {
    match self {
      Self::Type(text) | Self::Paste(text) => Some((*text).to_owned()),
      Self::Key(key) => Some(format!("<{key}>")),
      Self::Pointer { on, button } => Some(format!("<pointer {button} on {on}>")),
      Self::Until(_) | Self::Said(_) => None,
    }
  }
}

/// A step: its name, which names its frame, what it does, and a text that its screen shows once it
/// is done.
#[derive(Debug)]
pub(crate) struct Step {
  pub(crate) name: &'static str,
  pub(crate) acts: &'static [Act],
  pub(crate) until: &'static str,
}

/// A walk of the window: the first message of each session that stands before the window, the
/// word of the model at each turn of the window, in order, the pace of the model, and the steps.
#[derive(Debug)]
pub(crate) struct Walk {
  pub(crate) sessions: &'static [&'static str],
  pub(crate) words: &'static [&'static str],
  /// The time that the model takes to answer a turn that holds a text, as a real model takes
  /// them; each other turn it answers at once. Turns that start together and wait for different
  /// times take their words in the order of their times.
  pub(crate) pace: &'static [(&'static str, Duration)],
  /// Whether the window only reads, as `furb --read` opens it.
  pub(crate) reads: bool,
  /// A second machine on the tailnet, by its name, which the window finds through the stand-in of
  /// Tailscale: a daemon of its own, with the project `lab` and one session there.
  pub(crate) far: Option<&'static str>,
  pub(crate) steps: &'static [Step],
}

/// What the window has of a scenario.
#[derive(Debug)]
pub(crate) enum Has {
  Walk(Walk),
  /// Steps of other scenarios at one size: each the name of a scenario and of its step.
  Sized(Size, &'static [(&'static str, &'static str)]),
}

/// A scenario: its number and its name, as `05-prompt-box`, and what the window has of it.
#[derive(Debug)]
pub(crate) struct Scenario {
  pub(crate) name: &'static str,
  pub(crate) has: Has,
}

impl Scenario {
  pub(crate) fn walk(&self) -> Option<&Walk> {
    match &self.has {
      Has::Walk(walk) => Some(walk),
      Has::Sized(..) => None,
    }
  }
}

/// The scenario that a word names: its number, as `05`, or its whole name.
pub(crate) fn named(word: &str) -> Option<&'static Scenario> {
  SCENARIOS
    .iter()
    .find(|scenario| scenario.name == word || scenario.name.split('-').next() == Some(word))
}

/// A turn of the wheel down over the filter of an open list part.
const WHEEL_DOWN: Act = Act::Pointer { on: "Search a", button: 65 };

const fn step(name: &'static str, acts: &'static [Act], until: &'static str) -> Step {
  Step { name, acts, until }
}

/// The prose of a model with each mark of markdown: a heading, a list, a fence with its language,
/// a table, a link, a bold word and inline code.
const PROSE: &str = r"close('''## Where the open spends its time

The open takes **329 s**. Two stages hold most of it:

- `seed` reads each chain once
- the image skips the boot

```python
def total(stages):
    return sum(stage.took for stage in stages)
```

| Stage | Time | Share |
| --- | --- | --- |
| seed | 181 s | 55% |
| boot | 104 s | 32% |

See [the trace](https://github.com/uael/furb/issues/988).''')";

/// A word that asks the operator a choice, and closes its thread with the answer.
/// A long draft, of twelve lines, that a paste puts in the input.
const DRAFT: &str = "Read the boot, then the seed.\nThen the turns of each chain.\nThen the stages.\nSum them.\nSay where the time goes.\nKeep the numbers.\nName each stage.\nGive the share of each.\nSay what to cut first, and why it costs the most of the open of a record that holds a hundred thousand facts.\nThen the test.\nThen the proof.\nThen stop.";

const ASKS: &str = "side = await prompt(Literal['right', 'left'], 'Where does the list of cards stand?', to=OPERATOR)\nclose(side)";

/// The words of a model that changes two places of a file of the project and adds a file.
const CHANGES: &str = "m = read('src/inventory/models.py')\nwrite(m.replace('self.name = name', 'self.name = name.strip()\\n    self.count = 0'))\nwrite(Text('src/inventory/report.py', 'def total(items):\\n  return sum(one.count for one in items)\\n'))\nclose('Each item has a count, and report.py sums them.')";

/// The words of a long thread: an answer to each message, then the answer to the message that
/// goes from a moment of its past as a rewind.
const LONG: [&str; 16] = [
  "close('Answer 1 of the long thread.')",
  "close('Answer 2 of the long thread.')",
  "close('Answer 3 of the long thread.')",
  "close('Answer 4 of the long thread.')",
  "close('Answer 5 of the long thread.')",
  "close('Answer 6 of the long thread.')",
  "close('Answer 7 of the long thread.')",
  "close('Answer 8 of the long thread.')",
  "close('Answer 9 of the long thread.')",
  "close('Answer 10 of the long thread.')",
  "close('Answer 11 of the long thread.')",
  "close('Answer 12 of the long thread.')",
  "close('Answer 13 of the long thread.')",
  "close('Answer 14 of the long thread.')",
  "close('Answer 15 of the long thread.')",
  "close('Answer of the rewind.')",
];

/// The word that the README shows: it reviews each file that changed on an agent of its own, all
/// at once, then fixes the tests until they pass, and closes with what each agent found.
const REVIEW: &str = r"changed = (await bash('git diff --name-only main')).stdout.content.split()
reviews = [prompt(str, f'Find the bugs in {path}.', on=chain(path)) for path in changed]
found = []
for review in reviews:
  found.append(await review)
fixer = chain('fix')
tests = await bash('python3 tests/test_report.py', ok=True)
while tests.code:
  await prompt(None, f'Fix this failure:\n{tests.stdout.content}', on=fixer)
  tests = await bash('python3 tests/test_report.py', ok=True)
close('\n\n'.join(found))";

/// The word of the change before the review: a count on each item, a report of the total with a
/// bug, and the test that finds it, staged so the diff with `main` names the three files.
const CHANGE: &str = r"m = read('src/inventory/models.py')
write(m.replace('self.name = name', 'self.name = name\n    self.count = 0'))
write(Text('src/inventory/report.py', 'def total(items):\n  return sum(one.name for one in items)\n'))
write(Text('tests/test_report.py', '''import sys

sys.path.insert(0, 'src')

from inventory.models import Item
from inventory.report import total

a, b = Item('a'), Item('b')
a.count, b.count = 2, 3
assert total([a, b]) == 5
print('The total is right.')
'''))
await bash('git add src tests')
close('Each item has a count, and report.py sums them.')";

/// The README: why furb stands out, in one walk. A session holds a change, and one reply of Python
/// reviews each file of it on an agent of its own under the line that made them, loops until the
/// tests pass, and closes with what each agent found. Each model takes the time of a real one.
/// `furb-frames hero` makes the images of the README from it.
pub(crate) const README: Walk = Walk {
  sessions: &[],
  words: &[
    CHANGE,
    REVIEW,
    "source = read('src/inventory/models.py')\nclose('models.py: each item starts with a count of 0.')",
    "source = read('src/inventory/report.py')\nclose('report.py: `total` sums the names of the items, not their counts.')",
    "source = read('tests/test_report.py')\nclose('test_report.py: the test is right, and fails on the bug of report.py.')",
    "report = read('src/inventory/report.py')\nwrite(report.replace('one.name', 'one.count'))\nclose('`total` sums the counts now.')",
  ],
  pace: &[
    ("Find the bugs in src/inventory/models.py.", Duration::from_millis(4000)),
    ("Find the bugs in src/inventory/report.py.", Duration::from_millis(5500)),
    ("Find the bugs in tests/test_report.py.", Duration::from_millis(7000)),
    ("Fix this failure", Duration::from_millis(3000)),
    ("Review the change, and make the tests pass.", Duration::from_millis(2500)),
  ],
  reads: false,
  far: None,
  steps: &[
    step(
      "x-change",
      &[Act::Type("Add a count to each item, and a report of the total."), Act::Key("Enter")],
      "report.py sums them",
    ),
    step("o", &[Act::Key("C-n")], ""),
    step(
      "a",
      &[Act::Type("Review the change, and make the tests pass."), Act::Key("Enter")],
      "Working",
    ),
    step("b", &[], "Find the bugs in tests/test_report.py"),
    step("c", &[], "report.py: total sums"),
  ],
};

/// The scenarios, in their order.
pub(crate) const SCENARIOS: &[Scenario] = &[
  Scenario {
    name: "01-first-run",
    has: Has::Walk(Walk {
      sessions: &[],
      words: &[],
      pace: &[],
      reads: false,
      far: None,
      steps: &[
        step("a", &[], READY),
        step("b", &[Act::Key("C-x"), Act::Key("?")], "each key"),
        // Go to names the project of the folder, where no session stands yet.
        step("c", &[Act::Key("Escape"), Act::Key("C-p")], "no session yet"),
      ],
    }),
  },
  Scenario {
    name: "02-resume",
    has: Has::Walk(Walk {
      sessions: &[
        "Find where the open spends its time.",
        "Name the stages of a resume.",
        "Read the record of the slow open.",
        "Measure the seed of a large record.",
        "Say why the boot replays twice.",
        "List the tests of the checkpoint.",
      ],
      words: &[],
      pace: &[],
      reads: false,
      far: None,
      steps: &[
        step("a", &[Act::Key("C-o")], "6 sessions"),
        step("b", &[Act::Type("seed")], "Measure the seed"),
        step("c", &[Act::Key("Enter")], "reply 1"),
      ],
    }),
  },
  Scenario {
    name: "03-sessions-many",
    has: Has::Walk(Walk {
      sessions: &[],
      words: &["await bash('sleep 30')\nclose('slept')", ASKS],
      pace: &[],
      reads: false,
      far: None,
      steps: &[
        step("x-first", &[Act::Type("Run the bench."), Act::Key("Enter")], "⎋ stop"),
        step(
          "a",
          &[Act::Key("C-n"), Act::Type("Place the list of cards."), Act::Key("Enter")],
          "Where does the list",
        ),
        step("c", &[Act::Key("C-x"), Act::Key("j")], "Run the bench."),
        step("x-leave", &[Act::Key("C-d"), Act::Key("C-d")], "Leave the window"),
      ],
    }),
  },
  Scenario {
    name: "04-scroll-back",
    has: Has::Walk(Walk {
      sessions: &[],
      words: &LONG,
      pace: &[],
      reads: false,
      far: None,
      steps: &[
        step(
          "a",
          &[
            Act::Type("Message 1 of the long thread."),
            Act::Key("Enter"),
            Act::Until("Answer 1 of the long thread."),
            Act::Type("Message 2 of the long thread."),
            Act::Key("Enter"),
            Act::Until("Answer 2 of the long thread."),
            Act::Type("Message 3 of the long thread."),
            Act::Key("Enter"),
            Act::Until("Answer 3 of the long thread."),
            Act::Type("Message 4 of the long thread."),
            Act::Key("Enter"),
            Act::Until("Answer 4 of the long thread."),
            Act::Type("Message 5 of the long thread."),
            Act::Key("Enter"),
            Act::Until("Answer 5 of the long thread."),
            Act::Type("Message 6 of the long thread."),
            Act::Key("Enter"),
            Act::Until("Answer 6 of the long thread."),
            Act::Type("Message 7 of the long thread."),
            Act::Key("Enter"),
            Act::Until("Answer 7 of the long thread."),
            Act::Type("Message 8 of the long thread."),
            Act::Key("Enter"),
            Act::Until("Answer 8 of the long thread."),
            Act::Type("Message 9 of the long thread."),
            Act::Key("Enter"),
            Act::Until("Answer 9 of the long thread."),
            Act::Type("Message 10 of the long thread."),
            Act::Key("Enter"),
            Act::Until("Answer 10 of the long thread."),
            Act::Type("Message 11 of the long thread."),
            Act::Key("Enter"),
            Act::Until("Answer 11 of the long thread."),
            Act::Type("Message 12 of the long thread."),
            Act::Key("Enter"),
            Act::Until("Answer 12 of the long thread."),
            Act::Type("Message 13 of the long thread."),
            Act::Key("Enter"),
            Act::Until("Answer 13 of the long thread."),
            Act::Type("Message 14 of the long thread."),
            Act::Key("Enter"),
            Act::Until("Answer 14 of the long thread."),
            Act::Type("Message 15 of the long thread."),
            Act::Key("Enter"),
          ],
          "Answer 15 of the long thread.",
        ),
        step("b", &[Act::Key("PageUp")], "Message"),
        step(
          "c",
          &[Act::Key("PageUp"), Act::Key("PageUp"), Act::Key("PageUp"), Act::Key("PageUp")],
          "Message 1 of",
        ),
        // The bar of the time, whose track marks each message: the thread steps back moment by
        // moment, a click on the track takes it to a moment, a play steps it on, which the clocks
        // that stand still hold, and an escape goes back to now.
        step("x-time", &[Act::Key("End"), Act::Key("C-x"), Act::Key("b")], " now"),
        // The pointer on a diamond of the track shows the message said at that moment.
        step("x-hover", &[Act::Pointer { on: "◆", button: 35 }], "◆"),
        step(
          "x-back",
          &[Act::Key("Left"), Act::Key("Left"), Act::Key("Left"), Act::Key("Left")],
          " 4 moments back",
        ),
        step(
          "x-click",
          &[Act::Pointer { on: "━", button: 0 }],
          "close('Answer 1 of the long thread.')",
        ),
        step("x-play", &[Act::Key("Space")], "❚❚"),
        step("x-now", &[Act::Key("Escape")], "Answer 15 of the long thread."),
        // A message from a moment goes as a rewind: a chain on the messages until then, whose
        // agent answers under the line of the word.
        step(
          "x-rewind",
          &[
            Act::Key("C-x"),
            Act::Key("b"),
            Act::Until(" now"),
            Act::Key("Left"),
            Act::Type("Go on from here."),
            Act::Key("Enter"),
          ],
          "Answer of the rewind",
        ),
        // The rewind is the second branch of the thread, and its head goes back to the first.
        step("x-branch", &[Act::Pointer { on: "└ 'Answer of the rewind", button: 0 }], "⑂ 2 · "),
        // Back on the first branch, its head names how many branches the thread has.
        step("x-fork", &[Act::Key("Escape")], "⑂ 1 of 2 · "),
        // The chains of the life stand under the bar on its moments: none works once the rewind
        // answered, a text finds the root in grey, and enter opens its thread.
        step("x-chains", &[Act::Key("C-x"), Act::Key("t")], "no chain works now"),
        step("x-find", &[Act::Type("ro")], "find ro"),
        step("x-chain", &[Act::Key("Enter")], "Answer 15 of"),
        // A question aside goes as his word on the thread, which a fork of its chain answers.
        step(
          "x-aside",
          &[Act::Type("/btw Which message came first?"), Act::Key("Enter")],
          "on=chain('btw'",
        ),
      ],
    }),
  },
  Scenario {
    name: "05-prompt-box",
    has: Has::Walk(Walk {
      sessions: &[],
      words: &[],
      pace: &[],
      reads: false,
      far: None,
      steps: &[
        step("a", &[], READY),
        step("b", &[Act::Type("Where does the time of the open go?")], "open go?"),
        // A long paste folds into its mark, and the same paste again shows it whole.
        step("c", &[Act::Key("C-c"), Act::Paste(DRAFT)], "paste-1 · 12 lines"),
        step("d", &[Act::Paste(DRAFT)], "Then stop."),
      ],
    }),
  },
  Scenario {
    name: "06-prompt-history",
    has: Has::Walk(Walk {
      sessions: &[],
      words: &[],
      pace: &[],
      reads: false,
      far: None,
      steps: &[
        step("x-first", &[Act::Type("Read the boot of the host."), Act::Key("Enter")], "reply 1"),
        step(
          "x-second",
          &[Act::Type("Measure the seed of the open."), Act::Key("Enter")],
          "reply 2",
        ),
        step(
          "x-third",
          &[Act::Type("Say why the boot replays twice."), Act::Key("Enter")],
          "reply 3",
        ),
        step("a", &[Act::Key("Up")], "history 1 /"),
        step("b", &[Act::Key("Up"), Act::Key("Up")], "history 3 /"),
        step("c", &[Act::Key("C-c"), Act::Type("boot"), Act::Key("C-r")], "▌boot"),
      ],
    }),
  },
  Scenario {
    name: "07-prompt-complete",
    has: Has::Walk(Walk {
      sessions: &[],
      words: &[],
      pace: &[],
      reads: false,
      far: None,
      steps: &[
        step("x-thread", &[Act::Type("Where does this run?"), Act::Key("Enter")], "reply 1"),
        step("a", &[Act::Type("/")], "Commands and skills"),
        step("b", &[Act::Type("mo")], "model of the input"),
        step(
          "c",
          &[Act::Key("BSpace"), Act::Key("BSpace"), Act::Type("comp"), Act::Key("Tab")],
          "/compaction",
        ),
        step("d", &[Act::Key("C-c"), Act::Type("Where")], "does this run?"),
        // A colon at the start of a word finds an emoji by its short code, and a tab writes it.
        step("x-emoji", &[Act::Key("C-c"), Act::Type("Shipped :tada")], "Emoji"),
        step("x-emoji-take", &[Act::Key("Tab")], "Shipped 🎉"),
      ],
    }),
  },
  Scenario {
    name: "08-file-picker",
    has: Has::Walk(Walk {
      sessions: &[],
      words: &[],
      pace: &[],
      reads: false,
      far: None,
      steps: &[
        step("x-thread", &[Act::Type("Where does this run?"), Act::Key("Enter")], "reply 1"),
        step("a", &[Act::Type("look at @")], "Files of demo"),
        step("b", &[Act::Type("json_b")], "json_backend.py"),
        step("c", &[Act::Key("Tab")], "backends/json_backend.py"),
        step("d", &[Act::Type("what does it store?"), Act::Key("Enter")], "reply 2"),
        // The file that he named comes first when a mention names no letter yet.
        step("x-named", &[Act::Type("and @")], "Files of demo"),
        // A draft that he empties goes into the stash, and the key of the stash takes it back.
        step("x-empty", &[Act::Key("Escape"), Act::Key("C-c")], "A reply opens this thread"),
        step("x-unstash", &[Act::Key("C-x"), Act::Key("u")], "and @"),
      ],
    }),
  },
  Scenario {
    name: "09-image",
    has: Has::Walk(Walk {
      sessions: &[],
      words: &[],
      pace: &[],
      reads: false,
      far: None,
      steps: &[
        // A paste of the path of an image file puts the image in as its mark.
        step("a", &[Act::Paste("shot.png")], "▣ image-1.png"),
        // The image goes to the new session before the message that names it, and the model
        // sees it.
        step("b", &[Act::Type("what is in it?"), Act::Key("Enter")], "an image, reply 1"),
        // A word that reads the image shows it under its line.
        step(
          "c",
          &[Act::Key("C-x"), Act::Key("y"), Act::Type("shot = read('shot.png')")],
          "shot = read('shot.png')",
        ),
        step("d", &[Act::Key("Enter")], " 2  shot = read('shot.png')"),
      ],
    }),
  },
  Scenario {
    name: "10-shell-mode",
    has: Has::Walk(Walk {
      sessions: &[],
      words: &[],
      pace: &[],
      reads: false,
      far: None,
      steps: &[
        step("x-thread", &[Act::Type("Where does this run?"), Act::Key("Enter")], "reply 1"),
        step(
          "a",
          &[Act::Key("C-x"), Act::Key("y"), Act::Type("answer = 41 + 1")],
          "answer = 41 + 1",
        ),
        step("b", &[Act::Key("Enter")], " 2  answer = 41 + 1"),
        step("c", &[Act::Type("raise ValueError('no')"), Act::Key("Enter")], "✗  raise ValueError"),
        step("d", &[Act::Key("Escape")], "⏎ run · "),
      ],
    }),
  },
  Scenario {
    name: "11-prose-stream",
    has: Has::Walk(Walk {
      sessions: &[],
      words: &[PROSE],
      pace: &[],
      reads: false,
      far: None,
      steps: &[
        step(
          "b",
          &[Act::Type("Where does the time of the open go?"), Act::Key("Enter")],
          "See the trace",
        ),
        // A click on the mark of a cut line shows the line whole under it. A page up brings the
        // line in view where the answer fills the window.
        step(
          "x-whole",
          &[Act::Key("PageUp"), Act::Pointer { on: "…+", button: 0 }],
          r"55% |\n| boot",
        ),
      ],
    }),
  },
  Scenario {
    name: "12-thinking",
    has: Has::Walk(Walk {
      sessions: &[],
      words: &["await bash('sleep 2')\nclose('The open spends its time in the seed.')"],
      pace: &[],
      reads: false,
      far: None,
      steps: &[
        step(
          "a",
          &[Act::Type("Where does the time of the open go? THINK"), Act::Key("Enter")],
          "sleep 2",
        ),
        // The code line holds the words of the answer from frame a, so the step waits for the
        // close.
        step("c", &[], "✓ Resolved"),
        // A click on the line of the thought opens it whole, under the block.
        step("d", &[Act::Pointer { on: "thought ·", button: 0 }], "checkpoint"),
      ],
    }),
  },
  Scenario {
    name: "13-command",
    has: Has::Walk(Walk {
      sessions: &[],
      words: &[
        "out = await bash('for i in 1 2 3 4 5 6 7 8 9 10; do echo step $i of 10; sleep 1; done')\nclose(out.stdout.content)",
        "out = await bash('seq 1 300')\nclose(f'It printed {len(out.stdout.lines)} lines.')",
        "fast, slow = bash('echo fast'), bash('sleep 4; echo slow')\nfirst = await fast\nlast = await slow\nclose(f'Both ran: {first.stdout.content.strip()} and {last.stdout.content.strip()}.')",
      ],
      pace: &[],
      reads: false,
      far: None,
      steps: &[
        step("a", &[Act::Type("Run the slow script."), Act::Key("Enter")], "⎋ stop"),
        // The last line that the command writes shows while it still runs, so the step waits
        // for the close of the thread: a message sent before it would go to the word that awaits.
        step("b", &[], "✓ Resolved"),
        step(
          "c",
          &[Act::Type("Print three hundred lines."), Act::Key("Enter")],
          "It printed 300 lines.",
        ),
        // A line that awaits two commands at once says how many of them ended while it runs.
        step("d", &[Act::Type("Run the two checks at once."), Act::Key("Enter")], "1 of 2 ended"),
        step("e", &[], "Both ran: fast and slow."),
      ],
    }),
  },
  Scenario {
    name: "14-changes",
    has: Has::Walk(Walk {
      sessions: &[],
      words: &[
        CHANGES,
        "close('It sums the count of each item.')",
        "count = 2\nawait bash('sleep 600')",
      ],
      pace: &[],
      reads: false,
      far: None,
      steps: &[
        step(
          "a",
          &[Act::Type("Count the items, and add a report."), Act::Key("Enter")],
          "Each item",
        ),
        step("b", &[Act::Key("C-x"), Act::Key("g")], "main · src/inventory/models.py"),
        step("c", &[Act::Key("Down")], "return sum"),
        step("d", &[Act::Key("Escape")], "Each item"),
        // A click on a file that a line wrote opens the block, with the diff under the line.
        step("x-diff", &[Act::Pointer { on: "models.py  +2", button: 0 }], "+     self.count = 0"),
        // A revert writes back what the life knew before the answer, as a word of his.
        step("x-revert", &[Act::Pointer { on: "↶ Revert", button: 0 }], "models.py  +1 −2"),
        // A click on a line of the diff makes the next message about that line.
        step(
          "x-anchor",
          &[
            Act::Key("C-x"),
            Act::Key("g"),
            Act::Until("return sum(one.count"),
            Act::Pointer { on: "return sum(one.count", button: 0 },
          ],
          "Say what to do with report.py, line 2",
        ),
        // The message is a sub thread of its own, which opens, and its model answers in it.
        step(
          "x-anchored",
          &[Act::Type("Say what it sums."), Act::Key("Enter")],
          "It sums the count of each item.",
        ),
        step("x-reply", &[Act::Type("Keep it open."), Act::Key("Enter")], "count = 2"),
        // A sub thread that works asks a confirm, as a thread of the list does.
        step(
          "x-resolve",
          &[Act::Key("C-x"), Act::Key("r"), Act::Key("C-x"), Act::Key("r")],
          "✓ Resolved",
        ),
        // The thread that it is on names it under its messages.
        step("x-back", &[Act::Key("Escape")], "1 sub thread resolved"),
        // The review marks its line, and a click on that line opens it again.
        step(
          "x-mark",
          &[Act::Key("C-x"), Act::Key("g"), Act::Until("return sum(one.count")],
          "return sum(one.count",
        ),
        step(
          "x-open",
          &[Act::Pointer { on: "return sum(one.count", button: 0 }],
          "◆ report.py:2 ·",
        ),
      ],
    }),
  },
  Scenario {
    name: "15-failure",
    has: Has::Walk(Walk {
      sessions: &[],
      // The word that fails is one line: a line after it joins the rung or not as the World
      // times the fault, so the failed line would differ between two captures.
      words: &[
        "await bash(\"sh -c 'for i in 1 2 3 4 5; do echo error line $i >&2; done; exit 2'\")",
        "close('The command failed with exit 2.')",
      ],
      pace: &[],
      reads: false,
      far: None,
      steps: &[
        step("a", &[Act::Type("Run the check."), Act::Key("Enter")], "failed with exit 2"),
        // The provider says it is overloaded, so the window says when it asks the model again.
        step("b", &[Act::Type("Run it again. ERROR"), Act::Key("Enter")], "Attempt 2 of 10"),
        // He stops the retries, then the command line of the model fails for a cause that does
        // not pass, so the thread shows why.
        step(
          "c",
          &[
            Act::Key("Escape"),
            Act::Key("Escape"),
            Act::Type("Check the build. FAIL"),
            Act::Key("Enter"),
          ],
          "answered nothing",
        ),
      ],
    }),
  },
  Scenario {
    name: "16-ask",
    has: Has::Walk(Walk {
      sessions: &[],
      words: &[ASKS],
      pace: &[],
      reads: false,
      far: None,
      steps: &[
        step(
          "b",
          &[Act::Type("Place the list of cards."), Act::Key("Enter")],
          "Where does the list",
        ),
        step("c", &[Act::Key("Enter")], "right"),
      ],
    }),
  },
  Scenario {
    name: "17-steer",
    has: Has::Walk(Walk {
      sessions: &[],
      words: &["await bash('sleep 20')\nclose('slept')", "await bash1\nclose('The bench ran.')"],
      pace: &[],
      reads: false,
      far: None,
      steps: &[
        step("a", &[Act::Type("Wait for the bench."), Act::Key("Enter")], "⎋ stop"),
        step("b", &[Act::Type("Stop after the boot."), Act::Key("Enter")], "Stop after the boot."),
        // Alt and enter queue a message, which waits as a chip until the thread rests.
        step("q", &[Act::Type("Then rebase on main."), Act::Key("M-Enter")], " queued "),
        step("c", &[Act::Key("Escape"), Act::Key("Escape")], "✓ Resolved"),
      ],
    }),
  },
  Scenario {
    name: "18-parallel",
    has: Has::Walk(Walk {
      sessions: &[],
      // The claude command line of the tests answers its turns in the order that it reads them, so
      // the word starts the agent that takes a turn, waits until it took it, then starts the one
      // that takes the next; the agent that reads the boot only thinks, for ever. The bench runs
      // past the end of the walk, so the root awaits it at each step.
      words: &[
        "bench = prompt(str, 'Run the bench of the replay', on=chain('bench'))\nboot = prompt(str, 'Read the boot of the host\\nPONDER', on=chain('boot'))\nawait wait(5, bench)\nlines = await prompt(str, 'Count the lines of README.md', on=chain('count'))\nawait wait(600, bench)",
        "report = await bash('sleep 300; echo the bench ran')\nclose(report.stdout.content)",
        "close('README.md has 3 lines.')",
      ],
      pace: &[],
      reads: false,
      far: None,
      steps: &[
        step(
          "b",
          &[Act::Type("Measure the open on three chains."), Act::Key("Enter")],
          "✓ Resolved  opus",
        ),
        // The pointer on the line that awaits the bench lights the row of the bench, where it
        // stands under the line that made it.
        step("x-awaits", &[Act::Pointer { on: "await wait(600, bench)", button: 35 }], "wait(600"),
        // The pointer on a verb shows its call and its contract on the head of the block, and on
        // a name its type and its value; a click on the name goes to the line that bound it.
        step("x-verb", &[Act::Pointer { on: "prompt(str, 'Run", button: 35 }], "prompt(shape"),
        step("x-value", &[Act::Pointer { on: "bench)", button: 35 }], "bench: "),
        step("x-went", &[Act::Pointer { on: "bench)", button: 0 }], "bench: "),
        // A click on the call of `prompt` goes to the chain that runs it, and back and forward.
        step("x-prompt", &[Act::Pointer { on: "prompt(str, 'Run", button: 0 }], "· agent · "),
        step("x-back", &[Act::Key("Escape")], "Forward"),
        step("x-forward", &[Act::Key("C-x"), Act::Key("Right")], "· agent · "),
        step("x-home", &[Act::Key("Escape")], "Forward"),
        step("d", &[Act::Pointer { on: "└ report = await", button: 0 }], "· agent · "),
        step("c", &[Act::Key("Escape")], "demo · Measure"),
        // The agent that only thinks opens on its thought, whose last words stand brighter.
        step("x-thinks", &[Act::Pointer { on: "└ thinks", button: 0 }], "· agent · "),
        step("x-root", &[Act::Key("Escape")], "demo · Measure"),
        // The chains of the life stand as a tree under the bar of the time: the agents that work,
        // in color, under the root that made them, and the one that rests is found by a text.
        step("x-chains", &[Act::Key("C-x"), Act::Key("t")], "type to find a chain"),
        // Each prompt of the root to an agent flows down to its track, and the pointer on the flow
        // says what it asks.
        step("x-flow", &[Act::Pointer { on: "▼", button: 35 }], "root asks bench"),
      ],
    }),
  },
  Scenario {
    name: "19-context",
    has: Has::Walk(Walk {
      sessions: &[],
      words: &["close('The trace holds the open of each chain, stage by stage.')"],
      pace: &[],
      reads: false,
      far: None,
      steps: &[
        step("b", &[Act::Type("Read the trace of the open."), Act::Key("Enter")], "3%"),
        // The share is the head of the box beside the list, or the ring of the launcher while the
        // list hides: either opens each part of the box.
        step("x-top", &[Act::Pointer { on: "3%", button: 0 }], "Engine"),
        // A click on the bar shows the cache, and the key of the context steps the box to fold.
        step("e", &[Act::Pointer { on: "━━━━", button: 0 }], "From the cache"),
        step("f", &[Act::Key("C-x"), Act::Key("c")], "3%"),
      ],
    }),
  },
  Scenario {
    name: "20-models",
    has: Has::Walk(Walk {
      sessions: &[],
      words: &[],
      pace: &[],
      reads: false,
      far: None,
      steps: &[
        // Before any session, the models offer the sign-in of Codex, for a ChatGPT plan: in the
        // browser of this computer, or from any device with a code.
        step("a", &[Act::Key("C-x"), Act::Key("m")], "Sign in with a code"),
        // The sign-in with a code shows the code and the page where he enters it.
        step("b", &[Act::Type("with a code"), Act::Key("Enter")], "Enter the code there"),
        // The page opens, he enters the code there, and the models of his account join the list.
        step("c", &[Act::Key("Enter")], "Codex is connected"),
        // A model of Codex taken in the list that stays open, for the new session, which its first
        // message starts. No message goes.
        step("d", &[Act::Type("gpt"), Act::Key("Enter")], "gpt-5.5"),
        // The arrows change the effort of the chosen model in the list, which takes it with its
        // row: the left one lowers it.
        step("e", &[Act::Key("C-x"), Act::Key("m"), Act::Key("Left")], "in use         medium"),
        step("f", &[Act::Key("Enter")], "gpt-5.5  medium"),
      ],
    }),
  },
  Scenario {
    name: "21-config",
    has: Has::Walk(Walk {
      sessions: &[],
      words: &[],
      pace: &[],
      reads: false,
      far: None,
      steps: &[
        step("x-thread", &[Act::Type("Where does this run?"), Act::Key("Enter")], "reply 1"),
        // The settings of the session list each key with what it does, its value and its layer.
        step("a", &[Act::Key("C-x"), Act::Key(",")], "Settings"),
        // Enter flips a boolean in the local config of the project, as the key `skills` takes it,
        // and the list reads it again.
        step("b", &[Act::Type("naming"), Act::Key("Down"), Act::Key("Enter")], "true  local"),
        // The kit that the change turned on lists its keys, each with what it does.
        step(
          "c",
          &[Act::Key("Escape"), Act::Key("C-x"), Act::Key(","), Act::Type("naming")],
          "naming.actor",
        ),
        // A key of text takes the value that he writes, in the layer of its scope.
        step("d", &[Act::Type(".actor"), Act::Key("Enter")], "a string"),
        step("e", &[Act::Type("opus"), Act::Key("Enter")], "\"opus\"  user"),
        // The arrows show the group of the keys of furb, which a tab names beside each kit.
        step("x-group", &[Act::Key("Left")], "max_thinking_tokens"),
        // A value that the property of its key does not take is refused, with what it takes.
        step(
          "f",
          &[
            Act::Key("Escape"),
            Act::Key("C-x"),
            Act::Key(","),
            Act::Type("thinking"),
            Act::Key("Enter"),
            Act::Type("x"),
            Act::Key("Enter"),
          ],
          "takes an integer",
        ),
      ],
    }),
  },
  Scenario {
    name: "22-help-keys",
    has: Has::Walk(Walk {
      sessions: &[],
      words: &[],
      pace: &[],
      reads: false,
      far: None,
      steps: &[
        step("a", &[Act::Key("C-x"), Act::Key("?")], "each key"),
        step("c", &[Act::Key("Escape"), Act::Key("C-p")], "Search a thread"),
        step("d", &[WHEEL_DOWN; 40], "The model of the input"),
        step("e", &[Act::Pointer { on: "The model of the input", button: 0 }], "Search a model"),
      ],
    }),
  },
  Scenario {
    name: "23-extensions",
    has: Has::Walk(Walk {
      sessions: &[],
      words: &[],
      pace: &[],
      reads: false,
      far: None,
      steps: &[
        step("x-thread", &[Act::Type("Which skills play here?"), Act::Key("Enter")], "reply 1"),
        // Each skill of the session, the extensions that play first, each with how it plays and
        // where it comes from.
        step("a", &[Act::Key("C-x"), Act::Key("x")], "on by default"),
        // Enter turns an extension on, and it plays at once.
        step("b", &[Act::Type("web"), Act::Key("Enter")], "on  furb"),
        // A second enter turns it off for the next session, and it plays on until this one ends.
        step("c", &[Act::Key("Enter")], "off at the next session"),
      ],
    }),
  },
  Scenario {
    name: "24-remote",
    has: Has::Walk(Walk {
      sessions: &[],
      words: &[],
      pace: &[],
      reads: false,
      far: Some("studio"),
      steps: &[
        step("a", &[], READY),
        step("b", &[Act::Key("C-x"), Act::Key("h")], "Search a machine"),
        // The far machine takes a new session once it told its project, `lab`.
        step("c", &[Act::Type("studio")], "1 / 1"),
        step("d", &[Act::Key("Enter")], "studio ▾"),
        // The session runs on the far machine, in its project, and the list holds the cards of both
        // machines.
        step("e", &[Act::Type("Where does this run?"), Act::Key("Enter")], "  reply 1"),
      ],
    }),
  },
  Scenario {
    name: "25-narrow",
    has: Has::Sized(
      SIZES[1],
      &[("13-command", "c"), ("05-prompt-box", "d"), ("07-prompt-complete", "b")],
    ),
  },
  Scenario {
    name: "26-wide",
    has: Has::Sized(SIZES[2], &[("11-prose-stream", "b"), ("14-changes", "b")]),
  },
  Scenario {
    name: "27-mouse",
    has: Has::Walk(Walk {
      sessions: &[],
      words: &[ASKS],
      pace: &[],
      reads: false,
      far: None,
      steps: &[
        step(
          "x-thread",
          &[Act::Type("Place the list of cards."), Act::Key("Enter")],
          "Where does the list",
        ),
        step("a", &[Act::Pointer { on: "Resolve", button: 35 }], "Resolve"),
        step("b", &[Act::Pointer { on: "Resolve", button: 0 }], "✓ Resolved"),
        step("c", &[Act::Pointer { on: "opus/high", button: 64 }], "Place the list"),
      ],
    }),
  },
  Scenario { name: "28-readme", has: Has::Walk(README) },
  Scenario {
    name: "29-read-only",
    has: Has::Walk(Walk {
      sessions: &[],
      words: &["await bash('sleep 60')\nclose('slept')"],
      pace: &[],
      reads: true,
      far: None,
      steps: &[
        // Another window of the operator starts a session; this one only reads, as `furb --read`
        // opens it, and sees that session live.
        step("a", &[Act::Said("Run the bench of the replay.")], "1 session runs"),
        step(
          "b",
          &[Act::Key("C-p"), Act::Type("bench"), Act::Key("Enter")],
          "Read only · you watch this session",
        ),
      ],
    }),
  },
  Scenario {
    name: "30-no-project",
    has: Has::Walk(Walk {
      sessions: &[],
      words: &[ASKS],
      pace: &[],
      reads: false,
      far: None,
      steps: &[
        // The projects of a new session end with no project, for work with no code.
        step("a", &[Act::Key("C-x"), Act::Key("p")], "No project"),
        // The launcher and the home say that the session runs with no project.
        step("b", &[Act::Type("no project"), Act::Key("Enter")], "with no project"),
        // The session runs in a folder of its own, and its card has no badge of a project.
        step("c", &[Act::Type("Plan the week."), Act::Key("Enter")], "Where does the list"),
      ],
    }),
  },
];
