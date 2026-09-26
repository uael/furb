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
  wire,
};

/// Every shape the operator answers, by its name: a list and a dict as a line of JSON.
const SHAPES: [&str; 7] = ["None", "str", "int", "float", "bool", "list", "dict"];

/// One prompt the console took: its act, its shape and its message.
struct Asked {
  about: String,
  shape: String,
  message: String,
}

/// The console, as an ear: it takes a prompt to the operator, and its work closes it later by its voice.
///
/// It closes at once with a refusal a prompt whose shape the operator does not answer, which is the law of the World.
/// A prompt that is done before its line comes, by a cancel, is shown no more, and its work says nothing of it.
pub fn terminal() -> Box<dyn Ear> {
  ear(|co, voice| async move {
    let (asks, asked) = mpsc::channel::<Asked>();
    let over: Arc<Mutex<HashSet<String>>> = Arc::default();
    let (shown, speaks) = (Arc::clone(&over), voice.clone());
    thread::spawn(move || shows(&asked, &shown, &speaks));
    let mut taken = HashSet::new();
    loop {
      let a = hear(&co).await;
      let about = a.about().to_owned();
      match a.kind() {
        "prompt" if a.question() => {
          say(&co, Fact::says("started", &about, [])).await;
          let word = |at: usize| a.word(at).and_then(|one| one.as_str().map(str::to_owned));
          let (shape, message) = (word(1).unwrap_or_default(), word(2).unwrap_or_default());
          if !SHAPES.contains(&shape.as_str()) {
            let no = Fault::refused(format!("the operator answers no {shape}"));
            call(&co, "close", vec![no.object()], vec![("id", Object::string(&about))]).await?;
            continue;
          }
          taken.insert(about.clone());
          let _ = asks.send(Asked { about, shape, message });
        }
        "done" if taken.remove(&about) => {
          voice.hush(&about);
          if let Ok(mut over) = over.lock() {
            over.insert(about);
          }
        }
        _ => {}
      }
    }
  })
}

/// Each prompt the console took, shown in turn, and closed with what the operator answered, or with why no answer
/// came.
fn shows(asked: &mpsc::Receiver<Asked>, over: &Mutex<HashSet<String>>, voice: &Voice) {
  let done = |about: &str| over.lock().is_ok_and(|over| over.contains(about));
  for Asked { about, shape, message } in asked {
    if done(&about) {
      continue;
    }
    eprint!("{about} wants a {shape}: {message}\n> ");
    let _ = io::stderr().flush();
    let mut line = String::new();
    let value = match io::stdin().lock().read_line(&mut line) {
      Ok(0) => Err(Fault::refused("the operator cannot be read: the input is over")),
      Ok(_) => answered(&shape, line.trim()),
      Err(no) => Err(Fault::refused(format!("the operator cannot be read: {no}"))),
    };
    if !done(&about) {
      let value = value.unwrap_or_else(|no| no.object());
      voice.call(&about, "close", vec![value], vec![("id", Object::string(&about))]);
    }
  }
}

/// A line of the operator as a value of the shape the prompt wants, or why it is none.
fn answered(shape: &str, line: &str) -> Result<Object, Fault> {
  let no = || Fault::refused(format!("{line:?} is no {shape}"));
  match shape {
    "None" => Ok(Object::none()),
    "str" => Ok(Object::string(line)),
    "int" => line.parse::<i64>().map(Object::int).map_err(|_| no()),
    "float" => line.parse::<f64>().map(Object::float).map_err(|_| no()),
    "bool" => match line.to_lowercase().as_str() {
      "y" | "yes" | "true" | "1" => Ok(Object::bool(true)),
      "n" | "no" | "false" | "0" => Ok(Object::bool(false)),
      _ => Err(Fault::refused(format!("{line:?} is neither yes nor no"))),
    },
    _ => {
      let value = wire::parsed(line).map_err(|_| no())?;
      if value.as_ref().type_name() == shape { Ok(value) } else { Err(no()) }
    }
  }
}
