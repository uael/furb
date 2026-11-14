//! The scenarios of the captures: the 27 daily paths on which the window and the other programs in
//! a terminal are judged. Each step of a scenario is one frame. A scenario that the window has is a
//! walk: the words that its scripted model answers, turn by turn, and its steps. Another stands at a
//! size on the steps of other scenarios. A scenario that the window has not names its gap.

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
}

/// A step: its name, which names its frame, what it does, and a text that its screen shows once it
/// is done.
#[derive(Debug)]
pub(crate) struct Step {
  pub(crate) name: &'static str,
  pub(crate) acts: &'static [Act],
  pub(crate) until: &'static str,
}

/// A walk of the window: the word of the model at each turn, in order, and the steps.
#[derive(Debug)]
pub(crate) struct Walk {
  pub(crate) words: &'static [&'static str],
  pub(crate) steps: &'static [Step],
}

/// What the window has of a scenario.
#[derive(Debug)]
pub(crate) enum Has {
  Walk(Walk),
  /// Steps of other scenarios at one size: each the name of a scenario and of its step.
  Sized(Size, &'static [(&'static str, &'static str)]),
  /// No such thing: the gap of the judge that says what the window lacks.
  None(&'static str),
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
      Has::Sized(..) | Has::None(_) => None,
    }
  }
}

/// The scenario that a word names: its number, as `05`, or its whole name.
pub(crate) fn named(word: &str) -> Option<&'static Scenario> {
  SCENARIOS
    .iter()
    .find(|scenario| scenario.name == word || scenario.name.split('-').next() == Some(word))
}

const fn step(name: &'static str, acts: &'static [Act], until: &'static str) -> Step {
  Step { name, acts, until }
}

const fn none(gap: &'static str) -> Has {
  Has::None(gap)
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
const ASKS: &str = "side = await prompt(Literal['right', 'left'], 'Where does the list of cards stand?', to=OPERATOR)\nclose(side)";

/// The scenarios, in their order.
pub(crate) const SCENARIOS: &[Scenario] = &[
  Scenario {
    name: "01-first-run",
    has: Has::Walk(Walk {
      words: &[],
      steps: &[step("a", &[], READY), step("b", &[Act::Key("C-x"), Act::Key("?")], "each key")],
    }),
  },
  Scenario { name: "02-resume", has: none("4. The list says what needs him, and only that") },
  Scenario {
    name: "03-sessions-many",
    has: none("4. The list says what needs him, and only that"),
  },
  Scenario { name: "04-scroll-back", has: none("12. Reading a long thread") },
  Scenario {
    name: "05-prompt-box",
    has: Has::Walk(Walk {
      words: &[],
      steps: &[
        step("a", &[], READY),
        step("b", &[Act::Type("Where does the time of the open go?")], "open go?"),
        step(
          "c",
          &[
            Act::Key("C-c"),
            Act::Paste(
              "Read the boot, then the seed.\nThen the turns of each chain.\nThen the stages.\nSum them.\nSay where the time goes.\nKeep the numbers.\nName each stage.\nGive the share of each.\nSay what to cut first, and why it costs the most of the open of a record that holds a hundred thousand facts.\nThen the test.\nThen the proof.\nThen stop.",
            ),
          ],
          "Then stop.",
        ),
      ],
    }),
  },
  Scenario { name: "06-prompt-history", has: none("10. The input: history, paste, image, mode") },
  Scenario { name: "07-prompt-complete", has: none("17. After the first window") },
  Scenario { name: "08-file-picker", has: none("17. After the first window") },
  Scenario { name: "09-image", has: none("10. The input: history, paste, image, mode") },
  Scenario { name: "10-shell-mode", has: none("10. The input: history, paste, image, mode") },
  Scenario {
    name: "11-prose-stream",
    has: Has::Walk(Walk {
      words: &[PROSE],
      steps: &[step(
        "b",
        &[Act::Type("Where does the time of the open go?"), Act::Key("Enter")],
        "See the trace",
      )],
    }),
  },
  Scenario { name: "12-thinking", has: none("2. The work moves and says how long") },
  Scenario { name: "13-command", has: none("3. A command shows what it printed") },
  Scenario { name: "14-changes", has: none("17. After the first window") },
  Scenario { name: "15-failure", has: none("5. Failure: the cause, the way out, the retry") },
  Scenario {
    name: "16-ask",
    has: Has::Walk(Walk {
      words: &[ASKS],
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
  Scenario { name: "17-steer", has: none("6. Stop and steer leave a trace") },
  Scenario { name: "18-parallel", has: none("8. The dock of agents") },
  Scenario { name: "19-context", has: none("7. The context box") },
  Scenario { name: "20-models", has: none("9. The launcher picks the model and its effort") },
  Scenario { name: "21-config", has: none("17. After the first window") },
  Scenario {
    name: "22-help-keys",
    has: Has::Walk(Walk {
      words: &[],
      steps: &[
        step("a", &[Act::Key("C-x"), Act::Key("?")], "each key"),
        step("c", &[Act::Key("Escape"), Act::Key("C-p")], "commands"),
      ],
    }),
  },
  Scenario { name: "23-extensions", has: none("17. After the first window") },
  Scenario { name: "24-remote", has: none("15. Remote") },
  Scenario {
    name: "25-narrow",
    has: Has::Sized(
      SIZES[1],
      &[("13-command", "c"), ("05-prompt-box", "c"), ("07-prompt-complete", "b")],
    ),
  },
  Scenario {
    name: "26-wide",
    has: Has::Sized(SIZES[2], &[("11-prose-stream", "b"), ("14-changes", "b")]),
  },
  Scenario {
    name: "27-mouse",
    has: Has::Walk(Walk {
      words: &[ASKS],
      steps: &[
        step(
          "x-thread",
          &[Act::Type("Place the list of cards."), Act::Key("Enter")],
          "Where does the list",
        ),
        step("a", &[Act::Pointer { on: "Resolve", button: 35 }], "Resolve"),
        step("b", &[Act::Pointer { on: "Resolve", button: 0 }], "Place the list"),
        step("c", &[Act::Pointer { on: "opus/high", button: 64 }], "Place the list"),
      ],
    }),
  },
];
