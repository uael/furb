//! The daemon of a machine, as its windows reach it on the local transport: the holders of a life,
//! the grace after the last of them, a fault of one life, and the handover to a new build.

#![expect(
  clippy::disallowed_methods,
  reason = "the tests block on the runtime that tokio starts for them"
)]

#[cfg(test)]
mod daemon {
  use std::{
    collections::VecDeque,
    fs,
    path::{Path, PathBuf},
    sync::{
      Arc,
      atomic::{AtomicU32, AtomicUsize, Ordering},
    },
    time::Duration,
  };

  use furb::{
    Object,
    life::{Answer as Answered, Opening},
  };
  use furb_host::{
    DaemonError, Event, GRACE, ProtocolError, Settings,
    protocol::{Answer, Frame, Hello, Kind, Listed, PROTOCOL, Request, Verb},
    run,
    transport::{Dialer, Local},
    upgrade,
    view::Phase,
  };
  use futures::{SinkExt, StreamExt};
  use serde_json::json;
  use tokio::{task::JoinHandle, time::timeout};
  use tokio_util::codec::{Framed, LinesCodec};

  /// How long a test waits for a frame before it fails.
  const PATIENCE: Duration = Duration::from_secs(60);

  /// A grace short enough for a test, and the wait that passes it with room.
  const SHORT: Duration = Duration::from_millis(300);
  const PAST: Duration = Duration::from_millis(1500);

  /// A folder of one test, empty, and short enough for the path of a socket.
  fn place() -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let at = std::env::temp_dir().join(format!("fd{}-{n}", std::process::id()));
    fs::remove_dir_all(&at).ok();
    fs::create_dir_all(&at).unwrap();
    at
  }

  /// The opening of each life of a daemon of a test: no extension, and a model that closes each
  /// prompt with None.
  fn opening(at: &Path) -> Opening {
    let asked = AtomicUsize::new(0);
    let answer = Answered(Arc::new(move |_, _| {
      asked.fetch_add(1, Ordering::SeqCst);
      let turn = Object::tuple([
        Object::string("assistant"),
        Object::string("close(None)"),
        Object::none(),
        Object::none(),
      ]);
      Box::pin(async move { Ok(turn) })
    }));
    Opening {
      config: Some(at.join("config").display().to_string()),
      roster: Some(vec!["claude-cli:opus".to_owned()]),
      extensions: Some(false),
      answer: Some(answer),
      ..Opening::default()
    }
  }

  /// A daemon that listens in a folder, served on a task of the test.
  async fn started(at: &Path, grace: Duration) -> (Local, JoinHandle<Result<(), DaemonError>>) {
    let local = Local::new(at.join("sock"));
    let listener = local.listen().await.unwrap();
    let settings = Settings { opening: opening(at), grace };
    let served = local.clone();
    (local, tokio::spawn(async move { run(&served, listener, settings).await }))
  }

  /// A window of a test: its stream, the id of its next request, and each frame that it read while
  /// it waited for another.
  struct Window {
    lines: Framed<<Local as Dialer>::Stream, LinesCodec>,
    next: u64,
    pushed: VecDeque<Frame>,
  }

  impl Window {
    /// A window that dialed the daemon and said its hello.
    async fn dialed(local: &Local) -> Window {
      let lines = Framed::new(local.dial().await.unwrap(), LinesCodec::new());
      let mut window = Window { lines, next: 0, pushed: VecDeque::new() };
      let hello = Verb::Hello(Hello { protocol: PROTOCOL, kind: Kind::Window, name: None });
      let welcome = window.ask(hello).await;
      assert!(matches!(welcome, Ok(Answer::Welcome { .. })), "{welcome:?}");
      window
    }

    /// The next frame of the daemon, or none at the end of the stream.
    async fn read(&mut self) -> Option<Frame> {
      let line = timeout(PATIENCE, self.lines.next()).await.expect("the daemon says a frame");
      line.map(|line| serde_json::from_str(&line.unwrap()).unwrap())
    }

    /// The answer of the daemon to a verb.
    async fn ask(&mut self, verb: Verb) -> Result<Answer, ProtocolError> {
      let id = self.next;
      self.next += 1;
      self.lines.send(serde_json::to_string(&Request { id, verb }).unwrap()).await.unwrap();
      loop {
        match self.read().await.expect("the daemon answers each request") {
          Frame::Reply { id: got, answer } if got == id => return Ok(answer),
          Frame::Refused { id: Some(got), error, .. } if got == id => return Err(error),
          frame => self.pushed.push_back(frame),
        }
      }
    }

    /// The first frame that the daemon pushed and that the test waits for.
    async fn until(&mut self, wanted: impl Fn(&Frame) -> bool) -> Frame {
      if let Some(at) = self.pushed.iter().position(&wanted) {
        return self.pushed.remove(at).unwrap();
      }
      loop {
        let frame = self.read().await.expect("the daemon pushes the frame");
        if wanted(&frame) {
          return frame;
        }
      }
    }

    /// Whether the daemon closed the stream of the window.
    async fn closed(&mut self) -> bool {
      while self.read().await.is_some() {}
      true
    }

    /// A word run on the root of a life, and its rung once it is done.
    async fn ran(&mut self, life: &str, word: &str) -> String {
      let verb = Verb::Run { life: life.to_owned(), word: word.to_owned(), on: None };
      let Ok(Answer::Act(rung)) = self.ask(verb).await else { panic!("no rung") };
      self.until(done(&rung)).await;
      rung
    }

    async fn attach(&mut self, life: &str) {
      assert_eq!(self.ask(Verb::Attach { life: life.to_owned(), from: 0 }).await, Ok(Answer::Done));
    }

    async fn listed(&mut self) -> Vec<Listed> {
      let Ok(Answer::Lives(lives)) = self.ask(Verb::List).await else { panic!("no list") };
      lives
    }

    /// Whether a life lives in the daemon, held or in its grace.
    async fn lives(&mut self, life: &str) -> bool {
      self.listed().await.iter().any(|one| one.life == life && one.open)
    }
  }

  /// Whether a frame is the act of a name, done.
  fn done(id: &str) -> impl Fn(&Frame) -> bool + '_ {
    move |frame| {
      matches!(frame, Frame::Event { event: Event::Act(act), .. }
        if act.id == id && matches!(act.phase, Phase::Done(_)))
    }
  }

  /// Whether a frame says that a count of windows is attached.
  fn present(count: usize) -> impl Fn(&Frame) -> bool {
    move |frame| matches!(frame, Frame::Presence { peers, .. } if peers.len() == count)
  }

  /// A life opened on a new record in a folder.
  async fn opened(window: &mut Window, at: &Path, name: &str) -> String {
    let life = at.join(name).display().to_string();
    let verb = Verb::Open { life: life.clone(), cwd: Some(at.display().to_string()) };
    assert_eq!(window.ask(verb).await, Ok(Answer::Done));
    life
  }

  #[tokio::test]
  async fn two_windows_on_one_life_both_stay_and_hear_it() {
    let at = place();
    let (local, _daemon) = started(&at, GRACE).await;
    let (mut first, mut second) = (Window::dialed(&local).await, Window::dialed(&local).await);
    let life = opened(&mut first, &at, "a.jsonl").await;
    first.attach(&life).await;
    second.attach(&life).await;
    first.until(present(2)).await;
    let rung = second.ran(&life, "x = 1").await;
    first.until(done(&rung)).await;
    let rung = first.ran(&life, "y = 2").await;
    second.until(done(&rung)).await;
    let Ok(Answer::Presence(peers)) = first.ask(Verb::Presence { life }).await else { panic!() };
    assert_eq!(peers.len(), 2);
  }

  #[tokio::test]
  async fn the_detach_of_one_window_leaves_the_other() {
    let at = place();
    let (local, _daemon) = started(&at, SHORT).await;
    let (mut first, mut second) = (Window::dialed(&local).await, Window::dialed(&local).await);
    let life = opened(&mut first, &at, "a.jsonl").await;
    first.attach(&life).await;
    second.attach(&life).await;
    assert_eq!(first.ask(Verb::Detach { life: life.clone() }).await, Ok(Answer::Done));
    second.until(present(1)).await;
    tokio::time::sleep(PAST).await;
    assert!(second.lives(&life).await);
    second.ran(&life, "x = 1").await;
    let refused = first.ask(Verb::Detach { life: life.clone() }).await;
    assert_eq!(refused, Err(ProtocolError::Unattached { life }));
  }

  #[tokio::test]
  async fn a_life_ends_after_the_grace_of_its_last_holder_and_not_while_a_pin_holds_it() {
    let at = place();
    let (local, _daemon) = started(&at, SHORT).await;
    let mut window = Window::dialed(&local).await;
    let life = opened(&mut window, &at, "a.jsonl").await;
    let pin = || Verb::Pin { life: life.clone(), holder: "job".to_owned() };
    assert_eq!(window.ask(pin()).await, Ok(Answer::Done));
    tokio::time::sleep(PAST).await;
    assert!(window.lives(&life).await);
    let unpin = Verb::Unpin { life: life.clone(), holder: "job".to_owned() };
    assert_eq!(window.ask(unpin).await, Ok(Answer::Done));
    tokio::time::sleep(PAST).await;
    assert!(!window.lives(&life).await);
    let refused = window.ask(pin()).await;
    assert!(matches!(refused, Err(ProtocolError::Closed { .. })), "{refused:?}");
  }

  #[tokio::test]
  async fn a_life_that_ended_opens_again_from_its_checkpoint() {
    let at = place();
    let (local, _daemon) = started(&at, SHORT).await;
    let mut window = Window::dialed(&local).await;
    let life = opened(&mut window, &at, "a.jsonl").await;
    window.attach(&life).await;
    window.ran(&life, "x = 41 + 1").await;
    assert_eq!(window.ask(Verb::Detach { life: life.clone() }).await, Ok(Answer::Done));
    tokio::time::sleep(PAST).await;
    assert!(!window.lives(&life).await);
    opened(&mut window, &at, "a.jsonl").await;
    let inspect = Verb::Inspect { life, name: "x".to_owned(), on: None };
    let Ok(Answer::Inspected { value, .. }) = window.ask(inspect).await else { panic!() };
    assert_eq!(value, json!(42));
  }

  #[tokio::test]
  async fn a_fault_of_one_life_leaves_the_daemon_and_the_other_lives() {
    let at = place();
    let (local, daemon) = started(&at, GRACE).await;
    let mut window = Window::dialed(&local).await;
    let good = opened(&mut window, &at, "a.jsonl").await;
    fs::create_dir_all(at.join("bad.jsonl")).unwrap();
    let bad = opened(&mut window, &at, "bad.jsonl").await;
    match window.ask(Verb::Attach { life: bad.clone(), from: 0 }).await {
      Ok(Answer::Done) => {
        let ended = |frame: &Frame| matches!(frame, Frame::Event { life, event: Event::Ended(Some(_)), .. } if *life == bad);
        window.until(ended).await;
      }
      refused => assert_eq!(refused, Err(ProtocolError::Closed { life: bad.clone() })),
    }
    window.attach(&good).await;
    window.ran(&good, "x = 1").await;
    assert!(window.lives(&good).await);
    assert!(!daemon.is_finished());
  }

  #[tokio::test]
  async fn a_handover_keeps_each_life_its_pins_and_each_window() {
    let at = place();
    let (local, old) = started(&at, SHORT).await;
    let mut window = Window::dialed(&local).await;
    let life = opened(&mut window, &at, "a.jsonl").await;
    window.attach(&life).await;
    let pin = Verb::Pin { life: life.clone(), holder: "job".to_owned() };
    assert_eq!(window.ask(pin).await, Ok(Answer::Done));
    window.ran(&life, "x = 41 + 1").await;
    let settings = Settings { opening: opening(&at), grace: SHORT };
    let next = local.clone();
    let new = tokio::spawn(async move { upgrade(&next, settings).await });
    assert!(window.closed().await);
    assert!(matches!(old.await, Ok(Ok(()))));
    let mut window = Window::dialed(&local).await;
    window.attach(&life).await;
    let inspect = Verb::Inspect { life: life.clone(), name: "x".to_owned(), on: None };
    let Ok(Answer::Inspected { value, .. }) = window.ask(inspect).await else { panic!() };
    assert_eq!(value, json!(42));
    window.ran(&life, "y = x").await;
    assert_eq!(window.ask(Verb::Detach { life: life.clone() }).await, Ok(Answer::Done));
    tokio::time::sleep(PAST).await;
    assert!(window.lives(&life).await);
    assert!(!new.is_finished());
  }

  #[tokio::test]
  async fn a_second_daemon_of_the_machine_refuses_to_start() {
    let at = place();
    let (local, _daemon) = started(&at, GRACE).await;
    let refused = local.listen().await;
    assert!(matches!(refused, Err(furb_host::TransportError::Taken { .. })));
  }
}
