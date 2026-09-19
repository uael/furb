//! A World of this machine: the disk, the clock, chance, and the commands the machine runs.
//!
//! A host may write its own World and many will, since what a life may touch is the host's to decide. This one is
//! for a host that wants the ordinary answer: files under one directory, a real clock, and commands in a shell.
//! It is the Rust counterpart of `src/furb/world.py`, and it answers the same facts the same way.
//!
//! What it does not do is talk to a model or to a person, because neither is a thing of this machine. A host
//! gives those through [`Talks`], and everything else is answered here.
//!
//! A command outlives the fact that started it, so it says what it wrote and what it came to through the
//! [`Voice`] this World holds, long after the start was heard.

use std::{
  collections::HashMap,
  fs,
  io::{self, PipeReader, Read, Write},
  path::PathBuf,
  process::{Child, Command, Stdio},
  sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
  },
  thread,
  time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use crate::{
  fact::{Fact, Value},
  host::WORLD,
  record::Entry,
  voice::Voice,
  world::{Reply, World},
};

/// The most bytes this World reads of one file, since a text a model cannot hold is no answer.
pub const CAP: usize = 524_288;
/// The bytes this World reads of a stream at a time, which is one out word of a command.
pub const PIPE: usize = 65_536;
/// The acts this World starts, which are the ones it keeps while they wait for their start.
const STARTS: [&str; 3] = ["bash", "wait", "prompt"];

/// What is no thing of this machine: a model, and the person at the far end of a prompt.
///
/// Both answer whenever they answer, so neither answers here. A host is given the [`Voice`] of the World with
/// every call, and it says the answer into that Voice when it holds it: a model answers an ask with an answer
/// fact, and the operator answers a prompt by closing the prompt with the value the shape asked for.
pub trait Talks {
  /// One ask of a model: the rung that asks, the chain it is on, the actor, and the turns as they stand.
  fn asked(&mut self, rung: &str, on: &str, actor: &str, turns: &Value, voice: &Voice);

  /// One prompt of the operator: the act, the shape it wants, and the message it carries.
  fn shown(&mut self, about: &str, shape: &str, message: &str, voice: &Voice);
}

/// One command of the World: what waits to be fed to it, and its process once the process stands.
///
/// A rung writes the stdin of a command as soon as it has made the command, which may be before the World has
/// the process up, so what is fed before then waits here and goes in the order it was said once it stands.
#[derive(Debug)]
struct Running {
  /// The chain the command is on, since a control of that chain ends it.
  on: String,
  /// The process, once the machine has it up.
  child: Option<Arc<Mutex<Child>>>,
  /// What waits to be fed, said before the process stood, where a text of nothing closes the stdin.
  waiting: Vec<Option<String>>,
  /// Whether the command is over, so that nothing says its exit twice.
  over: Arc<AtomicBool>,
}

impl Running {
  /// One text into the stdin of the command, and a text of nothing closes that stdin.
  fn feed(&mut self, text: Option<&str>) {
    let Some(child) = self.child.as_ref() else {
      self.waiting.push(text.map(str::to_owned));
      return;
    };
    let Ok(mut held) = child.lock() else { return };
    match text {
      Some(said) => {
        if let Some(stdin) = held.stdin.as_mut() {
          let _ = stdin.write_all(said.as_bytes());
          let _ = stdin.flush();
        }
      }
      None => {
        held.stdin.take();
      }
    }
  }

  /// The process of the command, up, and everything that waited to be fed to it, fed.
  fn stands(&mut self, child: Arc<Mutex<Child>>) {
    self.child = Some(child);
    for text in std::mem::take(&mut self.waiting) {
      self.feed(text.as_deref());
    }
  }

  /// The command dies, and nothing of it says its exit again.
  fn slay(&self) {
    let Some(child) = self.child.as_ref() else { return };
    // The whole group dies and not the shell alone, since a command grows a tree of its own.
    let held = child.lock().ok().map(|mut one| (one.id(), one.kill()));
    if let Some((pid, _)) = held {
      let _ = Command::new("sh").arg("-c").arg(format!("kill -KILL -{pid} 2>/dev/null")).status();
    }
  }
}

/// What the World is waiting for the engine to answer, and the fact it answers once it holds it.
#[derive(Debug)]
enum Asking {
  /// A read of a path, waiting for where the paths of its chain resolve.
  Read(Fact),
  /// A write of a text, waiting for where the paths of its chain resolve.
  Write(Fact),
  /// A command, waiting for whether its stderr flows into its stdout.
  Merged(Fact),
  /// A command that knows its streams, waiting for where it runs.
  Where(Fact, bool),
}

/// The World of this machine, for one life.
pub struct Live<T> {
  /// Where the chains of this life start.
  directory: PathBuf,
  /// The actors this World offers, each as its name, its efforts and its window.
  roster: Value,
  /// The actor a prompt goes to when it names none.
  actor: String,
  /// The file the record is kept in, and nothing when this life keeps none.
  record: Option<PathBuf>,
  /// What answers a model and a person.
  pub talks: T,
  /// How this World says what it did not answer where it was asked.
  voice: Voice,
  /// The acts this World starts, kept from when it hears them until it is told to start them.
  acts: HashMap<String, Fact>,
  /// The commands that stand, by the act that made each one.
  running: HashMap<String, Running>,
  /// What the World is waiting for the engine to answer.
  asking: Option<Asking>,
  /// The state of the number this World draws.
  drawn: u64,
}

impl<T: Talks> Live<T> {
  /// A World of this machine, under one directory.
  pub fn new(directory: impl Into<PathBuf>, actor: impl Into<String>, roster: Value, talks: T, voice: Voice) -> Self {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |held| held.as_nanos() as u64);
    Live {
      directory: directory.into(),
      roster,
      actor: actor.into(),
      record: None,
      talks,
      voice,
      acts: HashMap::new(),
      running: HashMap::new(),
      asking: None,
      drawn: now | 1,
    }
  }

  /// The file this World keeps the record in, which a later life is opened from.
  #[must_use]
  pub fn keeping(mut self, record: impl Into<PathBuf>) -> Self {
    self.record = Some(record.into());
    self
  }

  /// One path of the disk: the directory of the life, where the chain stands, and then the path.
  ///
  /// A chain holds the path a cd was given, which may name no directory of its own, and the World has one place
  /// to stand such a path against: the directory every chain of the life started in.
  fn at(&self, here: &str, path: &str) -> PathBuf {
    self.directory.join(here.trim_start_matches('/')).join(path)
  }

  /// The text at a path: the file on the disk, and a refusal for the door of nothing that lives.
  fn read(&self, here: &str, path: &str) -> Value {
    if path.contains("://") {
      return Value::refused(format!("{path} is the door of nothing that lives"));
    }
    let at = self.at(here, path);
    if !at.is_file() {
      return Value::refused(format!("no file at {}", at.display()));
    }
    match fs::read(&at) {
      Ok(raw) if raw.len() > CAP => {
        Value::refused(format!("{} holds {} bytes, over the {CAP} the World reads", at.display(), raw.len()))
      }
      Ok(raw) => match String::from_utf8(raw) {
        Ok(said) => Value::text(at.display().to_string(), said),
        Err(_) => Value::refused(format!("{} is no text", at.display())),
      },
      Err(why) => Value::refused(format!("{} cannot be read: {why}", at.display())),
    }
  }

  /// The content onto the file at a path, and the text of that file as it stands on the disk after the write.
  fn write(&self, here: &str, text: &Value) -> Value {
    let path = text.field("path").and_then(Value::as_str).unwrap_or_default();
    let content = text.field("content").and_then(Value::as_str).unwrap_or_default();
    if path.contains("://") {
      return Value::refused(format!("{path} is the door of nothing that takes a word"));
    }
    let at = self.at(here, path);
    if let Some(over) = at.parent()
      && let Err(why) = fs::create_dir_all(over)
    {
      return Value::refused(format!("{} cannot be made: {why}", over.display()));
    }
    match fs::write(&at, content).and_then(|()| fs::read_to_string(&at)) {
      Ok(said) => Value::text(at.display().to_string(), said),
      Err(why) => Value::refused(format!("{} cannot be written: {why}", at.display())),
    }
  }

  /// One entry of the record onto its file, plain, as json, and on the disk before this gives back.
  fn keep(&self, entry: &Value) -> Option<()> {
    let at = self.record.as_ref()?;
    let held = entry.as_entries()?;
    let made = Entry {
      before: held.first().and_then(Value::as_str).unwrap_or_default().to_owned(),
      fact: Fact(held.get(1).and_then(Value::as_entries).unwrap_or_default().to_vec()),
      answer: held.get(2).cloned(),
    };
    if let Some(over) = at.parent() {
      fs::create_dir_all(over).ok()?;
    }
    let mut file = fs::OpenOptions::new().create(true).append(true).open(at).ok()?;
    writeln!(file, "{}", made.line()).ok()?;
    file.sync_all().ok()
  }

  /// A number this World draws, at least zero and under one.
  ///
  /// It is drawn here and not taken from a library, since what a life asks of it is that a model cannot say what
  /// comes next, and nothing more.
  fn chance(&mut self) -> f64 {
    self.drawn = self.drawn.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut held = self.drawn;
    held = (held ^ (held >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    held = (held ^ (held >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    ((held ^ (held >> 31)) >> 11) as f64 / (1u64 << 53) as f64
  }

  /// The act a start names, done, once the chain has said where its paths resolve.
  fn started(&mut self, act: &Fact, here: &str, merged: bool) -> Reply {
    let words = act.words();
    match act.kind() {
      "bash" => {
        let command = words.get(1).and_then(Value::as_str).unwrap_or_default().to_owned();
        let fed = words.get(2) == Some(&Value::Bool(true));
        self.runs(act, here, &command, fed, seconds(words.get(3), 600.0), merged);
      }
      "wait" => {
        let (voice, about) = (self.voice.clone(), act.about().to_owned());
        let held = Duration::from_secs_f64(seconds(words.get(1), 0.0));
        thread::spawn(move || {
          thread::sleep(held);
          voice.send(Fact::new("done", &about, WORLD, vec![Value::None]));
        });
      }
      "prompt" => {
        let shape = words.get(1).and_then(Value::as_str).unwrap_or_default().to_owned();
        let message = words.get(2).and_then(Value::as_str).unwrap_or_default().to_owned();
        self.talks.shown(act.about(), &shape, &message, &self.voice);
      }
      _ => {}
    }
    Reply::Nothing
  }

  /// One command in a session of its own: what it says as it says it, and its code when it is over.
  fn runs(&mut self, act: &Fact, here: &str, command: &str, fed: bool, timeout: f64, merged: bool) {
    let about = act.about().to_owned();
    let held = Running {
      on: act.on().unwrap_or_default().to_owned(),
      child: None,
      waiting: Vec::new(),
      over: Arc::new(AtomicBool::new(false)),
    };
    let over = Arc::clone(&held.over);
    self.running.insert(about.clone(), held);
    let made = self.spawned(here, command, fed, merged);
    let (child, merged) = match made {
      Ok(held) => held,
      Err(why) => {
        // The machine would not start it, so the command never runs and whoever waits for it hears why instead.
        self.running.remove(&about);
        self.voice.close(&about, Value::refused(format!("{command:?} did not start: {why}")));
        return;
      }
    };
    let child = Arc::new(Mutex::new(child));
    if let Some(one) = self.running.get_mut(&about) {
      one.stands(Arc::clone(&child));
    }
    let streams = self.streams(&about, &child, merged);
    self.ends(&about, child, over, streams, timeout);
  }

  /// One command, up, with a door for each stream it tells.
  ///
  /// A merged command writes both of its streams into one door, so the two stand in the order it wrote them, and
  /// the stderr of it tells nothing of its own.
  fn spawned(&self, here: &str, command: &str, fed: bool, merged: bool) -> io::Result<(Child, Option<PipeReader>)> {
    let mut made = Command::new("sh");
    made
      .arg("-c")
      .arg(command)
      .current_dir(self.at(here, ""))
      .stdin(if fed { Stdio::piped() } else { Stdio::null() })
      .stdout(Stdio::piped())
      .stderr(Stdio::piped());
    grouped(&mut made);
    if !merged {
      return Ok((made.spawn()?, None));
    }
    let (reader, writer) = io::pipe()?;
    made.stdout(writer.try_clone()?).stderr(writer);
    Ok((made.spawn()?, Some(reader)))
  }

  /// Every stream of a command, each said as it comes, one out fact for every part that arrives.
  fn streams(&self, about: &str, child: &Arc<Mutex<Child>>, merged: Option<PipeReader>) -> Vec<thread::JoinHandle<()>> {
    let mut doors: Vec<(&str, Box<dyn Read + Send>)> = Vec::new();
    if let Some(held) = merged {
      doors.push(("stdout", Box::new(held)));
    }
    if let Ok(mut held) = child.lock() {
      if let Some(one) = held.stdout.take() {
        doors.push(("stdout", Box::new(one)));
      }
      if let Some(one) = held.stderr.take() {
        doors.push(("stderr", Box::new(one)));
      }
    }
    doors
      .into_iter()
      .map(|(stream, door)| {
        let (voice, about, stream) = (self.voice.clone(), about.to_owned(), stream.to_owned());
        thread::spawn(move || told(&voice, &about, &stream, door))
      })
      .collect()
  }

  /// The end of a command: both streams to their end, and then its code, which is nothing after a timeout.
  fn ends(
    &self,
    about: &str,
    child: Arc<Mutex<Child>>,
    over: Arc<AtomicBool>,
    streams: Vec<thread::JoinHandle<()>>,
    timeout: f64,
  ) {
    let (voice, about) = (self.voice.clone(), about.to_owned());
    thread::spawn(move || {
      let deadline = Duration::from_secs_f64(timeout.max(0.0));
      let started = Instant::now();
      let mut late = false;
      let code = loop {
        match child.lock().ok().and_then(|mut one| one.try_wait().ok().flatten()) {
          Some(held) => break held.code().map_or(Value::None, |one| Value::Int(i64::from(one))),
          None if started.elapsed() > deadline => {
            late = true;
            let held = child.lock().ok().map(|mut one| (one.id(), one.kill()));
            if let Some((pid, _)) = held {
              let _ = Command::new("sh").arg("-c").arg(format!("kill -KILL -{pid} 2>/dev/null")).status();
            }
          }
          None => thread::sleep(Duration::from_millis(5)),
        }
      };
      for one in streams {
        let _ = one.join();
      }
      if !over.swap(true, Ordering::SeqCst) {
        voice.send(Fact::new("exited", &about, WORLD, vec![if late { Value::None } else { code }]));
      }
    });
  }

  /// Every command a control is over, ended, and each of them exited with nothing.
  fn slew(&mut self, about: &str) -> Reply {
    let held: Vec<String> =
      self.running.iter().filter(|(id, one)| under(id, about) || one.on == about).map(|(id, _)| id.clone()).collect();
    let mut said = Vec::new();
    for one in held {
      let Some(command) = self.running.get(&one) else { continue };
      command.over.store(true, Ordering::SeqCst);
      command.slay();
      said.push(Fact::new("exited", &one, WORLD, vec![Value::None]));
    }
    if said.is_empty() { Reply::Nothing } else { Reply::Say(said) }
  }
}

impl<T: Talks> World for Live<T> {
  fn hears(&mut self, fact: &Fact) -> Reply {
    let words = fact.words();
    if fact.question() && STARTS.contains(&fact.kind()) {
      // The World hears the act itself, with its plain words, and is told to start it after.
      self.acts.insert(fact.about().to_owned(), fact.clone());
    }
    match fact.kind() {
      "start" => {
        let Some(act) = self.acts.get(fact.about()) else { return Reply::Nothing };
        let (kind, on) = (act.kind().to_owned(), act.on().unwrap_or_default().to_owned());
        let act = act.clone();
        if kind != "bash" {
          // Nothing but a command runs on the disk, so nothing but a command asks where it stands.
          return self.started(&act, "", false);
        }
        self.asking = Some(Asking::Merged(act));
        Reply::Ask { kind: "merged".to_owned(), on, words: vec![Value::Str(fact.about().to_owned())] }
      }
      "stand" => {
        let standing = Value::Tuple(vec![
          self.roster.clone(),
          Value::Str(self.directory.display().to_string()),
          Value::Str(self.actor.clone()),
        ]);
        Reply::say(Fact::new("done", fact.about(), WORLD, vec![standing]))
      }
      "read" => {
        // Where a path resolves is the chain's to say, so it is asked before the disk is touched.
        self.asking = Some(Asking::Read(fact.clone()));
        Reply::ask("cwd", fact.on().unwrap_or_default())
      }
      "write" => {
        self.asking = Some(Asking::Write(fact.clone()));
        Reply::ask("cwd", fact.on().unwrap_or_default())
      }
      "ask" => {
        let (rung, on) = (fact.about().to_owned(), fact.on().unwrap_or_default().to_owned());
        let actor = words.get(1).and_then(Value::as_str).unwrap_or_default().to_owned();
        let turns = words.get(2).cloned().unwrap_or(Value::None);
        self.talks.asked(&rung, &on, &actor, &turns, &self.voice);
        Reply::Nothing
      }
      "feed" => {
        if let Some(one) = self.running.get_mut(fact.about()) {
          one.feed(words.first().and_then(Value::as_str));
        }
        Reply::Nothing
      }
      "cancel" | "close" => self.slew(fact.about()),
      "exited" => {
        self.running.remove(fact.about());
        Reply::Nothing
      }
      "keep" => {
        if let Some(entry) = words.first() {
          self.keep(entry);
        }
        Reply::Nothing
      }
      "clock" => {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0.0, |held| held.as_secs_f64());
        Reply::say(Fact::new("done", fact.about(), WORLD, vec![Value::Float(now)]))
      }
      "chance" => {
        let drew = self.chance();
        Reply::say(Fact::new("done", fact.about(), WORLD, vec![Value::Float(drew)]))
      }
      _ => Reply::Nothing,
    }
  }

  fn answered(&mut self, got: &Value) -> Reply {
    let Some(asking) = self.asking.take() else { return Reply::Nothing };
    let here = got.as_str().unwrap_or_default().to_owned();
    match asking {
      Asking::Read(held) => {
        let path = held.words().get(1).and_then(Value::as_str).unwrap_or_default().to_owned();
        Reply::say(Fact::new("done", held.about(), WORLD, vec![self.read(&here, &path)]))
      }
      Asking::Write(held) => {
        let text = held.words().get(1).cloned().unwrap_or(Value::None);
        Reply::say(Fact::new("done", held.about(), WORLD, vec![self.write(&here, &text)]))
      }
      Asking::Merged(held) => {
        let on = held.on().unwrap_or_default().to_owned();
        self.asking = Some(Asking::Where(held, got == &Value::Bool(true)));
        Reply::ask("cwd", on)
      }
      Asking::Where(held, merged) => self.started(&held, &here, merged),
    }
  }
}

/// One stream of a command, said as it comes, with a part of a character held until the rest of it arrives.
fn told(voice: &Voice, about: &str, stream: &str, mut door: Box<dyn Read + Send>) {
  let mut raw = vec![0u8; PIPE];
  let mut carry: Vec<u8> = Vec::new();
  while let Ok(read) = door.read(&mut raw) {
    if read == 0 {
      break;
    }
    let said = decoded(&mut carry, &raw[..read], false);
    if !said.is_empty() {
      voice.send(Fact::new("out", about, WORLD, vec![Value::Str(said), Value::Str(stream.to_owned())]));
    }
  }
  let said = decoded(&mut carry, &[], true);
  if !said.is_empty() {
    voice.send(Fact::new("out", about, WORLD, vec![Value::Str(said), Value::Str(stream.to_owned())]));
  }
}

/// What a stream said, as text: every character that stands whole, and a mark for a byte that is no character.
///
/// The last character of a part is often cut in two by the door, so what is cut waits in the carry for the part
/// that holds the rest of it, and a life never reads half a character as a broken one.
fn decoded(carry: &mut Vec<u8>, raw: &[u8], last: bool) -> String {
  carry.extend_from_slice(raw);
  let mut said = String::new();
  loop {
    match std::str::from_utf8(carry) {
      Ok(held) => {
        said.push_str(held);
        carry.clear();
        return said;
      }
      Err(why) => {
        let whole = why.valid_up_to();
        said.push_str(std::str::from_utf8(&carry[..whole]).unwrap_or_default());
        match why.error_len() {
          Some(held) => {
            said.push(char::REPLACEMENT_CHARACTER);
            carry.drain(..whole + held);
          }
          None => {
            carry.drain(..whole);
            if last && !carry.is_empty() {
              said.push(char::REPLACEMENT_CHARACTER);
              carry.clear();
            }
            return said;
          }
        }
      }
    }
  }
}

/// Whether one act is another or was made by it, which their lineages say, as the engine reads a name.
fn under(name: &str, of: &str) -> bool {
  let (name, of) = (lineage(name), lineage(of));
  !of.is_empty() && format!("{name}.").starts_with(&format!("{of}."))
}

/// The lineage of an act, the makers of it one under the other, which its name holds after its kind.
fn lineage(name: &str) -> &str {
  name.rsplit_once("://").map_or(name, |(_, held)| held)
}

/// The seconds a word of an act names, which the engine says as a number of either kind.
fn seconds(word: Option<&Value>, whenever: f64) -> f64 {
  match word {
    Some(Value::Float(held)) => *held,
    Some(Value::Int(held)) => *held as f64,
    _ => whenever,
  }
}

/// The command in a session of its own, so that a cancel ends the tree it grew and not its shell alone.
#[cfg(unix)]
fn grouped(made: &mut Command) {
  use std::os::unix::process::CommandExt;

  made.process_group(0);
}

/// On a machine with no sessions, where a command is its shell and the shell alone.
#[cfg(not(unix))]
fn grouped(made: &mut Command) {
  let _ = made;
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::voice::{Ears, Said};

  /// A Talks of the test, which answers nothing and keeps what it was asked.
  #[derive(Default)]
  struct Quiet {
    asked: Vec<String>,
    shown: Vec<String>,
  }

  impl Talks for Quiet {
    fn asked(&mut self, rung: &str, _on: &str, actor: &str, _turns: &Value, _voice: &Voice) {
      self.asked.push(format!("{rung} {actor}"));
    }

    fn shown(&mut self, about: &str, shape: &str, message: &str, voice: &Voice) {
      self.shown.push(format!("{about} {shape} {message}"));
      voice.close(about, Value::Int(3));
    }
  }

  /// A World of this machine under a directory of the test, and the Ears its Voice is heard by.
  fn live(at: &str) -> (Live<Quiet>, Ears) {
    let yard = std::env::temp_dir().join(format!("furb-live-{at}"));
    let _ = fs::remove_dir_all(&yard);
    fs::create_dir_all(&yard).unwrap();
    let (voice, ears) = Ears::made();
    let roster = Value::Tuple(vec![Value::Tuple(vec![
      Value::Str("m".to_owned()),
      Value::Tuple(vec![Value::Str("low".to_owned())]),
      Value::Int(200_000),
    ])]);
    (Live::new(yard, "m/low", roster, Quiet::default(), voice), ears)
  }

  /// One question of the engine, as the World hears it: it is about itself and it stands on a chain.
  fn asks(kind: &str, tail: &str, words: Vec<Value>) -> Fact {
    let mut held = vec![Value::Str("chain://operator.1".to_owned())];
    held.extend(words);
    Fact::new(kind, format!("{kind}://operator.1.{tail}"), "operator", held)
  }

  /// The facts a reply carries, and nothing for a reply that carries none.
  fn says(reply: Reply) -> Vec<Fact> {
    match reply {
      Reply::Say(held) => held,
      other => panic!("the World says facts here, and said {other:?}"),
    }
  }

  /// Everything the World said into its Voice, once what it started has said the fact that is waited for.
  fn heard(ears: &mut Ears, kind: &str) -> Vec<Fact> {
    let waited = Instant::now();
    let mut held = Vec::new();
    while waited.elapsed() < Duration::from_secs(20) {
      held.extend(ears.drained().into_iter().filter_map(|one| match one {
        Said::Fact(fact) => Some(fact),
        _ => None,
      }));
      if held.iter().any(|one| one.kind() == kind) {
        return held;
      }
      thread::sleep(Duration::from_millis(5));
    }
    panic!("the World never said a {kind}, and said {held:?}")
  }

  /// What a command wrote to one of its streams, as the parts of it stand together.
  fn wrote(held: &[Fact], stream: &str) -> String {
    held
      .iter()
      .filter(|one| one.kind() == "out" && one.words()[1] == Value::Str(stream.to_owned()))
      .map(|one| one.words()[0].as_str().unwrap_or_default())
      .collect()
  }

  /// One command, heard and then started, which is the two facts the engine says of every act.
  fn ran(world: &mut Live<Quiet>, command: &str, fed: bool, timeout: f64, merged: bool) -> String {
    let act = asks("bash", "1", vec![Value::Str(command.to_owned()), Value::Bool(fed), Value::Float(timeout)]);
    let about = act.about().to_owned();
    world.hears(&act);
    assert_eq!(
      world.hears(&Fact::new("start", &about, "operator", vec![])),
      Reply::Ask {
        kind: "merged".to_owned(),
        on: "chain://operator.1".to_owned(),
        words: vec![Value::Str(about.clone())],
      }
    );
    assert_eq!(world.answered(&Value::Bool(merged)), Reply::ask("cwd", "chain://operator.1"));
    assert_eq!(world.answered(&Value::Str(String::new())), Reply::Nothing);
    about
  }

  #[test]
  fn a_chain_is_answered_what_it_stands_on() {
    let (mut world, _) = live("stand");
    let held = says(world.hears(&asks("stand", "1", vec![])));
    let standing = held[0].words()[0].as_entries().unwrap().to_vec();
    assert_eq!(standing[2], Value::Str("m/low".to_owned()));
    assert_eq!(standing[0].as_entries().unwrap().len(), 1);
  }

  #[test]
  fn a_read_asks_the_chain_where_its_paths_resolve_and_then_reads() {
    let (mut world, _) = live("read");
    fs::create_dir_all(world.directory.join("w")).unwrap();
    fs::write(world.directory.join("w").join("a.txt"), "one\ntwo\n").unwrap();
    let asked = asks("read", "1", vec![Value::Str("a.txt".to_owned())]);
    assert_eq!(world.hears(&asked), Reply::ask("cwd", "chain://operator.1"));
    let held = says(world.answered(&Value::Str("w".to_owned())));
    assert_eq!(held[0].words()[0].field("content"), Some(&Value::Str("one\ntwo\n".to_owned())));
  }

  #[test]
  fn a_write_lands_on_the_disk_and_gives_the_text_of_the_file_as_it_stands() {
    let (mut world, _) = live("write");
    let asked = asks("write", "1", vec![Value::text("w/a.txt", "one\n")]);
    assert_eq!(world.hears(&asked), Reply::ask("cwd", "chain://operator.1"));
    let held = says(world.answered(&Value::Str(String::new())));
    assert_eq!(held[0].words()[0].field("content"), Some(&Value::Str("one\n".to_owned())));
    assert_eq!(fs::read_to_string(world.directory.join("w").join("a.txt")).unwrap(), "one\n");
  }

  #[test]
  fn a_read_of_no_file_is_refused_and_says_why() {
    let (mut world, _) = live("missing");
    world.hears(&asks("read", "1", vec![Value::Str("none.txt".to_owned())]));
    let held = says(world.answered(&Value::Str(String::new())));
    match &held[0].words()[0] {
      Value::Error { name, args } => {
        assert_eq!(name, "Refused");
        assert!(args[0].as_str().unwrap_or_default().starts_with("no file at"), "{args:?}");
      }
      other => panic!("a read of no file is refused, and gave {other:?}"),
    }
  }

  #[test]
  fn a_command_says_what_it_writes_and_the_code_it_came_to() {
    let (mut world, mut ears) = live("bash");
    let about = ran(&mut world, "echo hi; echo no 1>&2; exit 3", false, 30.0, false);
    let held = heard(&mut ears, "exited");
    // The two streams run at the same time, so the order of one against the other is the machine's to pick.
    assert_eq!(wrote(&held, "stdout"), "hi\n");
    assert_eq!(wrote(&held, "stderr"), "no\n");
    let exited: Vec<&Fact> = held.iter().filter(|one| one.kind() == "exited").collect();
    assert_eq!(exited[0].about(), about);
    assert_eq!(exited[0].words()[0], Value::Int(3));
  }

  #[test]
  fn the_stderr_of_a_merged_command_flows_into_its_stdout() {
    let (mut world, mut ears) = live("merged");
    ran(&mut world, "echo no 1>&2", false, 30.0, true);
    let held = heard(&mut ears, "exited");
    assert_eq!(wrote(&held, "stdout"), "no\n");
    assert_eq!(wrote(&held, "stderr"), "");
  }

  #[test]
  fn the_stdin_of_a_fed_command_takes_what_a_rung_writes_and_a_text_of_nothing_closes_it() {
    let (mut world, mut ears) = live("fed");
    let about = ran(&mut world, "cat", true, 30.0, false);
    world.hears(&Fact::new("feed", &about, "operator", vec![Value::Str("one\n".to_owned())]));
    world.hears(&Fact::new("feed", &about, "operator", vec![Value::None]));
    assert_eq!(wrote(&heard(&mut ears, "exited"), "stdout"), "one\n");
  }

  #[test]
  fn the_world_ends_a_command_at_its_timeout_and_the_code_of_it_is_nothing_then() {
    let (mut world, mut ears) = live("timeout");
    ran(&mut world, "sleep 30", false, 0.2, false);
    let held = heard(&mut ears, "exited");
    assert_eq!(held[0].words()[0], Value::None);
  }

  #[test]
  fn a_cancel_of_the_chain_ends_every_command_it_is_over_and_each_of_them_exits_with_nothing() {
    let (mut world, _) = live("cancel");
    let about = ran(&mut world, "sleep 30", false, 30.0, false);
    let held = says(world.hears(&Fact::new("cancel", "chain://operator.1", "operator", vec![])));
    assert_eq!(held.len(), 1);
    assert_eq!(held[0].kind(), "exited");
    assert_eq!(held[0].about(), about);
    assert_eq!(held[0].words()[0], Value::None);
  }

  #[test]
  fn what_the_journal_keeps_lands_on_the_disk_before_the_world_gives_back() {
    let (world, _) = live("record");
    let at = world.directory.join("record.jsonl");
    let mut world = world.keeping(&at);
    let entry = Value::List(vec![
      Value::Str(String::new()),
      Value::List(vec![
        Value::Str("chain".to_owned()),
        Value::Str("chain://operator.1".to_owned()),
        Value::Str("operator".to_owned()),
        Value::Str(String::new()),
        Value::Str("root".to_owned()),
      ]),
    ]);
    world.hears(&Fact::new("keep", "", "journal", vec![entry]));
    let held = fs::read_to_string(&at).unwrap();
    assert_eq!(held, "[\"\",[\"chain\",\"chain://operator.1\",\"operator\",\"\",\"root\"]]\n");
  }

  #[test]
  fn a_model_and_a_person_are_no_things_of_this_machine_so_a_host_answers_for_them() {
    let (mut world, mut ears) = live("talks");
    let ask = asks("ask", "1", vec![Value::Str("m/low".to_owned()), Value::List(vec![])]);
    assert_eq!(world.hears(&ask), Reply::Nothing);
    assert_eq!(world.talks.asked, vec!["ask://operator.1.1 m/low".to_owned()]);
    let prompt = asks("prompt", "2", vec![Value::Str("int".to_owned()), Value::Str("how many".to_owned())]);
    world.hears(&prompt);
    world.hears(&Fact::new("start", prompt.about(), "operator", vec![]));
    assert_eq!(world.talks.shown, vec!["prompt://operator.1.2 int how many".to_owned()]);
    assert_eq!(ears.drained(), vec![Said::Closed { id: "prompt://operator.1.2".to_owned(), value: Value::Int(3) }]);
  }

  #[test]
  fn a_wait_is_done_with_nothing_once_its_seconds_are_over() {
    let (mut world, mut ears) = live("wait");
    let act = asks("wait", "1", vec![Value::Float(0.01)]);
    world.hears(&act);
    assert_eq!(world.hears(&Fact::new("start", act.about(), "operator", vec![])), Reply::Nothing);
    let held = heard(&mut ears, "done");
    assert_eq!(held[0].about(), act.about());
    assert_eq!(held[0].words()[0], Value::None);
  }

  #[test]
  fn a_part_of_a_character_that_one_read_cuts_in_two_waits_for_the_read_that_holds_the_rest() {
    let mut carry = Vec::new();
    assert_eq!(decoded(&mut carry, "one ".as_bytes(), false), "one ");
    assert_eq!(decoded(&mut carry, &[0xE2, 0x82], false), "");
    assert_eq!(decoded(&mut carry, &[0xAC], false), "\u{20AC}");
    assert_eq!(decoded(&mut carry, &[0xFF], false), "\u{FFFD}");
    assert_eq!(decoded(&mut carry, &[0xE2], true), "\u{FFFD}");
  }
}
