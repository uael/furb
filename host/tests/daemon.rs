//! The daemon of a machine, as its windows reach it on the local transport: the holders of a life,
//! the grace after the last of them, a fault of one life, and the handover to a new build.

#![expect(
  clippy::disallowed_methods,
  reason = "the tests block on the runtime that tokio starts for them"
)]

#[path = "../../tests/common/yard.rs"]
mod yard;

#[cfg(test)]
mod daemon {
  use super::yard::Yard;

  use std::process::Command;
  use std::process::Stdio;
  use std::sync::mpsc::Receiver;
  use std::sync::mpsc::Sender;
  use std::sync::mpsc::channel;
  use std::{
    cell::RefCell,
    collections::VecDeque,
    env,
    fs::{self, File},
    ops::Deref,
    path::{Path, PathBuf},
    process,
    sync::{
      Arc, Mutex,
      atomic::{AtomicU32, AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, Instant},
  };

  use furb::{
    Object,
    life::{Answer as Answered, Log, Opening},
    view::Phase,
    world::codex,
  };
  use furb_host::{
    DaemonError, Event, GRACE, ProtocolError, STALL, Settings, Tailnet, TransportError,
    access::{Access, Ports},
    protocol::{
      Answer, Axis, Config, Dispose, Effect, Frame, Gesture, Hello, Kind, Listed, Mount, Node,
      PROTOCOL, Request, Role, Shape, Slot, Tree, Verb, ViewEffect, ViewEvent,
    },
    run,
    transport::{Accepted, Dialer, Identity, Local, LocalApi, Tailscale, Who},
    upgrade,
  };
  use futures::{SinkExt, StreamExt};
  use serde_json::Value;
  use serde_json::json;
  use tokio::task;
  use tokio::time;
  use tokio::{task::JoinHandle, time::timeout};
  use tokio_util::codec::{Framed, LinesCodec};

  /// How long a test waits for a frame before it fails.
  const PATIENCE: Duration = Duration::from_secs(60);

  /// A grace short enough for a test, and the wait that passes it with room.
  const SHORT: Duration = Duration::from_millis(300);
  const PAST: Duration = Duration::from_millis(1500);

  /// A folder of one test, empty, and short enough for the path of a socket. It goes at the end of
  /// the test, and again once the thread of the test ends: the end of the runtime of the test ends
  /// the daemon, and each life that it held and whose grace is short then writes its checkpoint
  /// there, on a thread of its own, before it lets its lease go.
  fn place() -> Place {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    Place(Yard::new(format!("fd{}-{n}", process::id())))
  }

  /// The folder of one test, which goes with it.
  struct Place(Yard);

  impl Deref for Place {
    type Target = Path;

    fn deref(&self) -> &Path {
      &self.0
    }
  }

  impl Drop for Place {
    fn drop(&mut self) {
      let entries = fs::read_dir(&*self.0).into_iter().flatten().flatten();
      // Each lease is open before the yard goes, since a lock holds the file and not its name.
      let leases = entries
        .map(|one| one.path())
        .filter(|one| one.extension().is_some_and(|ext| ext == "lock"))
        .filter_map(|one| File::open(one).ok())
        .collect();
      HELD.with(|held| held.0.borrow_mut().push((self.0.to_path_buf(), leases)));
    }
  }

  thread_local! {
    static HELD: Held = const { Held(RefCell::new(Vec::new())) };
  }

  /// The folder of each test of this thread, and the leases of its records, which go again when
  /// the thread ends, once no life holds them.
  struct Held(RefCell<Vec<(PathBuf, Vec<File>)>>);

  impl Drop for Held {
    fn drop(&mut self) {
      let kept = env::var_os("FURB_KEEP_YARDS").is_some();
      for (folder, leases) in self.0.take() {
        // A write of a life that lives on makes its folder again, and a life that the test asked
        // may still open, so the folder goes at each look until each lease is free, with a bound
        // twice the time that a life of a short grace takes to end.
        let end = Instant::now() + 2 * PAST;
        loop {
          if !kept {
            drop(fs::remove_dir_all(&folder));
          }
          let free = !leases.is_empty() && leases.iter().all(|lease| lease.try_lock().is_ok());
          if free || Instant::now() > end {
            break;
          }
          thread::sleep(Duration::from_millis(20));
        }
        if !kept {
          drop(fs::remove_dir_all(&folder));
        }
      }
    }
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
    started_with(at, grace, opening(at)).await
  }

  /// A daemon that listens in a folder with an opening of the test, served on a task of the test.
  async fn started_with(
    at: &Path,
    grace: Duration,
    opening: Opening,
  ) -> (Local, JoinHandle<Result<(), DaemonError>>) {
    let local = Local::new(at.join("sock"));
    let listener = local.listen().await.unwrap();
    let settings = Settings { opening, grace, stall: STALL, tailnet: None, door: None, host: None };
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

    async fn identify(&self, stream: &Accepted) -> Result<Who, TransportError> {
      let who = self.local.identify(stream).await?;
      let at = self.next.fetch_add(1, Ordering::SeqCst);
      Ok(Who { role: self.roles[at], ..who })
    }
  }

  /// A daemon that gives each peer that dials, in turn, the role of a list.
  async fn started_as(at: &Path, roles: &[Role]) -> Local {
    let local = Local::new(at.join("sock"));
    let listener = local.listen().await.unwrap();
    let settings = Settings {
      opening: opening(at),
      grace: GRACE,
      stall: STALL,
      tailnet: None,
      door: None,
      host: None,
    };
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
    life: String,
  }

  impl Window {
    /// A window that dialed the daemon and said its hello.
    async fn dialed(local: &Local) -> Self {
      Self::dialed_as(local, Kind::Window, None).await
    }

    /// A peer of a kind that dialed the daemon and said its hello.
    async fn dialed_as(local: &Local, kind: Kind, name: Option<&str>) -> Self {
      let lines = Framed::new(local.dial().await.unwrap(), LinesCodec::new());
      let mut window = Self { lines, next: 0, pushed: VecDeque::new(), life: String::new() };
      let name = name.map(str::to_owned);
      let hello = Verb::Hello(Hello { protocol: PROTOCOL, kind, name, role: None });
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

    /// The first frame that the test waits for, which keeps each frame before it for `until`.
    async fn keeping(&mut self, wanted: impl Fn(&Frame) -> bool) -> Frame {
      loop {
        let frame = self.read().await.expect("the daemon pushes the frame");
        if wanted(&frame) {
          return frame;
        }
        self.pushed.push_back(frame);
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

    /// The last reply of a life, which took the last answer of a model.
    async fn replied(&mut self, life: &str) -> String {
      let of = Some("reply".to_owned());
      let replies = Verb::Transcript { life: life.to_owned(), on: None, of };
      let Ok(Answer::Values(replies)) = self.ask(replies).await else { panic!("no replies") };
      let reply = replies.last().and_then(|one| one[1].as_str().map(str::to_owned));
      reply.expect("a reply took the answer")
    }

    async fn attach(&mut self, life: &str) {
      assert_eq!(self.ask(Verb::Attach { life: life.to_owned(), from: 0 }).await, Ok(Answer::Done));
    }

    async fn listed(&mut self) -> Vec<Listed> {
      let Ok(Answer::Lives(lives)) = self.ask(Verb::List { config: None }).await else {
        panic!("no list")
      };
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
    time::sleep(PAST).await;
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
    time::sleep(PAST).await;
    assert!(window.lives(&life).await);
    let unpin = Verb::Unpin { life: life.clone(), holder: "job".to_owned() };
    assert_eq!(window.ask(unpin).await, Ok(Answer::Done));
    time::sleep(PAST).await;
    assert!(!window.lives(&life).await);
    let refused = window.ask(pin()).await;
    assert!(matches!(refused, Err(ProtocolError::Closed { .. })), "{refused:?}");
  }

  #[tokio::test]
  async fn the_list_gives_each_record_of_the_sessions_of_each_workspace_and_whether_a_life_holds_it()
   {
    let at = place();
    let main = at.join("main");
    let sessions = main.join(".furb").join("sessions");
    let tree = at.join("tree");
    let marks = main.join(".git").join("worktrees").join("tree");
    for folder in [&sessions, &tree.join(".furb").join("sessions"), &marks] {
      fs::create_dir_all(folder).unwrap();
    }
    fs::write(marks.join("gitdir"), format!("{}\n", tree.join(".git").display())).unwrap();
    fs::write(tree.join(".git"), "gitdir: elsewhere\n").unwrap();
    let tree = dunce::canonicalize(&tree).unwrap().join(".furb").join("sessions");
    fs::write(tree.join("two.jsonl"), "").unwrap();
    fs::write(tree.join("two.changes.jsonl"), "").unwrap();
    let list = json!({ "workspaces": [{ "directory": main.display().to_string() }] });
    fs::create_dir_all(at.join("config")).unwrap();
    fs::write(at.join("config").join("workspaces.json"), list.to_string()).unwrap();
    let (local, _daemon) = started(&at, GRACE).await;
    let mut window = Window::dialed(&local).await;
    let one = opened(&mut window, &sessions, "one.jsonl").await;
    let alone = opened(&mut window, &at, "a.jsonl").await;
    let workspace = Some(main.display().to_string());
    let mut wanted = vec![
      Listed {
        life: one,
        workspace: workspace.clone(),
        held: true,
        open: true,
        watched: false,
        name: None,
      },
      Listed { life: alone, workspace: None, held: true, open: true, watched: false, name: None },
      Listed {
        life: tree.join("two.jsonl").display().to_string(),
        workspace,
        held: false,
        open: false,
        watched: false,
        name: None,
      },
    ];
    wanted.sort_by(|a, b| a.life.cmp(&b.life));
    let listed = window.listed().await;
    assert_eq!(listed, wanted);
    let verbatim = |one: &Listed| one.life.starts_with(r"\\?\");
    assert!(!listed.iter().any(verbatim), "a worktree is listed under the path that made it");
  }

  #[tokio::test]
  async fn the_list_says_whether_a_window_holds_each_life_and_a_pin_is_no_window() {
    let at = place();
    let (local, _daemon) = started(&at, GRACE).await;
    let mut window = Window::dialed(&local).await;
    let mut other = Window::dialed(&local).await;
    let life = opened(&mut window, &at, "a.jsonl").await;
    let watched = |listed: Vec<Listed>| listed.iter().any(|one| one.life == life && one.watched);
    assert!(!watched(window.listed().await), "an open that attaches nothing is held by no window");
    let pin = Verb::Pin { life: life.clone(), holder: "skill".to_owned() };
    assert_eq!(window.ask(pin).await, Ok(Answer::Done));
    assert!(!watched(window.listed().await), "a pin holds the life and no window holds it");
    other.attach(&life).await;
    assert!(watched(window.listed().await), "a window of another connection holds the life");
    assert_eq!(other.ask(Verb::Detach { life: life.clone() }).await, Ok(Answer::Done));
    assert!(!watched(window.listed().await), "a window that detached holds nothing");
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
    time::sleep(PAST).await;
    assert!(window.lives(&life).await);
    window.ran(&life, "x = 1").await;
    assert_eq!(window.ask(Verb::Detach { life: life.clone() }).await, Ok(Answer::Done));
    time::sleep(PAST).await;
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
    time::sleep(PAST).await;
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
    time::sleep(PAST).await;
    assert!(!window.lives(&life).await);
    // A second birth opens on the checkpoint of the first, and ends with a checkpoint of its own.
    let mut between = Window::dialed(&local).await;
    opened(&mut between, &at, "a.jsonl").await;
    between.attach(&life).await;
    between.ran(&life, "z = 0").await;
    assert_eq!(between.ask(Verb::Detach { life: life.clone() }).await, Ok(Answer::Done));
    time::sleep(PAST).await;
    assert!(!between.lives(&life).await);
    let mut later = Window::dialed(&local).await;
    opened(&mut later, &at, "a.jsonl").await;
    later.attach(&life).await;
    later.ran(&life, "y = 1").await;
    let facts_of = |window: &Window| -> Vec<Value> {
      let facts = window.pushed.iter().filter_map(|frame| match frame {
        Frame::Event { event: Event::Fact(fact), .. } if fact[0] != "keep" => Some(fact.clone()),
        _ => None,
      });
      facts.collect()
    };
    let (once, twice, facts) = (facts_of(&window), facts_of(&between), facts_of(&later));
    let about = |kind: &str| facts.iter().filter(|one| one[0] == kind && one[1] == first).count();
    let mine: Vec<&Value> = facts.iter().filter(|one| one[1] == first).collect();
    assert_eq!((about("rung"), about("done")), (1, 1), "{mine:?}");
    // The second and the third births do alike, so each adds as many occurrences of each fact of
    // the first: a fact that a checkpoint restates told again would add more, and a fact that
    // repeats an earlier one, dropped, fewer.
    let times = |facts: &[Value], one: &Value| facts.iter().filter(|other| *other == one).count();
    let added = |later: &[Value], earlier: &[Value], one: &Value| {
      i64::try_from(times(later, one)).unwrap() - i64::try_from(times(earlier, one)).unwrap()
    };
    let unlike = |one: &&Value| added(&facts, &twice, one) != added(&twice, &once, one);
    let again: Vec<&Value> = once.iter().filter(unlike).collect();
    assert!(again.is_empty(), "the births add these facts unlike: {again:?}");
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

  /// A record whose replay drifts at the place of its first stand: its checkpoints dropped, so the
  /// life replays it from its first entry, and its first stand moved to a place that the life does
  /// not stand in.
  fn drifted(life: &str) -> Vec<u8> {
    let text = fs::read_to_string(life).unwrap();
    let checkpoint = |line: &&str| line.starts_with(r#"[["checkpoint","#);
    let mut lines: Vec<String> =
      text.lines().filter(|line| !checkpoint(line)).map(str::to_owned).collect();
    let stood = lines.iter().position(|line| line.starts_with(r#"[["done","stand1","#)).unwrap();
    let mut entry: Value = serde_json::from_str(&lines[stood]).unwrap();
    let place = entry[0][3][1].as_str().expect("the stand names its place").to_owned();
    entry[0][3][1] = Value::from(format!("{place}-elsewhere"));
    lines[stood] = entry.to_string();
    fs::write(life, lines.join("\n") + "\n").unwrap();
    fs::write(format!("{life}.log"), "{\"entries\": 0}\n").unwrap();
    fs::read(life).unwrap()
  }

  #[tokio::test]
  async fn a_drift_at_the_open_of_a_life_tells_its_window_why_and_leaves_the_daemon() {
    let at = place();
    let lines = Arc::new(Mutex::new(Vec::<String>::new()));
    let kept = Arc::clone(&lines);
    let log = Log(Arc::new(move |line: &str| kept.lock().unwrap().push(line.to_owned())));
    let (local, daemon) =
      started_with(&at, SHORT, Opening { log: Some(log), ..opening(&at) }).await;
    let mut window = Window::dialed(&local).await;
    let life = opened(&mut window, &at, "a.jsonl").await;
    window.attach(&life).await;
    window.ran(&life, "x = 1").await;
    assert_eq!(window.ask(Verb::Detach { life: life.clone() }).await, Ok(Answer::Done));
    time::sleep(PAST).await;
    assert!(!window.lives(&life).await);
    let held = drifted(&life);
    let mut later = Window::dialed(&local).await;
    let drift = |why: &str| why.starts_with("Drift: ") && why.contains(" drifts: ");
    let verb = Verb::Open {
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
    assert_eq!(later.ask(verb).await, Ok(Answer::Done));
    let ended = |frame: &Frame| matches!(frame, Frame::Event { life: one, event: Event::Ended(Some(why)), .. } if *one == life && drift(why));
    later.until(ended).await;
    let said = format!("furb host: the life of {life} ended: ");
    let told = |line: &String| line.strip_prefix(&said).is_some_and(drift);
    let start = Instant::now();
    while !lines.lock().unwrap().iter().any(told) {
      assert!(start.elapsed() < PATIENCE, "{:?}", lines.lock().unwrap());
      time::sleep(SHORT).await;
    }
    assert_eq!(fs::read(&life).unwrap(), held, "a life that drifts keeps nothing in its record");
    let other = opened(&mut later, &at, "b.jsonl").await;
    later.attach(&other).await;
    later.ran(&other, "y = 1").await;
    assert!(!daemon.is_finished());
  }

  #[tokio::test]
  async fn the_log_of_the_daemon_says_each_end_of_a_life_with_its_fault() {
    let at = place();
    let lines = Arc::new(Mutex::new(Vec::<String>::new()));
    let kept = Arc::clone(&lines);
    let log = Log(Arc::new(move |line: &str| kept.lock().unwrap().push(line.to_owned())));
    let (local, _daemon) =
      started_with(&at, SHORT, Opening { log: Some(log), ..opening(&at) }).await;
    let mut window = Window::dialed(&local).await;
    let life = opened(&mut window, &at, "a.jsonl").await;
    let said = format!("furb host: the life of {life} ended: with no fault");
    let start = Instant::now();
    while !lines.lock().unwrap().contains(&said) {
      assert!(start.elapsed() < PATIENCE, "{:?}", lines.lock().unwrap());
      time::sleep(SHORT).await;
    }
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
    let settings = Settings {
      opening: opening(&at),
      grace: SHORT,
      stall: STALL,
      tailnet: None,
      door: None,
      host: None,
    };
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
    time::sleep(PAST).await;
    assert!(window.lives(&life).await);
    assert!(!new.is_finished());
  }

  /// The opening of a test whose model blocks the thread of its life: it tells the test that it
  /// began, then waits until the test drops the sender of the gate.
  fn blocking(at: &Path, log: Arc<Mutex<Vec<String>>>) -> (Opening, Sender<()>, Receiver<()>) {
    let (hold, gate) = channel::<()>();
    let (began, entered) = channel::<()>();
    let (gate, began) = (Mutex::new(gate), Mutex::new(began));
    let inner = opening(at).answer.expect("the opening of a test has a model");
    let answer = Answered(Arc::new(move |kind, message| {
      began.lock().unwrap().send(()).ok();
      gate.lock().unwrap().recv().ok();
      (inner.0)(kind, message)
    }));
    let tell = Log(Arc::new(move |line: &str| log.lock().unwrap().push(line.to_owned())));
    (Opening { answer: Some(answer), log: Some(tell), ..opening(at) }, hold, entered)
  }

  #[tokio::test]
  async fn a_handover_waits_for_each_life_to_end_and_the_life_goes_on() {
    let at = place();
    let log = Arc::new(Mutex::new(Vec::new()));
    let (blocked, hold, entered) = blocking(&at, Arc::clone(&log));
    let local = Local::new(at.join("sock"));
    let listener = local.listen().await.unwrap();
    let settings = Settings {
      opening: blocked,
      grace: GRACE,
      stall: STALL,
      tailnet: None,
      door: None,
      host: None,
    };
    let served = local.clone();
    let old = tokio::spawn(async move { run(&served, listener, settings).await });
    let mut window = Window::dialed(&local).await;
    let life = opened(&mut window, &at, "a.jsonl").await;
    window.attach(&life).await;
    window.ran(&life, "x = 41 + 1").await;
    let word = "prompt(None, 'go', on=chain('side'))".to_owned();
    let verb = Verb::Run { life: life.clone(), word, on: None };
    assert!(matches!(window.ask(verb).await, Ok(Answer::Act(_))));
    let began = task::spawn_blocking(move || entered.recv_timeout(PATIENCE));
    assert!(began.await.unwrap().is_ok(), "the model of the life blocks its thread");
    let settings = Settings {
      opening: opening(&at),
      grace: GRACE,
      stall: STALL,
      tailnet: None,
      door: None,
      host: None,
    };
    let next = local.clone();
    let new = tokio::spawn(async move { upgrade(&next, settings).await });
    time::sleep(PAST).await;
    assert!(!old.is_finished(), "the handover waits for a life that has not ended");
    let seen = log.lock().unwrap().join("\n");
    assert!(
      seen.contains("the handover waits for each life to end") && seen.contains(&life),
      "the log says which lives the handover waits for: {seen}"
    );
    drop(hold);
    let gone = timeout(PATIENCE, old).await.expect("the handover ends once each life ended");
    assert!(matches!(gone, Ok(Ok(()))));
    let mut window = Window::dialed(&local).await;
    window.attach(&life).await;
    window.ran(&life, "y = x").await;
    let inspect = Verb::Inspect { life: life.clone(), name: "y".to_owned(), on: None };
    let Ok(Answer::Inspected { value, .. }) = window.ask(inspect).await else { panic!() };
    assert_eq!(value, json!(42), "the new daemon holds the life whole, and a word runs on it");
    assert!(!new.is_finished());
  }

  /// A view mounted in the footer by a window, once the skill that serves it attached the life.
  async fn mounted(window: &mut Window, verb: &Verb) {
    let start = Instant::now();
    while window.ask(verb.clone()).await != Ok(Answer::Done) {
      assert!(start.elapsed() < PATIENCE, "no skill serves {verb:?}");
      time::sleep(SHORT).await;
    }
  }

  /// Whether a frame is a tree whose first run says a text.
  fn shows(text: &str) -> impl Fn(&Frame) -> bool + '_ {
    move |frame| {
      matches!(frame, Frame::ViewTree(Tree { tree: Node { shape: Shape::Text { runs }, .. }, .. })
        if runs.first().is_some_and(|run| run.text == text))
    }
  }

  #[tokio::test]
  async fn a_kit_of_a_session_gives_its_view_to_each_window_in_each_life_of_its_record() {
    let at = place();
    fs::create_dir_all(at.join(".furb")).unwrap();
    fs::write(at.join(".furb").join("config.json"), r#"{"extensions": {"ouroboros": true}}"#)
      .unwrap();
    let local = Local::new(at.join("sock"));
    let listener = local.listen().await.unwrap();
    let opening = Opening { extensions: None, ..opening(&at) };
    let host = Some(at.join("sock"));
    let settings =
      Settings { opening, grace: SHORT, stall: STALL, tailnet: None, door: None, host };
    let served = local.clone();
    tokio::spawn(async move { run(&served, listener, settings).await });
    let mut window = Window::dialed(&local).await;
    let life = opened(&mut window, &at, "a.jsonl").await;
    window.attach(&life).await;
    let said = |text: &str| {
      format!(r#"view = {{"id": "todo", "node": "text", "runs": [{{"text": "{text}"}}]}}"#)
    };
    let word = window.ran(&life, &said("first")).await;
    window.ran(&life, &format!("extend('todo', '{word}', session=True, view='view')")).await;
    let mount = |viewer: &str| {
      let (viewer, life, skill) = (viewer.to_owned(), life.clone(), "todo".to_owned());
      Verb::ViewMount(Mount { viewer, life, slot: Slot::Footer, skill })
    };
    mounted(&mut window, &mount("v1")).await;
    window.until(shows("first")).await;
    window.ran(&life, &said("second")).await;
    window.until(shows("second")).await;
    assert_eq!(window.ask(Verb::Detach { life: life.clone() }).await, Ok(Answer::Done));
    time::sleep(PAST).await;
    assert!(!window.lives(&life).await);
    let mut later = Window::dialed(&local).await;
    opened(&mut later, &at, "a.jsonl").await;
    later.attach(&life).await;
    mounted(&mut later, &mount("v2")).await;
    later.until(shows("second")).await;
    assert!(!at.join(".furb").join("extensions").exists(), "the kit writes no file");
  }

  /// The view of a skill that a window mounts in the footer of a life.
  fn mount(life: &str, viewer: &str) -> Verb {
    let (viewer, life, skill) = (viewer.to_owned(), life.to_owned(), "k".to_owned());
    Verb::ViewMount(Mount { viewer, life, slot: Slot::Footer, skill })
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
  async fn a_skill_serves_a_tree_to_a_writer_and_a_reader_and_hears_the_writer_alone() {
    let at = place();
    let local = started_as(&at, &[Role::Write, Role::Write, Role::Read]).await;
    let mut skill = Window::dialed_as(&local, Kind::Skill, Some("k")).await;
    let (mut writer, mut reader) = (Window::dialed(&local).await, Window::dialed(&local).await);
    let life = opened(&mut writer, &at, "a.jsonl").await;
    for peer in [&mut skill, &mut writer, &mut reader] {
      peer.attach(&life).await;
    }
    for (window, viewer) in [(&mut writer, "w"), (&mut reader, "r")] {
      assert_eq!(window.ask(mount(&life, viewer)).await, Ok(Answer::Done));
      skill.until(|frame| matches!(frame, Frame::ViewMount(one) if one.viewer == viewer)).await;
      assert_eq!(skill.ask(tree(viewer)).await, Ok(Answer::Done));
    }
    assert_eq!(inert(&writer.until(|frame| inert(frame).is_some()).await), Some(false));
    assert_eq!(inert(&reader.until(|frame| inert(frame).is_some()).await), Some(true));
    let notice = Effect::Notice { text: "done".to_owned() };
    let effect = Verb::ViewEffect(ViewEffect { viewer: "w".to_owned(), effect: notice });
    assert_eq!(skill.ask(effect).await, Ok(Answer::Done));
    writer.until(|frame| matches!(frame, Frame::ViewEffect(_))).await;
    assert!(writer.pushed.iter().all(|frame| inert(frame).is_none()));
    let refused = reader.ask(pressed("r")).await;
    assert_eq!(refused, Err(ProtocolError::Role { needs: Role::Write, has: Role::Read }));
    assert_eq!(writer.ask(pressed("w")).await, Ok(Answer::Done));
    let heard = skill.until(|frame| matches!(frame, Frame::ViewEvent(_))).await;
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
  async fn a_mount_needs_a_skill_and_a_holder_and_a_view_ends_with_its_peer() {
    let at = place();
    let local = started_as(&at, &[Role::Write, Role::Write]).await;
    let (mut writer, mut skill) =
      (Window::dialed(&local).await, Window::dialed_as(&local, Kind::Skill, Some("k")).await);
    let life = opened(&mut writer, &at, "a.jsonl").await;
    writer.attach(&life).await;
    let noskill = ProtocolError::NoSkill { life: life.clone(), skill: "k".to_owned() };
    assert_eq!(writer.ask(mount(&life, "v")).await, Err(noskill));
    skill.attach(&life).await;
    assert_eq!(writer.ask(mount(&life, "v")).await, Ok(Answer::Done));
    let taken = writer.ask(mount(&life, "v")).await;
    assert_eq!(taken, Err(ProtocolError::Mounted { viewer: "v".to_owned() }));
    let stranger = skill
      .ask(Verb::ViewEvent(ViewEvent {
        viewer: "v".to_owned(),
        node: "go".to_owned(),
        kind: Gesture::Press,
        value: None,
      }))
      .await;
    assert!(matches!(stranger, Err(ProtocolError::Speaker { .. })), "{stranger:?}");
    drop(skill);
    let ended = Frame::ViewDispose(Dispose { viewer: "v".to_owned() });
    assert_eq!(writer.until(|frame| matches!(frame, Frame::ViewDispose(_))).await, ended);
    let gone = writer.ask(pressed("v")).await;
    assert_eq!(gone, Err(ProtocolError::Unmounted { viewer: "v".to_owned() }));
  }

  #[tokio::test]
  async fn a_skill_says_each_act_under_its_name_and_a_window_under_its_operator() {
    let at = place();
    let local = served(&at, None).await;
    let mut writer = Window::dialed(&local).await;
    let mut skill = Window::dialed_as(&local, Kind::Skill, Some("k")).await;
    let life = opened(&mut writer, &at, "a.jsonl").await;
    writer.attach(&life).await;
    skill.attach(&life).await;
    let to = Some("operator".to_owned());
    let message = "from k".to_owned();
    let asked =
      Verb::Prompt { life: life.clone(), shape: "None".to_owned(), message, to, on: None };
    let Ok(Answer::Act(asked)) = skill.ask(asked).await else { panic!("no prompt") };
    let said = skill.ran(&life, "x = 1").await;
    let mine = writer.ran(&life, "y = 1").await;
    let mut speakers = Vec::new();
    for id in [&asked, &said, &mine] {
      let get = Verb::Get { life: life.clone(), act: id.clone() };
      let Ok(Answer::Value(fact)) = writer.ask(get).await else { panic!("no act {id}") };
      speakers.push(fact[2].clone());
    }
    let Ok(Answer::Presence(peers)) = writer.ask(Verb::Presence { life: life.clone() }).await
    else {
      panic!("no presence")
    };
    let login = peers.iter().find(|one| one.kind == Kind::Window).map(|one| one.login.clone());
    let member = json!(format!("operator/{}", login.expect("the window is present")));
    assert_eq!(speakers, [json!("skill/k"), json!("skill/k"), member]);
    let y = Verb::Inspect { life: life.clone(), name: "x".to_owned(), on: None };
    assert!(matches!(skill.ask(y).await, Ok(Answer::Inspected { .. })), "the word of a skill ran");
  }

  #[tokio::test]
  async fn the_roster_of_a_life_names_the_user_of_the_daemon_as_a_member_of_the_operator() {
    let at = place();
    let (local, _daemon) = started(&at, GRACE).await;
    let mut window = Window::dialed(&local).await;
    let life = opened(&mut window, &at, "a.jsonl").await;
    window.attach(&life).await;
    let Ok(Answer::Presence(peers)) = window.ask(Verb::Presence { life: life.clone() }).await
    else {
      panic!("no presence")
    };
    let login = peers.iter().find(|one| one.kind == Kind::Window).map(|one| one.login.clone());
    let login = login.expect("the window is present");
    let word = format!(
      "members = [x[1] for x in standing()[0] if x[0] == OPERATOR][0]\nasks = offered(standing()[0], 'operator/{login}') is not None"
    );
    window.ran(&life, &word).await;
    let read = |name: &str| Verb::Inspect { life: life.clone(), name: name.to_owned(), on: None };
    let Ok(Answer::Inspected { value: members, .. }) = window.ask(read("members")).await else {
      panic!("no members")
    };
    assert_eq!(members, json!([login]), "the operator of the roster lists the user");
    let Ok(Answer::Inspected { value: asks, .. }) = window.ask(read("asks")).await else {
      panic!("no asks")
    };
    assert_eq!(asks, json!(true), "a prompt can ask that member by name");
  }

  /// The facts of the stands of a life, which grow with each stand that it asks or hears.
  async fn stands(window: &mut Window, life: &str) -> usize {
    let of = Some("stand".to_owned());
    let verb = Verb::Transcript { life: life.to_owned(), on: None, of };
    let Ok(Answer::Values(facts)) = window.ask(verb).await else { panic!("no stands") };
    facts.len()
  }

  /// The members of the operator in the roster of a life, read again until they are `until`, or
  /// as they stand when the patience of the test runs out.
  async fn members(window: &mut Window, life: &str, until: &Value) -> Value {
    let word = "members = [x[1] for x in standing()[0] if x[0] == OPERATOR][0]";
    let read = Verb::Inspect { life: life.to_owned(), name: "members".to_owned(), on: None };
    let mut found = Value::Null;
    for _ in 0..100 {
      window.ran(life, word).await;
      let Ok(Answer::Inspected { value, .. }) = window.ask(read.clone()).await else {
        panic!("no members")
      };
      found = value;
      if found == *until {
        break;
      }
      time::sleep(Duration::from_millis(50)).await;
    }
    found
  }

  #[tokio::test]
  async fn the_roster_names_the_access_list_and_a_change_of_it_tells_each_life_to_stand() {
    let at = place();
    let list = |logins: &[&str]| -> Access {
      logins.iter().map(|one| ((*one).to_owned(), Role::Write)).collect()
    };
    let api = LocalApi::Port { port: 1, token: "none".to_owned() };
    let identity = Tailscale::new(api, list(&["ann", "bob"]));
    let tailnet = Some(Tailnet { identity: identity.clone(), ports: Ports::default() });
    let local = Local::new(at.join("sock"));
    let listener = local.listen().await.unwrap();
    let settings = Settings {
      opening: opening(&at),
      grace: GRACE,
      stall: STALL,
      tailnet,
      door: None,
      host: None,
    };
    let served = local.clone();
    let _daemon = tokio::spawn(async move { run(&served, listener, settings).await });
    let mut window = Window::dialed(&local).await;
    let life = opened(&mut window, &at, "a.jsonl").await;
    window.attach(&life).await;
    let Ok(Answer::Presence(peers)) = window.ask(Verb::Presence { life: life.clone() }).await
    else {
      panic!("no presence")
    };
    let login = peers.iter().find(|one| one.kind == Kind::Window).map(|one| one.login.clone());
    let login = login.expect("the window is present");
    let before = json!([login, "ann", "bob"]);
    assert_eq!(members(&mut window, &life, &before).await, before, "the list gives the members");
    let stood = stands(&mut window, &life).await;
    identity.replace(list(&["ann", "bob", "cat"]));
    for _ in 0..100 {
      if stands(&mut window, &life).await > stood {
        break;
      }
      time::sleep(Duration::from_millis(50)).await;
    }
    assert!(
      stands(&mut window, &life).await > stood,
      "a change of the list tells the life to stand"
    );
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
    let turn = Some(json!(["assistant", "close('ok')", [500_000, 10, 0, 0, 0.0], null]));
    let answered = Verb::Answered { life: life.clone(), ask, turn, why: None };
    assert_eq!(window.ask(answered.clone()).await, Ok(Answer::Done));
    let Frame::Event { event: Event::Act(act), .. } = window.until(done(&id)).await else {
      panic!("no act")
    };
    assert_eq!(act.value(), Some(&json!("ok")));
    let filled = Verb::Filled { life: life.clone(), act: window.replied(&life).await };
    let share = window.ask(filled).await;
    let half = |one: &Value| one.as_f64().is_some_and(|share| share > 0.0 && share <= 1.0);
    assert!(matches!(&share, Ok(Answer::Value(one)) if half(one)), "{share:?}");
    let again = window.ask(answered).await;
    assert_eq!(again, Err(ProtocolError::Unasked { life: life.clone(), ask }));
    let other = opened(&mut window, &at, "b.jsonl").await;
    let answered = Verb::Answered { life: other.clone(), ask, turn: None, why: None };
    assert_eq!(window.ask(answered).await, Err(ProtocolError::Answerless { life: other }));
  }

  #[tokio::test]
  async fn a_peer_that_opens_with_answers_answers_in_the_place_of_one_that_is_gone() {
    let at = place();
    let (local, _daemon) = started(&at, GRACE).await;
    let life = at.join("a.jsonl").display().to_string();
    let open = Verb::Open {
      life: life.clone(),
      cwd: Some(at.display().to_string()),
      actor: None,
      roster: Some(vec!["claude-cli:sonnet".to_owned()]),
      extensions: None,
      config: None,
      claude: None,
      answers: true,
      attach: Some(0),
    };
    let mut first = Window::dialed(&local).await;
    assert_eq!(first.ask(open.clone()).await, Ok(Answer::Done));
    drop(first);
    let mut window = Window::dialed(&local).await;
    let mut taken = window.ask(open.clone()).await;
    while taken.is_err() {
      time::sleep(SHORT).await;
      taken = window.ask(open.clone()).await;
    }
    assert_eq!(taken, Ok(Answer::Done), "the peer that answered is gone");
    let prompt = Verb::Prompt {
      life: life.clone(),
      shape: "str".to_owned(),
      message: "Say ok.".to_owned(),
      to: None,
      on: None,
    };
    let Ok(Answer::Act(id)) = window.ask(prompt).await else { panic!("no prompt") };
    let Frame::Ask { ask, .. } = window.until(|one| matches!(one, Frame::Ask { .. })).await else {
      panic!("the new peer is asked")
    };
    let turn = Some(json!(["assistant", "close('ok')", [10, 10, 0, 0, 0.0], null]));
    let answered = Verb::Answered { life: life.clone(), ask, turn, why: None };
    assert_eq!(window.ask(answered).await, Ok(Answer::Done));
    let Frame::Event { event: Event::Act(act), .. } = window.until(done(&id)).await else {
      panic!("no act")
    };
    assert_eq!(act.value(), Some(&json!("ok")));
  }

  #[tokio::test]
  async fn a_second_writer_cannot_answer_the_request_of_the_peer_that_answers_the_models() {
    let at = place();
    let (local, _daemon) = started(&at, GRACE).await;
    let (mut window, mut other) = (Window::dialed(&local).await, Window::dialed(&local).await);
    let life = at.join("a.jsonl").display().to_string();
    let open = Verb::Open {
      life: life.clone(),
      cwd: Some(at.display().to_string()),
      actor: None,
      roster: Some(vec!["claude-cli:sonnet".to_owned()]),
      extensions: None,
      config: None,
      claude: None,
      answers: true,
      attach: None,
    };
    assert_eq!(window.ask(open.clone()).await, Ok(Answer::Done));
    window.attach(&life).await;
    let answering = Err(ProtocolError::Answering { life: life.clone() });
    assert_eq!(other.ask(open).await, answering, "the peer that answers lives");
    other.attach(&life).await;
    let prompt = Verb::Prompt {
      life: life.clone(),
      shape: "str".to_owned(),
      message: "Say ok.".to_owned(),
      to: None,
      on: None,
    };
    let Ok(Answer::Act(id)) = other.ask(prompt).await else { panic!("no prompt") };
    let Frame::Ask { ask, .. } = window.until(|one| matches!(one, Frame::Ask { .. })).await else {
      panic!("no ask")
    };
    let foreign = Err(ProtocolError::Foreign { life: life.clone(), ask });
    let wrote = Verb::Wrote {
      life: life.clone(),
      ask,
      text: "close('taken')".to_owned(),
      thinking: String::new(),
    };
    assert_eq!(other.ask(wrote).await, foreign);
    let turn = Some(json!(["assistant", "close('taken')", [10, 10, 0, 0, 0.0], null]));
    let taken = Verb::Answered { life: life.clone(), ask, turn, why: None };
    assert_eq!(other.ask(taken).await, foreign);
    let turn = Some(json!(["assistant", "close('ok')", [10, 10, 0, 0, 0.0], null]));
    let answered = Verb::Answered { life: life.clone(), ask, turn, why: None };
    assert_eq!(window.ask(answered).await, Ok(Answer::Done));
    let Frame::Event { event: Event::Act(act), .. } = window.until(done(&id)).await else {
      panic!("no act")
    };
    assert_eq!(act.value(), Some(&json!("ok")));
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

  /// The manifest of a skill of a test whose process says where it stands, then waits for its end.
  #[cfg(unix)]
  const PROBE: &str = "---\nname: probe\ndescription: Says where its process stands.\nmetadata:\n  furb-word: probe.py\n  furb-process: probe.sh\n---\nA probe.\n";

  /// The lines of a file once it holds a count of them.
  #[cfg(unix)]
  async fn lines(file: &Path, count: usize) -> Vec<String> {
    let read = || fs::read_to_string(file).unwrap_or_default().lines().map(str::to_owned).collect();
    let waited = timeout(PATIENCE, async {
      loop {
        let got: Vec<String> = read();
        if got.len() >= count {
          return got;
        }
        time::sleep(Duration::from_millis(50)).await;
      }
    });
    waited.await.expect("the file gets its lines")
  }

  /// Whether the process of a pid ends.
  #[cfg(unix)]
  async fn gone(pid: &str) -> bool {
    let lives = || {
      Command::new("kill")
        .args(["-0", pid])
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|one| one.success())
    };
    let waited = timeout(PATIENCE, async {
      while lives() {
        time::sleep(Duration::from_millis(50)).await;
      }
    });
    waited.await.is_ok()
  }

  #[cfg(unix)]
  #[tokio::test]
  async fn the_process_of_a_skill_lives_from_its_play_to_its_reload_and_to_the_end_of_its_life() {
    use std::os::unix::fs::PermissionsExt;
    let at = place();
    let skill = at.join(".furb").join("skills").join("probe");
    fs::create_dir_all(&skill).unwrap();
    fs::write(skill.join("SKILL.md"), PROBE).unwrap();
    fs::write(skill.join("probe.py"), "PROBED = 1\n").unwrap();
    let program = skill.join("probe.sh");
    let said =
      "#!/bin/sh\necho \"$FURB_SKILL $FURB_LIFE $FURB_HOST $$\" >> started\nexec sleep 600\n";
    fs::write(&program, said).unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    let local = Local::new(at.join("sock"));
    let listener = local.listen().await.unwrap();
    let host = Some(at.join("sock"));
    let settings = Settings {
      opening: opening(&at),
      grace: SHORT,
      stall: STALL,
      tailnet: None,
      door: None,
      host,
    };
    let served = local.clone();
    tokio::spawn(async move { run(&served, listener, settings).await });
    let mut window = Window::dialed(&local).await;
    let life = opened(&mut window, &at, "a.jsonl").await;
    window.attach(&life).await;
    window.ran(&life, "x = 1").await;
    let started = skill.join("started");
    assert!(!started.exists(), "a skill that the life does not play starts no process");
    window.ran(&life, r#"act("skill", "", None, "probe")"#).await;
    let first = lines(&started, 1).await.remove(0);
    let first: Vec<&str> = first.split(' ').collect();
    let sock = at.join("sock").display().to_string();
    assert_eq!(first[..3], ["probe", life.as_str(), sock.as_str()]);
    window.ran(&life, r#"write(Text("skills://probe", "PROBED = 2"))"#).await;
    // The write copies the folder of the skill in the project, whose word the reload plays, so the
    // process of that copy, which keeps the lines of the file, starts.
    let started = at.join(".furb").join("extensions").join("probe").join("started");
    let second = lines(&started, 2).await.remove(1);
    assert!(gone(first[3]).await, "a reload ends the process that ran");
    let second = second.rsplit(' ').next().unwrap().to_owned();
    assert_eq!(window.ask(Verb::Detach { life: life.clone() }).await, Ok(Answer::Done));
    time::sleep(PAST).await;
    assert!(!window.lives(&life).await);
    assert!(gone(&second).await, "the end of the life ends its process");
    opened(&mut window, &at, "a.jsonl").await;
    let third = lines(&started, 3).await.remove(2);
    time::sleep(PAST).await;
    assert_eq!(lines(&started, 3).await.len(), 3, "a replay starts the process once, at its tip");
    assert!(third.starts_with("probe "));
  }

  /// The bound of a stall in a test: a gap between two facts of a word that waits 0.05 s in a loop
  /// stays far below it on a loaded machine.
  const QUIET: Duration = Duration::from_millis(1500);

  /// Whether a frame tells that a life stalled.
  fn stall(frame: &Frame) -> bool {
    matches!(frame, Frame::Stalled { .. })
  }

  /// Whether a frame is the fact of a prompt of None, the message of a stall to the root.
  fn message(frame: &Frame) -> bool {
    matches!(frame, Frame::Event { event: Event::Fact(fact), .. }
      if fact[0] == "prompt" && fact[3] == "chain1" && fact[4] == "None")
  }

  impl Window {
    /// Each frame that the daemon pushed, up to the first that the test waits for.
    async fn heard_until(&mut self, wanted: impl Fn(&Frame) -> bool) -> Vec<Frame> {
      let mut heard: Vec<Frame> = self.pushed.drain(..).collect();
      while !heard.last().is_some_and(&wanted) {
        heard.push(self.read().await.expect("the daemon pushes the frame"));
      }
      heard
    }

    /// Each frame that the daemon pushed for a while.
    async fn during(&mut self, span: Duration) -> Vec<Frame> {
      let mut heard: Vec<Frame> = self.pushed.drain(..).collect();
      let end = time::Instant::now() + span;
      while let Ok(Some(line)) = time::timeout_at(end, self.lines.next()).await {
        heard.push(serde_json::from_str(&line.unwrap()).unwrap());
      }
      heard
    }
  }

  #[tokio::test]
  async fn a_life_that_holds_an_open_act_and_says_no_fact_is_told_once_to_its_windows_and_root() {
    let at = place();
    let local = Local::new(at.join("sock"));
    let listener = local.listen().await.unwrap();
    let settings = Settings {
      opening: opening(&at),
      grace: GRACE,
      stall: QUIET,
      tailnet: None,
      door: None,
      host: None,
    };
    let served = local.clone();
    tokio::spawn(async move { run(&served, listener, settings).await });
    let mut window = Window::dialed(&local).await;
    let life = opened(&mut window, &at, "a.jsonl").await;
    window.attach(&life).await;
    let flows = "for _ in range(60):\n  await wait(0.05, __name__)";
    let verb = Verb::Run { life: life.clone(), word: flows.to_owned(), on: None };
    let Ok(Answer::Act(rung)) = window.ask(verb).await else { panic!("no rung") };
    let heard = window.heard_until(done(&rung)).await;
    assert!(!heard.iter().any(stall), "a run whose facts flow for twice the bound never stalls");
    let asked = r#"await bash("sleep 20", timeout=60)"#;
    let verb = Verb::Run { life: life.clone(), word: asked.to_owned(), on: None };
    let began = Instant::now();
    let Ok(Answer::Act(_)) = window.ask(verb).await else { panic!("no rung") };
    let Frame::Stalled { acts, notice, .. } = window.until(stall).await else { unreachable!() };
    assert!(began.elapsed() >= QUIET, "the daemon tells a stall after its bound alone");
    assert!(acts.iter().any(|one| one.kind == "run" && one.chain == "chain1"), "{acts:?}");
    assert!(notice.contains("1.5 seconds") && notice.contains(" on chain1"), "{notice}");
    let Frame::Event { event: Event::Fact(sent), .. } = window.until(message).await else {
      unreachable!()
    };
    assert_eq!(sent[5], notice.as_str(), "the root takes the notice as its message");
    let heard = window.during(QUIET * 3).await;
    assert!(!heard.iter().any(stall), "the watch tells one stall once");
    assert!(!heard.iter().any(message), "the watch sends one message for one stall");
    let verb = Verb::Run { life: life.clone(), word: "x = 1".to_owned(), on: None };
    let Ok(Answer::Act(rung)) = window.ask(verb).await else { panic!("no rung") };
    let heard = window.heard_until(stall).await;
    assert!(heard.iter().any(done(&rung)), "a new stall comes after a new fact");
    window.until(message).await;
    let prompts = Verb::Transcript { life, on: None, of: Some("prompt".to_owned()) };
    let Ok(Answer::Values(prompts)) = window.ask(prompts).await else { panic!("no prompts") };
    let messages = prompts.iter().filter(|one| one[4] == "None").count();
    assert_eq!(messages, 2, "a new fact and a new stall send a new message: {prompts:?}");
  }

  #[tokio::test]
  async fn a_rung_that_waits_with_no_reply_is_told_to_its_chain_after_the_bound() {
    let at = place();
    let local = Local::new(at.join("sock"));
    let listener = local.listen().await.unwrap();
    let settings = Settings {
      opening: opening(&at),
      grace: GRACE,
      stall: QUIET,
      tailnet: None,
      door: None,
      host: None,
    };
    let served = local.clone();
    tokio::spawn(async move { run(&served, listener, settings).await });
    let mut window = Window::dialed(&local).await;
    let life = opened(&mut window, &at, "a.jsonl").await;
    window.attach(&life).await;
    let waits = "side = chain(\"side\")\nlong = bash(\"for i in $(seq 100); do echo $i; sleep 0.1; done\", \
                 on=side)\nrung(\"x = 1\", long)";
    let verb = Verb::Run { life: life.clone(), word: waits.to_owned(), on: None };
    let began = Instant::now();
    let Ok(Answer::Act(_)) = window.ask(verb).await else { panic!("no rung") };
    let Frame::Stalled { acts, notice, .. } = window.until(stall).await else { unreachable!() };
    assert!(began.elapsed() >= QUIET, "the daemon tells a stall after its bound alone");
    assert!(acts.iter().any(|one| one.kind == "rung" && one.chain == "chain1"), "{acts:?}");
    assert!(!acts.iter().any(|one| one.kind == "run"), "no run is open: {acts:?}");
    let Frame::Event { event: Event::Fact(sent), .. } = window.until(message).await else {
      unreachable!()
    };
    assert_eq!(sent[5], notice.as_str(), "the root takes the notice as its message");
  }

  #[tokio::test]
  async fn a_chain_stuck_beside_a_chain_that_works_is_told_alone_and_takes_the_notice() {
    let at = place();
    let local = Local::new(at.join("sock"));
    let listener = local.listen().await.unwrap();
    let settings = Settings {
      opening: opening(&at),
      grace: GRACE,
      stall: QUIET,
      tailnet: None,
      door: None,
      host: None,
    };
    let served = local.clone();
    tokio::spawn(async move { run(&served, listener, settings).await });
    let mut window = Window::dialed(&local).await;
    let life = opened(&mut window, &at, "a.jsonl").await;
    window.attach(&life).await;
    let word = "long = bash(\"for i in $(seq 100); do echo $i; sleep 0.1; done\")\nside = chain(\"side\")\n\
                rung(\"x = 1\", long, on=side)\nfor _ in range(80):\n  await wait(0.05, __name__)";
    let verb = Verb::Run { life: life.clone(), word: word.to_owned(), on: None };
    let Ok(Answer::Act(rung)) = window.ask(verb).await else { panic!("no rung") };
    let heard = window.heard_until(stall).await;
    assert!(!heard.iter().any(done(&rung)), "the stall comes while the root works");
    let Some(Frame::Stalled { acts, notice, .. }) = heard.last() else { unreachable!() };
    let side = acts.first().map(|one| one.chain.clone()).expect("the stall names an act");
    assert!(side != "chain1" && acts.iter().all(|one| one.chain == side), "{acts:?}");
    assert!(acts.iter().any(|one| one.kind == "rung"), "{acts:?}");
    let taken = |frame: &Frame| {
      matches!(frame, Frame::Event { event: Event::Fact(fact), .. }
        if fact[0] == "prompt" && fact[3] == side.as_str() && fact[4] == "None")
    };
    let Frame::Event { event: Event::Fact(sent), .. } = window.until(taken).await else {
      unreachable!()
    };
    assert_eq!(sent[5], notice.as_str(), "the stuck chain takes the notice as its message");
  }

  /// The skill probe in a folder of skills, whose process writes the file `started` of its folder,
  /// which this gives.
  #[cfg(unix)]
  fn probed(skills: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let skill = skills.join("probe");
    fs::create_dir_all(&skill).unwrap();
    fs::write(skill.join("SKILL.md"), PROBE).unwrap();
    fs::write(skill.join("probe.py"), "PROBED = 1\n").unwrap();
    let program = skill.join("probe.sh");
    fs::write(&program, "#!/bin/sh\necho \"$FURB_SKILL $$\" >> started\nexec sleep 600\n").unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    skill.join("started")
  }

  /// A daemon of a test whose lives end after a short grace, and which starts the processes of
  /// skills when it is given the folder where they dial it.
  async fn served(at: &Path, host: Option<PathBuf>) -> Local {
    let local = Local::new(at.join("sock"));
    let listener = local.listen().await.unwrap();
    let settings = Settings {
      opening: opening(at),
      grace: SHORT,
      stall: STALL,
      tailnet: None,
      door: None,
      host,
    };
    let served = local.clone();
    tokio::spawn(async move { run(&served, listener, settings).await });
    local
  }

  /// A daemon of a test that starts the processes of skills, and a window dialed to it.
  #[cfg(unix)]
  async fn hosting(at: &Path) -> Window {
    Window::dialed(&served(at, Some(at.join("sock"))).await).await
  }

  /// The skill probe of a test in the folder of the skills of a project, whose process is the
  /// script given, and the file `started` of its folder.
  #[cfg(unix)]
  fn scripted(at: &Path, script: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let skill = at.join(".furb").join("skills").join("probe");
    fs::create_dir_all(&skill).unwrap();
    fs::write(skill.join("SKILL.md"), PROBE).unwrap();
    fs::write(skill.join("probe.py"), "PROBED = 1\n").unwrap();
    let program = skill.join("probe.sh");
    fs::write(&program, script).unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    skill.join("started")
  }

  /// Whether a frame is the fact of a message that the process of the skill probe says to the
  /// operator.
  #[cfg(unix)]
  fn crashed(frame: &Frame) -> bool {
    matches!(frame, Frame::Event { event: Event::Fact(fact), .. }
      if fact[0] == "prompt" && fact[2] == "skill/probe" && fact[6] == "operator")
  }

  #[cfg(unix)]
  #[tokio::test]
  async fn the_process_of_a_skill_comes_from_the_folder_that_the_chain_played_it_from() {
    let at = place();
    let (one, two) = (at.join("one"), at.join("two"));
    let first = probed(&one.join(".furb").join("skills"));
    let second = probed(&two.join(".furb").join("skills"));
    let mut window = hosting(&at).await;
    let life = opened(&mut window, &one, "a.jsonl").await;
    window.attach(&life).await;
    window.ran(&life, &format!("cd({:?})", two.display().to_string())).await;
    window.ran(&life, r#"act("skill", "", None, "probe")"#).await;
    let started = lines(&second, 1).await.remove(0);
    time::sleep(PAST).await;
    assert!(!first.exists(), "the skill of the folder of the birth starts no process");
    assert_eq!(window.ask(Verb::Detach { life: life.clone() }).await, Ok(Answer::Done));
    time::sleep(PAST).await;
    assert!(gone(started.rsplit(' ').next().unwrap()).await, "the end of the life ends it");
    opened(&mut window, &one, "a.jsonl").await;
    lines(&second, 2).await;
    time::sleep(PAST).await;
    assert!(!first.exists(), "a reopen starts the process of the folder that the record names");
  }

  #[cfg(unix)]
  #[tokio::test]
  async fn the_process_of_a_skill_of_the_config_of_an_open_starts() {
    let at = place();
    let config = at.join("other");
    let started = probed(&config.join("skills"));
    let mut window = hosting(&at).await;
    let life = at.join("a.jsonl").display().to_string();
    let verb = Verb::Open {
      life: life.clone(),
      cwd: Some(at.display().to_string()),
      actor: None,
      roster: None,
      extensions: None,
      config: Some(config.display().to_string()),
      claude: None,
      answers: false,
      attach: None,
    };
    assert_eq!(window.ask(verb).await, Ok(Answer::Done));
    window.attach(&life).await;
    window.ran(&life, r#"act("skill", "", None, "probe")"#).await;
    assert!(lines(&started, 1).await[0].starts_with("probe "));
  }

  #[cfg(unix)]
  #[tokio::test]
  async fn a_process_that_ends_before_its_life_tells_the_operator_once_and_starts_at_the_next_enable()
   {
    let at = place();
    let script = "#!/bin/sh\necho $$ >> started\necho first >&2\necho 'broke here' >&2\nexit 3\n";
    let started = scripted(&at, script);
    let mut window = hosting(&at).await;
    let life = opened(&mut window, &at, "a.jsonl").await;
    window.attach(&life).await;
    window.ran(&life, r#"act("skill", "", None, "probe")"#).await;
    let Frame::Event { event: Event::Fact(told), .. } = window.until(crashed).await else {
      unreachable!()
    };
    assert_eq!((&told[3], &told[4]), (&json!("chain1"), &json!("None")));
    let message = told[5].as_str().unwrap();
    assert!(message.contains("probe.sh of the skill probe ended with exit status: 3"), "{message}");
    assert!(message.ends_with("stderr:\nfirst\nbroke here"), "{message}");
    let heard = window.during(PAST).await;
    assert!(!heard.iter().any(crashed), "a process that ended is told once");
    assert_eq!(lines(&started, 1).await.len(), 1, "a process that ended does not start by itself");
    window.ran(&life, r#"write(Text("skills://probe", "PROBED = 2"))"#).await;
    let again = at.join(".furb").join("extensions").join("probe").join("started");
    assert_eq!(lines(&again, 2).await.len(), 2, "the next enable starts the process again");
    window.until(crashed).await;
  }

  #[cfg(unix)]
  #[tokio::test]
  async fn the_end_of_a_life_kills_each_process_that_the_process_of_its_skill_started() {
    let at = place();
    let script = "#!/bin/sh\nsleep 600 &\necho \"$$ $!\" >> started\nexec sleep 600\n";
    let started = scripted(&at, script);
    let mut window = hosting(&at).await;
    let life = opened(&mut window, &at, "a.jsonl").await;
    window.attach(&life).await;
    window.ran(&life, r#"act("skill", "", None, "probe")"#).await;
    let pids = lines(&started, 1).await.remove(0);
    let (process, child) = pids.split_once(' ').unwrap();
    assert_eq!(window.ask(Verb::Detach { life: life.clone() }).await, Ok(Answer::Done));
    time::sleep(PAST).await;
    assert!(!window.lives(&life).await);
    assert!(gone(process).await, "the end of the life ends the process");
    assert!(gone(child).await, "the end of the life ends what the process started");
    let heard = window.during(PAST).await;
    assert!(!heard.iter().any(crashed), "a process that the life ends is not told");
  }

  /// One test at a time holds the port of the browser callback of a sign-in.
  static PORT: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

  /// The request that turns codex on, said with no wait for its answer, and its id.
  async fn signing(window: &mut Window) -> u64 {
    let id = window.next;
    window.next += 1;
    let key = ["providers", "codex"].map(str::to_owned).to_vec();
    let config = Config::Set { key, value: Some(json!(true)), layer: None };
    let verb = Verb::Config { life: window.life.clone(), config };
    window.lines.send(serde_json::to_string(&Request { id, verb }).unwrap()).await.unwrap();
    id
  }

  fn signed_in(state: &'static str) -> impl Fn(&Frame) -> bool {
    move |frame| matches!(frame, Frame::SignIn { state: got, .. } if got == state)
  }

  #[tokio::test]
  async fn a_sign_in_of_codex_that_fails_is_pushed_to_the_window_and_refuses_its_config() {
    let _port = PORT.lock().await;
    let at = place();
    let (local, _daemon) = started(&at, GRACE).await;
    let mut window = Window::dialed(&local).await;
    window.life = opened(&mut window, &at, "a.jsonl").await;
    let life = window.life.clone();
    window.attach(&life).await;
    let taken = std::net::TcpListener::bind(("127.0.0.1", 1455)).unwrap();
    let id = signing(&mut window).await;
    let Frame::SignIn { url, message, .. } = window.keeping(signed_in("failed")).await else {
      unreachable!()
    };
    assert_eq!(url, None);
    assert!(message.is_some_and(|why| !why.is_empty()), "the frame tells why it failed");
    let answer =
      window.until(|frame| matches!(frame, Frame::Refused { id: Some(got), .. } if *got == id));
    assert!(matches!(answer.await, Frame::Refused { error: ProtocolError::Refused { .. }, .. }));
    drop(taken);
  }

  #[tokio::test]
  async fn a_sign_in_of_codex_that_the_operator_cancels_is_pushed_and_refuses_its_config() {
    let _port = PORT.lock().await;
    let at = place();
    let (local, _daemon) = started(&at, GRACE).await;
    let mut window = Window::dialed(&local).await;
    window.life = opened(&mut window, &at, "a.jsonl").await;
    let life = window.life.clone();
    window.attach(&life).await;
    let id = signing(&mut window).await;
    let Frame::SignIn { url, .. } = window.keeping(signed_in("waiting")).await else {
      unreachable!()
    };
    assert!(url.is_some_and(|one| one.starts_with("https://")), "the frame holds the address");
    codex::cancel(Some(&at.join("config")));
    window.keeping(signed_in("canceled")).await;
    let answer =
      window.until(|frame| matches!(frame, Frame::Refused { id: Some(got), .. } if *got == id));
    assert!(matches!(answer.await, Frame::Refused { error: ProtocolError::Refused { .. }, .. }));
  }
}
