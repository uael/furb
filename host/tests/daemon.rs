//! The daemon of a machine, as its windows reach it on the local transport: the holders of a life,
//! the grace after the last of them, a fault of one life, and the handover to a new build.

#![expect(
  clippy::disallowed_methods,
  reason = "the tests block on the runtime that tokio starts for them"
)]

#[cfg(test)]
mod daemon {
  use std::{
    collections::{HashSet, VecDeque},
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
    DaemonError, Event, GRACE, ProtocolError, Settings, TransportError,
    protocol::{
      Answer, Axis, Dispose, Effect, Frame, Gesture, Hello, Kind, Listed, Mount, Node, PROTOCOL,
      Request, Role, Shape, Slot, Tree, Verb, ViewEffect, ViewEvent,
    },
    run,
    transport::{Accepted, Dialer, Identity, Local, Who},
    upgrade,
    view::Phase,
  };
  use futures::{SinkExt, StreamExt};
  use serde_json::Value;
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

  /// The identity of a test: the peers that dial in turn hold the roles of its list, so that a
  /// window can read.
  #[derive(Clone)]
  struct Grants {
    local: Local,
    next: Arc<AtomicUsize>,
    roles: Arc<[Role]>,
  }

  impl Identity for Grants {
    type Stream = Accepted;

    fn identify(&self, stream: &Accepted) -> Result<Who, TransportError> {
      let who = self.local.identify(stream)?;
      let at = self.next.fetch_add(1, Ordering::SeqCst);
      Ok(Who { role: self.roles[at], ..who })
    }
  }

  /// A daemon that gives each peer that dials, in turn, the role of a list.
  async fn started_as(at: &Path, roles: &[Role]) -> Local {
    let local = Local::new(at.join("sock"));
    let listener = local.listen().await.unwrap();
    let settings = Settings { opening: opening(at), grace: GRACE };
    let identity = Grants { local: local.clone(), next: Arc::default(), roles: roles.into() };
    tokio::spawn(async move { run(&identity, listener, settings).await });
    local
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
      Window::dialed_as(local, Kind::Window, None).await
    }

    /// A peer of a kind that dialed the daemon and said its hello.
    async fn dialed_as(local: &Local, kind: Kind, name: Option<&str>) -> Window {
      let lines = Framed::new(local.dial().await.unwrap(), LinesCodec::new());
      let mut window = Window { lines, next: 0, pushed: VecDeque::new() };
      let name = name.map(str::to_owned);
      let hello = Verb::Hello(Hello { protocol: PROTOCOL, kind, name });
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
    let cwd = Some(at.display().to_string());
    let verb = Verb::Open {
      life: life.clone(),
      cwd,
      actor: None,
      roster: None,
      extensions: None,
      config: None,
      claude: None,
      answers: false,
      attach: None,
    };
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
  async fn a_life_that_its_open_attaches_has_a_holder_before_a_grace_of_none_passes() {
    let at = place();
    let (local, _daemon) = started(&at, Duration::ZERO).await;
    let mut window = Window::dialed(&local).await;
    let life = at.join("a.jsonl").display().to_string();
    let open = Verb::Open {
      life: life.clone(),
      cwd: Some(at.display().to_string()),
      actor: None,
      roster: None,
      extensions: None,
      config: None,
      claude: None,
      answers: false,
      attach: Some(0),
    };
    assert_eq!(window.ask(open).await, Ok(Answer::Done));
    window.until(present(1)).await;
    tokio::time::sleep(PAST).await;
    assert!(window.lives(&life).await);
    window.ran(&life, "x = 1").await;
    assert_eq!(window.ask(Verb::Detach { life: life.clone() }).await, Ok(Answer::Done));
    tokio::time::sleep(PAST).await;
    assert!(!window.lives(&life).await);
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
  async fn a_window_of_a_life_born_from_its_checkpoint_hears_each_fact_of_the_life_once() {
    let at = place();
    let (local, _daemon) = started(&at, SHORT).await;
    let mut window = Window::dialed(&local).await;
    let life = opened(&mut window, &at, "a.jsonl").await;
    window.attach(&life).await;
    let first = window.ran(&life, "x = 41 + 1").await;
    assert_eq!(window.ask(Verb::Detach { life: life.clone() }).await, Ok(Answer::Done));
    tokio::time::sleep(PAST).await;
    assert!(!window.lives(&life).await);
    let mut later = Window::dialed(&local).await;
    opened(&mut later, &at, "a.jsonl").await;
    later.attach(&life).await;
    later.ran(&life, "y = 1").await;
    let facts: Vec<Value> = later
      .pushed
      .iter()
      .filter_map(|frame| match frame {
        Frame::Event { event: Event::Fact(fact), .. } => Some(fact.clone()),
        _ => None,
      })
      .collect();
    let about = |kind: &str| facts.iter().filter(|one| one[0] == kind && one[1] == first).count();
    let mine: Vec<&Value> = facts.iter().filter(|one| one[1] == first).collect();
    assert_eq!((about("rung"), about("done")), (1, 1), "{mine:?}");
    let mut seen = HashSet::new();
    let again: Vec<&Value> = facts.iter().filter(|one| !seen.insert(one.to_string())).collect();
    assert!(again.is_empty(), "the window heard these facts twice: {again:?}");
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

  /// The view of a kit that a window mounts in the footer of a life.
  fn mount(life: &str, viewer: &str) -> Verb {
    let (viewer, life, kit) = (viewer.to_owned(), life.to_owned(), "k".to_owned());
    Verb::ViewMount(Mount { viewer, life, slot: Slot::Footer, kit })
  }

  /// A tree with a button, which only a window that writes may press.
  fn tree(viewer: &str) -> Verb {
    let button =
      Node { id: "go".to_owned(), shape: Shape::Button { label: "go".to_owned(), inert: false } };
    let shape = Shape::Box { axis: Axis::Row, gap: 0, border: false, children: vec![button] };
    let tree = Node { id: "root".to_owned(), shape };
    Verb::ViewTree(Tree { viewer: viewer.to_owned(), slot: Slot::Footer, version: 1, tree })
  }

  /// Whether a frame is the tree of a viewer, and whether its button is inert.
  fn inert(frame: &Frame) -> Option<bool> {
    let Frame::ViewTree(Tree { tree: Node { shape: Shape::Box { children, .. }, .. }, .. }) = frame
    else {
      return None;
    };
    let Some(Node { shape: Shape::Button { inert, .. }, .. }) = children.first() else {
      return None;
    };
    Some(*inert)
  }

  fn pressed(viewer: &str) -> Verb {
    let (viewer, node) = (viewer.to_owned(), "go".to_owned());
    Verb::ViewEvent(ViewEvent { viewer, node, kind: Gesture::Press, value: None })
  }

  #[tokio::test]
  async fn a_kit_serves_a_tree_to_a_writer_and_a_reader_and_hears_the_writer_alone() {
    let at = place();
    let local = started_as(&at, &[Role::Write, Role::Write, Role::Read]).await;
    let mut kit = Window::dialed_as(&local, Kind::Kit, Some("k")).await;
    let (mut writer, mut reader) = (Window::dialed(&local).await, Window::dialed(&local).await);
    let life = opened(&mut writer, &at, "a.jsonl").await;
    for peer in [&mut kit, &mut writer, &mut reader] {
      peer.attach(&life).await;
    }
    for (window, viewer) in [(&mut writer, "w"), (&mut reader, "r")] {
      assert_eq!(window.ask(mount(&life, viewer)).await, Ok(Answer::Done));
      kit.until(|frame| matches!(frame, Frame::ViewMount(one) if one.viewer == viewer)).await;
      assert_eq!(kit.ask(tree(viewer)).await, Ok(Answer::Done));
    }
    assert_eq!(inert(&writer.until(|frame| inert(frame).is_some()).await), Some(false));
    assert_eq!(inert(&reader.until(|frame| inert(frame).is_some()).await), Some(true));
    let notice = Effect::Notice { text: "done".to_owned() };
    let effect = Verb::ViewEffect(ViewEffect { viewer: "w".to_owned(), effect: notice });
    assert_eq!(kit.ask(effect).await, Ok(Answer::Done));
    writer.until(|frame| matches!(frame, Frame::ViewEffect(_))).await;
    assert!(writer.pushed.iter().all(|frame| inert(frame).is_none()));
    let refused = reader.ask(pressed("r")).await;
    assert_eq!(refused, Err(ProtocolError::Role { needs: Role::Write, has: Role::Read }));
    assert_eq!(writer.ask(pressed("w")).await, Ok(Answer::Done));
    let heard = kit.until(|frame| matches!(frame, Frame::ViewEvent(_))).await;
    assert_eq!(
      heard,
      Frame::ViewEvent(ViewEvent {
        viewer: "w".to_owned(),
        node: "go".to_owned(),
        kind: Gesture::Press,
        value: None
      })
    );
  }

  #[tokio::test]
  async fn a_mount_needs_a_kit_and_a_holder_and_a_view_ends_with_its_peer() {
    let at = place();
    let local = started_as(&at, &[Role::Write, Role::Write]).await;
    let (mut writer, mut kit) =
      (Window::dialed(&local).await, Window::dialed_as(&local, Kind::Kit, Some("k")).await);
    let life = opened(&mut writer, &at, "a.jsonl").await;
    writer.attach(&life).await;
    let nokit = ProtocolError::NoKit { life: life.clone(), kit: "k".to_owned() };
    assert_eq!(writer.ask(mount(&life, "v")).await, Err(nokit));
    kit.attach(&life).await;
    assert_eq!(writer.ask(mount(&life, "v")).await, Ok(Answer::Done));
    let taken = writer.ask(mount(&life, "v")).await;
    assert_eq!(taken, Err(ProtocolError::Mounted { viewer: "v".to_owned() }));
    let stranger = kit
      .ask(Verb::ViewEvent(ViewEvent {
        viewer: "v".to_owned(),
        node: "go".to_owned(),
        kind: Gesture::Press,
        value: None,
      }))
      .await;
    assert!(matches!(stranger, Err(ProtocolError::Speaker { .. })), "{stranger:?}");
    drop(kit);
    let ended = Frame::ViewDispose(Dispose { viewer: "v".to_owned() });
    assert_eq!(writer.until(|frame| matches!(frame, Frame::ViewDispose(_))).await, ended);
    let gone = writer.ask(pressed("v")).await;
    assert_eq!(gone, Err(ProtocolError::Unmounted { viewer: "v".to_owned() }));
  }

  #[tokio::test]
  async fn a_second_daemon_of_the_machine_refuses_to_start() {
    let at = place();
    let (local, _daemon) = started(&at, GRACE).await;
    let refused = local.listen().await;
    assert!(matches!(refused, Err(TransportError::Taken { .. })));
  }

  #[tokio::test]
  async fn a_window_that_answers_the_models_of_its_life_gives_each_turn() {
    let at = place();
    let (local, _daemon) = started(&at, GRACE).await;
    let mut window = Window::dialed(&local).await;
    let life = at.join("a.jsonl").display().to_string();
    let roster = Some(vec!["claude-cli:sonnet".to_owned()]);
    let cwd = Some(at.display().to_string());
    let open = Verb::Open {
      life: life.clone(),
      cwd,
      actor: None,
      roster,
      extensions: None,
      config: None,
      claude: None,
      answers: true,
      attach: None,
    };
    assert_eq!(window.ask(open).await, Ok(Answer::Done));
    window.attach(&life).await;
    let prompt = Verb::Prompt {
      life: life.clone(),
      shape: "str".to_owned(),
      message: "Say ok.".to_owned(),
      to: None,
      on: None,
    };
    let Ok(Answer::Act(id)) = window.ask(prompt).await else { panic!("no prompt") };
    let Frame::Ask { ask, request, .. } =
      window.until(|one| matches!(one, Frame::Ask { .. })).await
    else {
      panic!("no ask")
    };
    assert!(request["actor"].as_str().is_some_and(|one| one.contains("sonnet")), "{request}");
    let wrote = |ask| Verb::Wrote {
      life: life.clone(),
      ask,
      text: "close(".to_owned(),
      thinking: "So.".to_owned(),
    };
    assert_eq!(window.ask(wrote(ask)).await, Ok(Answer::Done));
    let thought = |one: &Frame| matches!(one, Frame::Event { event: Event::Text { thinking, .. }, .. } if thinking == "So.");
    window.until(thought).await;
    let turn = Some(json!(["assistant", "close('ok')", null, null]));
    let answered = Verb::Answered { life: life.clone(), ask, turn, why: None };
    assert_eq!(window.ask(answered.clone()).await, Ok(Answer::Done));
    let Frame::Event { event: Event::Act(act), .. } = window.until(done(&id)).await else {
      panic!("no act")
    };
    assert_eq!(act.value(), Some(&json!("ok")));
    let again = window.ask(answered).await;
    assert_eq!(again, Err(ProtocolError::Unasked { life: life.clone(), ask }));
    let other = opened(&mut window, &at, "b.jsonl").await;
    let answered = Verb::Answered { life: other.clone(), ask, turn: None, why: None };
    assert_eq!(window.ask(answered).await, Err(ProtocolError::Answerless { life: other }));
  }

  #[tokio::test]
  async fn a_command_that_a_window_feeds_reads_each_line_that_it_writes() {
    let at = place();
    let (local, _daemon) = started(&at, GRACE).await;
    let mut window = Window::dialed(&local).await;
    let life = opened(&mut window, &at, "a.jsonl").await;
    window.attach(&life).await;
    let bash = |command: &str, stdin: Option<&str>, feed| Verb::Bash {
      life: life.clone(),
      command: command.to_owned(),
      stdin: stdin.map(str::to_owned),
      feed,
      on: None,
    };
    let Ok(Answer::Act(given)) = window.ask(bash("cat", Some("given\n"), false)).await else {
      panic!("no command")
    };
    let Frame::Event { event: Event::Act(act), .. } = window.until(done(&given)).await else {
      panic!("no act")
    };
    assert!(act.value().is_some_and(|exit| exit.to_string().contains("given")), "{act:?}");
    let Ok(Answer::Act(fed)) = window.ask(bash("cat", None, true)).await else {
      panic!("no command")
    };
    for content in ["fed\n", ""] {
      let write = Verb::Write {
        life: life.clone(),
        path: format!("{fed}/stdin"),
        content: content.to_owned(),
        on: None,
      };
      assert_eq!(window.ask(write).await, Ok(Answer::Done));
    }
    let Frame::Event { event: Event::Act(act), .. } = window.until(done(&fed)).await else {
      panic!("no act")
    };
    assert!(act.value().is_some_and(|exit| exit.to_string().contains("fed")), "{act:?}");
  }
}
