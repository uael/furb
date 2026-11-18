//! The tasks that run one real life of furb in a folder of their own: the smoke, the play, and the
//! bench of compaction on a copy of a record. Each life runs through the program furb on monty,
//! the engine that a user runs, as [`Life`] says, and each folder stays for a person to read.

use std::{
  collections::BTreeMap,
  env, fs,
  path::Path,
  process::{self, ExitCode},
  time::{Duration, Instant},
};

use crate::deepswe::{Ask, Asked, Fault, Life, Mark, Numbers, furb, removed};

/// What the smoke asks, of a file that the model must read to answer.
const SMOKE: &str = "How many lines does the file a.txt hold?";
/// What that file holds.
const LINES: &str = "one\ntwo\nthree\n";

/// The dollars that the play may spend over each chain of its record.
const CEILING: f64 = 4.0;
/// What the play asks the model, with the path of the engine in place of `{engine}`.
const PLAY: &str = "You have the engine and a directory of your own. Use them, in this order.

1. Read the engine at {engine} and say in one line what a chain is.
2. Write a small python module into your directory, run it with bash, and read its stdout.
3. Open a chain with your own chain as its source and take() as its filter, so the model you ask on it reads
   none of your work, and end that word without closing anything. In the next word, prompt that chain with no actor
   named, with shape int for a small sum, and await the value. Say in that message that the answer is a close of the number.
4. Prompt the operator, which is the actor named \"operator\", with shape str for a word, await it, and hold the
   word it gives back.
5. Debug one value with a template string, and peek at one act you made.
6. Make a subdirectory, cd into it by its full path, and show that cwd changed.
7. Close with a list of exactly 7 items, in this order: your one line about a chain (str), the stdout of your
   module (str), the int the chain you opened gave (int), the word of the operator (str), the id of the act you
   peeked at (str), the working directory you are now in (str), and the id of the chain you opened (str).
";
/// The word that the operator answers the model with.
const SAID: &str = "windlass";
/// The word that holds the first life of the play to what it asked, on its record, with the folder
/// of the play in place of `{folder}`. It raises at the first thing that the life did not do.
const PLAYED: &str = r#"root = scope(acting())
got = peek([x[1] for x in transcript(root, "prompt") if x[2] == OPERATOR and x[4] == "list"][0])
said = transcript(root)
forks = [x[1] for x in said if x[0] == "chain" and x[5] == root]
asked = [x[1] for x in said if x[0] == "prompt" and x[6] == OPERATOR]
assert isinstance(got, list) and len(got) == 7, got
assert got[6] in forks, f"{got[6]} is no chain that the root opened on itself: {forks}"
assert [x for x in transcript(got[6]) if x[0] == "prompt"], f"no prompt stands on {got[6]}"
assert [t for t in turns(on=got[6]) if t[0] == "assistant"], f"no model answered on {got[6]}"
assert [peek(x) for x in asked] == ["windlass"] == got[3:4], f"the operator was asked {asked}"
assert [x for x in said if x[0] == "write" and x[4].path.endswith(".py")], "no python module was written"
assert all(isinstance(x, str) and x.strip() for x in got[:2]) and isinstance(got[2], int), got
assert [t for t in turns(on=root) if " debugged" in t[1]], "no debug stands in the turns"
assert get(got[4]), f"{got[4]!r} names no act"
assert got[5] != {folder} and cwd(on=root) == got[5], f"the directory is {cwd(on=root)}"
close(f"the first life gave {got!r}")"#;
/// What the play asks the life on the record of its first life.
const AGAIN: &str = "How many items did you give back? Close with the number and nothing else.";

/// The word of the bench of compaction, with the chain in place of `{chain}` and whether it asks
/// the model after the cut in place of `{after}`.
const CONDENSED: &str = r#"await skill("compaction")
c = {chain}
def replied():
  return [x[1] for x in transcript(c, "reply") if scope(x[1]) == c and isinstance(peek(x[1]), tuple)]
out = {"chain": c, "label": get(c)[4], "share before": filled(replied()[-1])}
out["acts before"] = len([a for a in transcript(c) if question(a)])
await condense(c)
out["choice"] = next(peek(x[1]) for x in reversed(transcript(c, "prompt")) if x[5] == BRIEF)
out["acts kept"] = len([a for a in transcript(c) if question(a)])
if {after}:
  await prompt(str, "Say in one line what this chain works on now.", on=c)
  out |= {"share after": filled(replied()[-1]), "word after": peek(replied()[-1])[1]}
close(out)"#;

/// The smoke of a real life: one model, one record, and a second life that its record answers.
pub(crate) fn smoke(root: &Path, to: Option<String>) -> Result<ExitCode, Fault> {
  let life = scratch(root, "smoke", to, Duration::from_mins(10))?;
  fs::write(life.app.join("a.txt"), LINES)
    .map_err(|source| Fault::File { path: life.app.join("a.txt"), source })?;
  let mut mark = Mark::default();
  // A run ends with its own rung, so the grant goes in a second run, which plays the skill that
  // the first loaded: a dollar, and half the window for one answer, which a free model that
  // loops passes too.
  life.word("await skill('grant')", &mut mark)?;
  life.word("grant(1.0, 0.5)", &mut mark)?;
  for which in ["first", "second"] {
    let got =
      answered(life.prompt(&Ask { message: SMOKE, shape: "int", operator: "" }, &mut mark)?)?;
    let answers = Numbers::of(&life.record)?.answers;
    println!("the {which} life gave {got}, and the record holds {answers} answer(s) of a model");
    if got != LINES.lines().count().to_string() {
      return Err(Fault::Unproven(format!("the {which} life answered {got}")));
    }
  }
  let numbers = Numbers::of(&life.record)?;
  if numbers.answers != 1 {
    return Err(Fault::Unproven(
      "the second life asked a model for what its record holds".to_owned(),
    ));
  }
  println!("the smoke spent ${:.6}\nSMOKE OK", numbers.usd);
  Ok(ExitCode::SUCCESS)
}

/// The play: a life asked to use each part of its runtime, held to what it did by a word on its
/// record, then a second life on that record, which asks no model for what it holds and takes one
/// prompt more.
pub(crate) fn play(root: &Path, to: Option<String>) -> Result<ExitCode, Fault> {
  let life = scratch(root, "play", to, Duration::from_mins(30))?;
  let mut mark = Mark::spending(CEILING);
  life.word(&format!("await skill('grant')\nrung('grant(usd={CEILING:?})')"), &mut mark)?;
  let engine = root.join("src").join("furb").join("engine.py");
  let message = PLAY.replace("{engine}", &engine.display().to_string());
  let operator = format!("{SAID}\n");
  answered(
    life.prompt(&Ask { message: &message, shape: "list", operator: &operator }, &mut mark)?,
  )?;
  let answers = Numbers::of(&life.record)?.answers;
  let folder = format!("{:?}", life.app.display().to_string());
  println!("{}", life.word(&PLAYED.replace("{folder}", &folder), &mut mark)?);
  if Numbers::of(&life.record)?.answers != answers {
    return Err(Fault::Unproven(
      "the life on the record asked a model for what it holds".to_owned(),
    ));
  }
  let more =
    answered(life.prompt(&Ask { message: AGAIN, shape: "int", operator: "" }, &mut mark)?)?;
  if more != "7" {
    return Err(Fault::Unproven(format!("the second life counted {more} items, not 7")));
  }
  let numbers = Numbers::of(&life.record)?;
  println!(
    "the second life counted {more} items\nthe play spent ${:.6} over {} answers\nPLAY OK",
    numbers.usd, numbers.answers
  );
  Ok(ExitCode::SUCCESS)
}

/// The bench of compaction: on a copy of a record, a life plays the skill compaction and condenses
/// one chain, as `compacted()` does once, and prints the share of the window that the chain read
/// before the cut and, unless dry, after it, the choice of its model, and the acts that it kept.
pub(crate) fn compaction(
  root: &Path,
  record: &Path,
  chain: &str,
  dry: bool,
) -> Result<ExitCode, Fault> {
  let life = scratch(root, "compaction", None, Duration::from_mins(30))?;
  fs::copy(record, &life.record)
    .map_err(|source| Fault::File { path: record.to_owned(), source })?;
  let word = CONDENSED
    .replace("{chain}", &format!("{chain:?}"))
    .replace("{after}", if dry { "False" } else { "True" });
  println!("{}", life.word(&word, &mut Mark::default())?);
  Ok(ExitCode::SUCCESS)
}

/// A life on a new folder of its own in the temporary folder, whose log stands in `.run`.
fn scratch(root: &Path, name: &str, to: Option<String>, limit: Duration) -> Result<Life, Fault> {
  let app = env::temp_dir().join(format!("furb-{name}-{}", process::id()));
  removed(&app)?;
  let runs = app.join(".run");
  fs::create_dir_all(&runs).map_err(|source| Fault::File { path: runs.clone(), source })?;
  println!("the folder of the {name} is {}", app.display());
  let life = Life {
    furb: furb(root)?,
    record: app.join("record.jsonl"),
    runs,
    app,
    env: BTreeMap::new(),
    to,
    limit: Some(limit),
    began: Instant::now(),
  };
  Ok(life)
}

/// What a command of furb printed, or why the life stopped, as a fault.
fn answered(asked: Asked) -> Result<String, Fault> {
  match asked {
    Asked::Answered(said) => Ok(said),
    Asked::Stopped(why) => Err(Fault::Unproven(why)),
  }
}
