//! The console of the operator at a terminal: an ear of the World that takes each prompt put to the operator, shows it
//! on stderr, and closes it with the line the operator writes back on stdin, read as the shape the prompt wants.
//!
//! There is one terminal and one operator, so the prompts are shown and answered one at a time, in the order they
//! asked, and never two at once on one stream. Stdout is left to what the command prints.

use std::{
  collections::HashSet,
  io::{self, BufRead, Write},
  sync::{Arc, Mutex, mpsc},
  thread,
};

use furb::{
  Ear, Fact, Fault, Object, Voice,
  ear::{call, ear, hear, say},
  world::{SHAPES, answered},
};

/// A prompt put to the operator: its act, the chain it is on, its shape and its message.
pub struct Asked {
  pub about: String,
  pub on: String,
  pub shape: String,
  pub message: String,
}

impl Asked {
  /// The prompt a fact asks the operator, when it is one that no ear before the console took.
  pub fn of(a: &Fact) -> Option<Asked> {
    let word = |at: usize| a.word(at).and_then(|one| one.as_str().map(str::to_owned));
    (a.kind() == "prompt" && a.question()).then(|| Asked {
      about: a.about().to_owned(),
      on: a.on().to_owned(),
      shape: word(1).unwrap_or_default(),
      message: word(2).unwrap_or_default(),
    })
  }
}

/// The console, as an ear: it takes a prompt to the operator, and its work closes it later by its voice. A prompt that
/// is done before its line comes, by a cancel, is shown no more, and its work says nothing of it.
pub fn terminal() -> Box<dyn Ear> {
  ear(|co, voice| async move {
    let (asks, asked) = mpsc::channel::<Asked>();
    let over: Arc<Mutex<HashSet<String>>> = Arc::default();
    let (shown, speaks) = (Arc::clone(&over), voice.clone());
    thread::spawn(move || shows(&asked, &shown, &speaks));
    let mut taken = HashSet::new();
    loop {
      let a = hear(&co).await;
      if let Some(asked) = Asked::of(&a) {
        say(&co, Fact::says("started", &asked.about, [])).await;
        // A prompt of a shape that the operator answers not is shown not, and closed at once with what no line comes
        // to, which is the refusal of its shape.
        if !SHAPES.contains(&asked.shape.as_str()) {
          let no = answered(&asked.shape, "").unwrap_or_else(|no| no.object());
          call(&co, "close", vec![no], vec![("id", Object::string(&asked.about))]).await?;
          continue;
        }
        taken.insert(asked.about.clone());
        let _ = asks.send(asked);
      } else if a.kind() == "done" && taken.remove(a.about()) {
        voice.hush(a.about());
        if let Ok(mut over) = over.lock() {
          over.insert(a.about().to_owned());
        }
      }
    }
  })
}

/// Each prompt the console took, shown in turn, and closed with what the operator answered, or with why no answer
/// came.
fn shows(asked: &mpsc::Receiver<Asked>, over: &Mutex<HashSet<String>>, voice: &Voice) {
  let done = |about: &str| over.lock().is_ok_and(|over| over.contains(about));
  for Asked { about, shape, message, .. } in asked {
    if done(&about) {
      continue;
    }
    eprint!("{about} wants a {shape}: {message}\n> ");
    let _ = io::stderr().flush();
    let mut line = String::new();
    let value = match io::stdin().lock().read_line(&mut line) {
      Ok(0) => Err(Fault::refused("the operator cannot be read: the input is over")),
      Ok(_) => answered(&shape, line.trim_end_matches(['\n', '\r'])),
      Err(no) => Err(Fault::refused(format!("the operator cannot be read: {no}"))),
    };
    if !done(&about) {
      let value = value.unwrap_or_else(|no| no.object());
      voice.call(&about, "close", vec![value], vec![("id", Object::string(&about))]);
    }
  }
}
