//! One life of the engine, as the operator holds it: the World of this machine it stands on, the record it keeps,
//! and what the operator hears of it.

use std::{
  cell::RefCell,
  future::Future,
  path::{Path, PathBuf},
  pin::Pin,
  rc::Rc,
  sync::Arc,
  task::{Context, Poll, Wake, Waker},
  thread,
};

use furb::{
  Act, Ear, Engine, Fact, Fault, Object,
  ear::{ear, hear},
  verbs,
  world::{self, claude::Claude},
};

use crate::console;

/// The actor a prompt goes to when it names none, which is opus at the least effort it takes.
pub const ACTOR: &str = "opus/low";

/// One life, as the operator holds it: its engine, its root, the record it was made again from, and what the
/// operator hears of its root.
pub struct Life {
  pub engine: Engine,
  pub root: String,
  held: Vec<Object>,
  quiet: Rc<RefCell<Quiet>>,
}

/// Whether a pause stands over the acts the operator watches, the root and the act it awaits, and the last refusal
/// of a reply, which says why the World paused a chain. A control is over the act it names and over every act on the
/// chain it names, and an act of the operator has no maker above it, so the controls about these acts are all the
/// controls over them.
#[derive(Default)]
struct Quiet {
  acts: Vec<String>,
  paused: bool,
  refused: String,
}

impl Quiet {
  /// The acts it watches, and what the facts said so far say of them, from the first.
  fn watch(&mut self, acts: Vec<String>, facts: &[Fact]) {
    (self.acts, self.paused) = (acts, false);
    for one in facts {
      self.heard(one);
    }
  }

  /// What one more fact says.
  fn heard(&mut self, a: &Fact) {
    let watched = self.acts.iter().any(|one| one == a.about());
    let reply = a.about().strip_prefix("reply").is_some_and(|n| n.parse::<u64>().is_ok());
    match a.kind() {
      "pause" | "wake" if watched => self.paused = a.kind() == "pause",
      "done" if reply => {
        if let Some(no) = a.word(0).and_then(Fault::of) {
          self.refused = no.message();
        }
      }
      _ => {}
    }
  }
}

impl Life {
  /// A life on the World of this machine and on a console of the operator: the provider of the models of the claude
  /// command line, the files, the commands and time, and the store of its record when it keeps one. Each ear is
  /// heard under the name the TUI hears it by, so the record of one opens in the other.
  ///
  /// The journal says the whole record again before boot returns, so the life stands whole on its record here, and a
  /// life whose record drifted is refused, since it would keep nothing more. `heard` is given every fact said after
  /// that, as an ear of the engine hears it.
  pub fn open(
    record: Option<&Path>,
    keeps: bool,
    cwd: &Path,
    console: Box<dyn Ear>,
    mut heard: impl FnMut(&Fact) + 'static,
  ) -> Result<Life, String> {
    let failed = |no: Fault| no.to_string();
    let (held, store) = match record {
      Some(path) if keeps => {
        let (held, store) = world::store(path).map_err(failed)?;
        (held, Some(store))
      }
      Some(path) => (world::kept(path).map_err(failed)?, None),
      None => (Vec::new(), None),
    };
    let models = Claude::new().models();
    let provider = world::provider(cwd.display().to_string(), models, Some(ACTOR.to_owned()));
    let mut ears = vec![
      ("provider", provider),
      ("console", console),
      ("files", world::files()),
      ("bash", world::bash()),
      ("time", world::time()),
    ];
    ears.extend(store.map(|store| ("store", store)));
    let mut engine = Engine::boot(held.clone(), ears).map_err(failed)?;
    if let Some(no) = engine.raised() {
      return Err(no.to_string());
    }
    let root = engine.root().to_owned();
    let mut quiet = Quiet::default();
    let facts = engine.transcript(verbs::Transcript { on: Some(root.clone()) }).map_err(failed)?;
    quiet.watch(vec![root.clone()], &facts);
    let quiet = Rc::new(RefCell::new(quiet));
    let quieted = Rc::clone(&quiet);
    let observer = ear(move |co, _| async move {
      loop {
        let a = hear(&co).await;
        quieted.borrow_mut().heard(&a);
        heard(&a);
      }
    });
    engine.drive(observer, "observer").map_err(failed)?;
    Ok(Life { engine, root, held, quiet })
  }

  /// A life for one command of the operator, on the terminal. It keeps what it says to its record when it keeps, and
  /// a life that only reads its record keeps nothing, since every life stands as it opens and a life that keeps
  /// keeps that stand.
  pub fn lived(record: Option<&Path>, cwd: Option<&Path>, keeps: bool) -> Result<Life, String> {
    Life::open(record, keeps, &directory(cwd), console::terminal(), |_| {})
  }

  /// The name of the prompt the record already holds for this message: of the operator, on the root, of this shape
  /// and to this actor; and nothing when it holds none.
  ///
  /// The engine matches nothing the operator says again, so a life stood up on its own record would open a second
  /// prompt beside the one that record stands on, and ask a model for what it was answered once.
  pub fn again(&self, shape: &str, message: &str, to: &str) -> Option<String> {
    let wanted = ["operator", self.root.as_str(), shape, message, to];
    self.held.iter().find_map(|entry| {
      let fact = entry.as_ref().items()?.first()?.items()?;
      let words = fact.iter().map(|one| one.as_str()).collect::<Option<Vec<&str>>>()?;
      (words.len() == 7 && words[0] == "prompt" && words[2..] == wanted)
        .then(|| words[1].to_owned())
    })
  }

  /// What an act of the operator on the root came to, awaited until it is done, as the operator waits on its own
  /// loop; or why it never will be: a pause over the act or over the root, which no one wakes while one command
  /// runs.
  pub fn settled(&mut self, id: &str) -> Result<Object, String> {
    let facts = self.engine.transcript(verbs::Transcript { on: Some(self.root.clone()) });
    let facts = facts.map_err(|no| no.to_string())?;
    self.quiet.borrow_mut().watch(vec![self.root.clone(), id.to_owned()], &facts);
    let waker = Waker::from(Arc::new(Parked(thread::current())));
    let mut cx = Context::from_waker(&waker);
    loop {
      if let Poll::Ready(got) = Pin::new(&mut Act::<Object>::of(&mut self.engine, id)).poll(&mut cx)
      {
        return got.map_err(|no| no.to_string());
      }
      let quiet = self.quiet.borrow();
      if quiet.paused {
        let why =
          if quiet.refused.is_empty() { String::new() } else { format!(": {}", quiet.refused) };
        return Err(format!("{id} is paused{why}. A wake from the TUI makes it go on."));
      }
      drop(quiet);
      thread::park();
    }
  }
}

/// The directory a life stands on: the one given, or the current one, as an absolute path.
pub fn directory(cwd: Option<&Path>) -> PathBuf {
  let cwd = cwd.unwrap_or(Path::new("."));
  std::path::absolute(cwd).unwrap_or_else(|_| cwd.to_path_buf())
}

/// A waker that unparks the thread that awaits, so a voice that speaks from another thread wakes it.
struct Parked(thread::Thread);

impl Wake for Parked {
  fn wake(self: Arc<Self>) {
    self.0.unpark();
  }
}
