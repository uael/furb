//! The JSON-RPC of furb: one life served to a client on stdin and stdout, as Pi serves its RPC mode.
//!
//! The client writes one command on each line of stdin, a JSON object whose `type` names it. furb writes one record on
//! each line of stdout: the response to each command, which repeats its `id`, and the events of the life, which are
//! each fact it says, each prompt it puts to the operator, and the done of each act that a command made. `docs/rpc.md`
//! says each command and each event. The life ends when stdin ends.

use std::{
  cell::RefCell,
  collections::HashMap,
  future::Future,
  io::{self, BufRead, Write},
  path::{Path, PathBuf},
  pin::Pin,
  rc::Rc,
  sync::{Arc, mpsc},
  task::{Context, Poll, Wake, Waker},
  thread,
};

use furb::{
  Act, Ear, Fact, Fault, Object,
  ear::{ear, hear, say},
  life::Opening,
  verbs, wire,
};
use serde_json::{Value, json, value::RawValue};

use crate::{
  console::Asked,
  life::{self, Life},
};

/// A life served on stdin and stdout, until stdin ends, on the record it keeps when it is given one. It wakes
/// nothing: what an earlier life left paused or pending waits for the wake of the client.
pub fn serve(opening: Opening, record: Option<&Path>) -> Result<(), String> {
  let client = Rc::new(RefCell::new(Client::default()));
  let facts = Rc::clone(&client);
  let life = Life::open(opening, console(Rc::clone(&client)), move |fact: &Fact| {
    let fact = wire::outward(fact.0.as_ref());
    facts.borrow_mut().records.push(json!({"type": "fact", "fact": fact}));
  })?;
  let (sends, heard) = mpsc::channel();
  let reads = sends.clone();
  thread::spawn(move || lines(&reads));
  let mut server = Server {
    life,
    client,
    record: record.map(|one| std::path::absolute(one).unwrap_or_else(|_| one.to_path_buf())),
    awaited: Vec::new(),
    waker: Waker::from(Arc::new(Wakes(sends))),
    out: io::BufWriter::new(io::stdout().lock()),
  };
  server.served(&heard).map_err(|no| no.to_string())
}

/// The client as the server holds it: the records not yet written to it, and each prompt of the operator that waits
/// for its close, in the order they asked.
#[derive(Default)]
struct Client {
  records: Vec<Value>,
  prompts: Vec<Asked>,
}

/// A prompt put to the operator as the client reads it.
fn fields(asked: &Asked) -> Value {
  json!({"act": asked.about, "on": asked.on, "shape": asked.shape, "message": asked.message})
}

/// The console of the client, as an ear of the World: it takes each prompt put to the operator and sends it to the
/// client, whose close answers it.
fn console(client: Rc<RefCell<Client>>) -> Box<dyn Ear> {
  ear(move |co, _| async move {
    loop {
      let a = hear(&co).await;
      if let Some(asked) = Asked::of(&a) {
        say(&co, Fact::says("started", &asked.about, [])).await;
        let mut event = fields(&asked);
        event["type"] = json!("prompt");
        let mut client = client.borrow_mut();
        client.records.push(event);
        client.prompts.push(asked);
      } else if a.kind() == "done" {
        client.borrow_mut().prompts.retain(|one| one.about != a.about());
      }
    }
  })
}

/// What reaches the loop of the server: a line of the client, a voice of the life, or the end of stdin.
enum Input {
  Line(Vec<u8>),
  Wake,
  End,
}

/// The waker of the loop: a voice that speaks from any thread wakes the loop, which drives the life.
struct Wakes(mpsc::Sender<Input>);

impl Wake for Wakes {
  fn wake(self: Arc<Self>) {
    let _ = self.0.send(Input::Wake);
  }
}

/// Each line of stdin, sent as it comes, split at a line feed alone and without a carriage return before it, and then
/// its end.
fn lines(sends: &mpsc::Sender<Input>) {
  let mut input = io::stdin().lock();
  loop {
    let mut line = Vec::new();
    match input.read_until(b'\n', &mut line) {
      Ok(0) | Err(_) => break,
      Ok(_) => {
        if line.ends_with(b"\n") {
          line.pop();
        }
        if line.ends_with(b"\r") {
          line.pop();
        }
        if sends.send(Input::Line(line)).is_err() {
          return;
        }
      }
    }
  }
  let _ = sends.send(Input::End);
}

/// One command of the client: its fields, each as the JSON it was written in.
struct Command(HashMap<String, Box<RawValue>>);

impl Command {
  /// A field that is text, or nothing when it is absent or null.
  fn text(&self, key: &str) -> Result<Option<String>, String> {
    let Some(raw) = self.0.get(key) else { return Ok(None) };
    serde_json::from_str::<Option<String>>(raw.get()).map_err(|_| format!("{key} is no string."))
  }

  /// A field that is text, which the command needs.
  fn needed(&self, key: &str) -> Result<String, String> {
    self.text(key)?.ok_or_else(|| format!("The command needs {key}."))
  }
}

/// The server of one life: the life, the client, the acts its commands made that are not done, the waker of its loop,
/// and stdout.
struct Server {
  life: Life,
  client: Rc<RefCell<Client>>,
  record: Option<PathBuf>,
  awaited: Vec<String>,
  waker: Waker,
  out: io::BufWriter<io::StdoutLock<'static>>,
}

impl Server {
  /// The loop: the life is driven, then the next line or voice is heard, until stdin ends.
  fn served(&mut self, heard: &mpsc::Receiver<Input>) -> io::Result<()> {
    loop {
      self.driven()?;
      match heard.recv() {
        Ok(Input::Line(line)) => self.answer(&line)?,
        Ok(Input::Wake) => {}
        Ok(Input::End) | Err(_) => return Ok(()),
      }
    }
  }

  /// The life driven: what the voices of its ears said is said into it, the done of each act a command made that is
  /// done now is sent, and every record is written.
  fn driven(&mut self) -> io::Result<()> {
    // To await an act drives the life, and the root never completes.
    let mut cx = Context::from_waker(&self.waker);
    let mut root = Act::<Object>::of(&mut self.life.engine, self.life.root.clone());
    if let Poll::Ready(Err(no)) = Pin::new(&mut root).poll(&mut cx) {
      eprintln!("furb: {no}");
    }
    let mut waiting = Vec::new();
    for id in std::mem::take(&mut self.awaited) {
      match self.life.engine.outcome(&id).unwrap_or_else(|no| Some(no.object())) {
        Some(got) => {
          let mut done = json!({"type": "done", "act": id});
          let (key, value) = outcome(&got);
          done[key] = value;
          self.client.borrow_mut().records.push(done);
        }
        None => waiting.push(id),
      }
    }
    self.awaited = waiting;
    let records = std::mem::take(&mut self.client.borrow_mut().records);
    for one in records {
      writeln!(self.out, "{one}")?;
    }
    self.out.flush()
  }

  /// One command, answered: its response is written first, and the events it caused after it.
  fn answer(&mut self, line: &[u8]) -> io::Result<()> {
    if line.iter().all(u8::is_ascii_whitespace) {
      return Ok(());
    }
    let response = match serde_json::from_slice::<HashMap<String, Box<RawValue>>>(line) {
      Err(no) => json!({
        "type": "response", "command": "parse", "success": false,
        "error": format!("Failed to parse command: {no}"),
      }),
      Ok(fields) => {
        let command = Command(fields);
        let kind = command.text("type").ok().flatten().unwrap_or_default();
        let done = self.done(&kind, &command);
        let mut response = json!({"type": "response", "command": kind, "success": done.is_ok()});
        if let Some(id) = command.0.get("id").and_then(|one| serde_json::from_str(one.get()).ok()) {
          response["id"] = id;
        }
        match done {
          Ok(Value::Null) => {}
          Ok(data) => response["data"] = data,
          Err(why) => response["error"] = json!(why),
        }
        response
      }
    };
    writeln!(self.out, "{response}")?;
    self.out.flush()
  }

  /// What a command did, as the data of its response, or why it did nothing.
  fn done(&mut self, kind: &str, command: &Command) -> Result<Value, String> {
    let failed = |no: Fault| no.to_string();
    let on = command.text("on")?.unwrap_or_else(|| self.life.root.clone());
    match kind {
      "prompt" => {
        let shape = life::shape(&command.text("shape")?.unwrap_or_default());
        let with = verbs::Prompt {
          message: command.text("message")?,
          to: command.text("to")?.map(|to| life::actor(&to)),
          on: Some(on),
        };
        let id = self.life.engine.prompt(shape, with).map_err(failed)?.id().to_owned();
        Ok(self.made(id))
      }
      "rung" => {
        let with =
          verbs::Rung { word: Some(command.needed("word")?), on: Some(on), ..Default::default() };
        let id = self.life.engine.rung(with).map_err(failed)?.id().to_owned();
        Ok(self.made(id))
      }
      "close" => {
        let act = command.needed("act")?;
        let value = self.closing(&act, command)?;
        self.life.engine.close(value, verbs::Close { id: Some(act) }).map_err(failed)?;
        Ok(Value::Null)
      }
      "cancel" | "pause" | "wake" => {
        let (act, engine) = (command.needed("act")?, &mut self.life.engine);
        let done = match kind {
          "cancel" => engine.cancel(&act),
          "pause" => engine.pause(&act),
          _ => engine.wake(&act),
        };
        done.map_err(failed).map(|()| Value::Null)
      }
      "turns" => {
        let turns = self.life.engine.turns(verbs::Turns { on: Some(on) }).map_err(failed)?;
        let turns: Vec<Value> = turns.iter().map(|one| wire::outward(one.as_ref())).collect();
        Ok(json!({"turns": turns}))
      }
      "transcript" => {
        let facts = self.life.engine.transcript(verbs::Transcript { on: Some(on) });
        let facts: Vec<Value> =
          facts.map_err(failed)?.iter().map(|one| wire::outward(one.0.as_ref())).collect();
        Ok(json!({"facts": facts}))
      }
      "peek" => {
        let act = command.needed("act")?;
        if self.life.engine.get(&act).map_err(failed)?.is_none() {
          return Err(format!("No act is named {act}."));
        }
        let got = self.life.engine.outcome(&act).map_err(failed)?;
        let mut peeked = json!({"done": got.is_some()});
        if let Some(got) = got {
          let (key, value) = outcome(&got);
          peeked[key] = value;
        }
        Ok(peeked)
      }
      "state" => {
        let standing = self.life.engine.standing().map_err(failed)?;
        let prompts: Vec<Value> = self.client.borrow().prompts.iter().map(fields).collect();
        let extensions: Vec<String> =
          self.life.extensions()?.into_iter().map(|one| one.name).collect();
        Ok(json!({
          "root": self.life.root,
          "record": self.record.as_ref().map(|one| one.display().to_string()),
          "standing": wire::outward(standing.as_ref()),
          "extensions": extensions,
          "paused": self.life.paused(),
          "prompts": prompts,
          "acts": self.awaited,
          "pending": self.life.pending(),
        }))
      }
      "" => Err("The command needs type.".to_owned()),
      _ => Err(format!("Unknown command: {kind}")),
    }
  }

  /// An act a command made, whose done the client gets when it is done, and the data of the response that names it.
  fn made(&mut self, id: String) -> Value {
    let data = json!({"act": id});
    self.awaited.push(id);
    data
  }

  /// The value a close carries, with every number exact. The operator answers a float with any number, so a whole
  /// number that closes a prompt of a float is that float.
  fn closing(&self, act: &str, command: &Command) -> Result<Object, String> {
    let raw = command.0.get("value").map_or("null", |one| one.get());
    let value = wire::parsed(raw).map_err(|no| no.to_string())?;
    let float =
      self.client.borrow().prompts.iter().any(|one| one.about == act && one.shape == "float");
    Ok(match value.as_ref().as_int() {
      Some(n) if float => Object::float(n as f64),
      _ => value,
    })
  }
}

/// What an act came to, as a response or an event says it: its value, or the exception it raised.
fn outcome(got: &Object) -> (&'static str, Value) {
  let key = if Fault::of(got.as_ref()).is_some() { "raised" } else { "value" };
  (key, wire::outward(got.as_ref()))
}
