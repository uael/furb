//! `furb host` ended by a signal at each moment of the work of a life, then started again: the
//! kill matrix of #591, which proves D8 of #983 on the daemon. After each kill, the life opens
//! on its record with no drift and with its module as it stood, no model answers a turn twice,
//! no command runs twice, a reply that was asked is answered with no wake, no process of the
//! life outlives the daemon, and the open replays at most `TAIL` facts after the last checkpoint.
//!
//! A scripted claude stands in for the models. Each process of it, and each command that holds
//! the life at a moment, holds the write end of the FIFO `lifeline`, so the test knows that no
//! process outlived the daemon when the FIFO ends, and waits on no clock.

#![cfg(unix)]
#![expect(
  clippy::disallowed_methods,
  reason = "the tests block on the runtime that tokio starts for them"
)]

#[path = "../../tests/common/yard.rs"]
mod yard;

#[cfg(test)]
mod kills {
  use std::{
    ffi::CString,
    fs,
    os::unix::{ffi::OsStrExt, fs::PermissionsExt},
    path::PathBuf,
    process::Stdio,
    sync::atomic::{AtomicU32, Ordering},
    time::Duration,
  };

  use furb::view::Phase;
  use furb_host::{
    Event,
    protocol::{Answer, Frame, Hello, Kind, PROTOCOL, Request, Verb},
    transport::{Dialer, Local},
  };
  use futures::{SinkExt, StreamExt};
  use serde_json::Value;
  use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
    sync::mpsc,
    time::timeout,
  };
  use tokio_util::codec::{Framed, LinesCodec};

  /// How long a test waits for a frame, a line or the end of a process before it fails.
  const PATIENCE: Duration = Duration::from_secs(60);

  /// The most facts that an open replays after the last checkpoint of its record, which is the
  /// tail at which a life of the host writes its checkpoint and is born again.
  const TAIL: usize = 10_000;

  /// The fake claude of the tests of the crate.
  const FAKE: &str = include_str!("../../src/world/provider/claude/fake-claude.sh");

  /// The two signals that end the daemon: one that it may handle, and one that it cannot.
  const SIGNALS: [i32; 2] = [libc::SIGTERM, libc::SIGKILL];

  /// A word that the life keeps in its module before each kill, which each open after it reads.
  const KEEP: &str = "kept = 41";

  /// The word that proves the module of the root as it stood before the kill.
  const KEPT: &str = "assert kept == 41";

  /// A folder of one test, short enough for the path of a socket: the work of the life, the fake
  /// claude and its FIFO, the config of the user and the cache of furb, where the daemon listens.
  struct Yard {
    root: PathBuf,
    /// The folder of the yard, which goes with it.
    _base: super::yard::Yard,
  }

  impl Yard {
    fn new() -> Yard {
      static NEXT: AtomicU32 = AtomicU32::new(0);
      let n = NEXT.fetch_add(1, Ordering::Relaxed);
      let base = super::yard::Yard::new(format!("fk{}-{n}", std::process::id()));
      let root = fs::canonicalize(&base).unwrap();
      for folder in ["work", "fake", "config"] {
        fs::create_dir_all(root.join(folder)).unwrap();
      }
      fs::write(root.join("config/config.json"), r#"{"extensions": {"naming": false}}"#).unwrap();
      let claude = root.join("fake/claude");
      fs::write(&claude, FAKE).unwrap();
      fs::set_permissions(&claude, fs::Permissions::from_mode(0o755)).unwrap();
      Yard { root, _base: base }
    }

    fn life(&self) -> String {
      self.root.join("work/record.jsonl").display().to_string()
    }

    fn fake(&self, name: &str) -> PathBuf {
      self.root.join("fake").join(name)
    }

    /// The lines of a file of the work, or none.
    fn lines(&self, name: &str) -> Vec<String> {
      let text = fs::read_to_string(self.root.join("work").join(name)).unwrap_or_default();
      text.lines().map(str::to_owned).collect()
    }

    /// The words that the fake answers with, from its next turn on.
    fn words(&self, words: &[&str]) {
      let count = fs::read_to_string(self.fake("count")).unwrap_or_default();
      let done: usize = count.trim().parse().unwrap_or_default();
      for (at, word) in words.iter().enumerate() {
        let said = serde_json::to_string(word).unwrap();
        fs::write(self.fake(&format!("word.{}", done + at + 1)), said).unwrap();
      }
    }

    /// How many turns the fake answered.
    fn answered(&self) -> usize {
      let count = fs::read_to_string(self.fake("count")).unwrap_or_default();
      count.trim().parse().unwrap_or_default()
    }

    /// The facts of the record, each as JSON, past a line that a kill cut.
    fn facts(&self) -> Vec<Value> {
      let text = fs::read_to_string(self.life()).unwrap_or_default();
      text.lines().filter_map(|line| serde_json::from_str::<Value>(line).ok()).collect()
    }

    /// The program furb with the env of the yard.
    fn furb(&self, words: &[&str]) -> Command {
      let mut command = Command::new(env!("CARGO_BIN_EXE_furb"));
      command.args(words).current_dir(self.root.join("work")).kill_on_drop(true);
      command.env("FURB_CLAUDE_BIN", self.fake("claude")).env_remove("FURB_TUI");
      for name in ["FURB_LOCAL_URL", "FURB_LOCAL_KEY", "MAX_THINKING_TOKENS"] {
        command.env_remove(name);
      }
      command.env("FURB_CONFIG_DIR", self.root.join("config"));
      command.env("XDG_CACHE_HOME", self.root.join("cache"));
      command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::inherit());
      command
    }

    fn local(&self) -> Local {
      Local::new(self.root.join("cache/furb/host"))
    }
  }

  /// The FIFO that each process of the fake and each command that holds the life writes to: the
  /// lines that they say, which a thread reads, and its end once the last of them is gone.
  struct Lifeline {
    at: PathBuf,
    lines: mpsc::UnboundedReceiver<String>,
    own: Option<fs::File>,
  }

  impl Lifeline {
    /// The FIFO made beside the fake, read, and held open by the test until the moment that it
    /// waits for, so that it ends with the processes of the life alone.
    fn new(yard: &Yard) -> Lifeline {
      let at = yard.fake("lifeline");
      let path = CString::new(at.as_os_str().as_bytes()).unwrap();
      // SAFETY: the path is a string of C that lives for the call.
      assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0, "the FIFO is made");
      // The end of the test opens first, for reading and writing, which waits for no other end, so
      // the end for reading opens with no wait too.
      let own = Some(fs::OpenOptions::new().read(true).write(true).open(&at).unwrap());
      let read = std::io::BufReader::new(fs::File::open(&at).unwrap());
      let (says, lines) = mpsc::unbounded_channel();
      std::thread::spawn(move || {
        for line in std::io::BufRead::lines(read).map_while(Result::ok) {
          says.send(line).ok();
        }
      });
      Lifeline { at, lines, own }
    }

    /// The path that a command writes to.
    fn path(&self) -> String {
      self.at.display().to_string()
    }

    /// The next line that starts with a word.
    async fn heard(&mut self, word: &str) {
      loop {
        let line = timeout(PATIENCE, self.lines.recv()).await.expect("a line on the lifeline");
        if line.expect("the lifeline is open").starts_with(word) {
          return;
        }
      }
    }

    /// The FIFO read to its end, once the test let go of its own end: no process of the life
    /// outlived the daemon. It goes, so no later process holds it.
    async fn ended(mut self) {
      self.own.take();
      fs::remove_file(&self.at).unwrap();
      let read = timeout(PATIENCE, async { while self.lines.recv().await.is_some() {} }).await;
      assert!(read.is_ok(), "a process of the life outlived the daemon");
    }
  }

  /// A daemon of the yard, as a process.
  struct Daemon {
    child: Child,
  }

  impl Daemon {
    /// `furb host`, once it listens.
    async fn started(yard: &Yard) -> Daemon {
      Daemon::spawned(yard.furb(&["host"]), true).await
    }

    async fn spawned(mut command: Command, listens: bool) -> Daemon {
      let mut child = command.spawn().unwrap();
      if listens {
        let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
        let line = timeout(PATIENCE, lines.next_line()).await.expect("the daemon listens");
        assert!(line.unwrap().is_some_and(|line| line.starts_with("furb host listens")));
      }
      Daemon { child }
    }

    /// The daemon once it ended by itself.
    async fn ended(mut self) {
      timeout(PATIENCE, self.child.wait()).await.expect("the daemon ends").unwrap();
    }

    /// The daemon ended by a signal, once it is gone.
    async fn killed(mut self, signal: i32) {
      let pid = i32::try_from(self.child.id().unwrap()).unwrap();
      // SAFETY: the pid is of a child of the test that it has not waited for.
      assert_eq!(unsafe { libc::kill(pid, signal) }, 0);
      timeout(PATIENCE, self.child.wait()).await.expect("the daemon ends").unwrap();
    }
  }

  /// A window: its stream, the id of its next request, and each frame that it read while it
  /// waited for another.
  struct Window {
    lines: Framed<<Local as Dialer>::Stream, LinesCodec>,
    next: u64,
    pushed: Vec<Frame>,
    life: String,
  }

  impl Window {
    /// A window that said its hello, opened the life of the yard and attached it from its start.
    async fn attached(yard: &Yard) -> Window {
      let mut window = Window::dialed(yard).await;
      window.hello().await;
      let open = Verb::Open {
        life: window.life.clone(),
        cwd: None,
        actor: None,
        roster: None,
        extensions: None,
        config: None,
        claude: None,
        answers: false,
        attach: None,
      };
      assert_eq!(window.ask(open).await, Answer::Done);
      let attach = Verb::Attach { life: window.life.clone(), from: 0 };
      assert_eq!(window.ask(attach).await, Answer::Done);
      window
    }

    async fn dialed(yard: &Yard) -> Window {
      let stream = timeout(PATIENCE, yard.local().dial()).await.unwrap().unwrap();
      Window {
        lines: Framed::new(stream, LinesCodec::new()),
        next: 0,
        pushed: Vec::new(),
        life: yard.life(),
      }
    }

    async fn hello(&mut self) {
      let hello =
        Verb::Hello(Hello { protocol: PROTOCOL, kind: Kind::Window, name: None, role: None });
      assert!(matches!(self.ask(hello).await, Answer::Welcome { .. }));
    }

    /// The stream of the window read to its end, which the daemon closed.
    async fn closed(&mut self) {
      while timeout(PATIENCE, self.lines.next()).await.expect("the daemon closes").is_some() {}
    }

    /// A request said, with no wait for its answer.
    async fn said(&mut self, verb: Verb) -> u64 {
      let id = self.next;
      self.next += 1;
      self.lines.send(serde_json::to_string(&Request { id, verb }).unwrap()).await.unwrap();
      id
    }

    async fn read(&mut self) -> Frame {
      let line = timeout(PATIENCE, self.lines.next()).await.expect("the daemon says a frame");
      serde_json::from_str(&line.expect("the daemon stays").unwrap()).unwrap()
    }

    async fn ask(&mut self, verb: Verb) -> Answer {
      let id = self.said(verb).await;
      loop {
        match self.read().await {
          Frame::Reply { id: got, answer } if got == id => return answer,
          Frame::Refused { id: Some(got), error, .. } if got == id => panic!("refused: {error}"),
          frame => self.pushed.push(frame),
        }
      }
    }

    /// The first event of the life that the test waits for.
    async fn until(&mut self, wanted: impl Fn(&Event) -> bool) -> Event {
      let at = self
        .pushed
        .iter()
        .position(|frame| matches!(frame, Frame::Event { event, .. } if wanted(event)));
      if let Some(Frame::Event { event, .. }) = at.map(|at| self.pushed.remove(at)) {
        return event;
      }
      loop {
        if let Frame::Event { event, .. } = self.read().await {
          assert!(!matches!(&event, Event::Ended(Some(fault)) if !fault.is_empty()), "{event:?}");
          if wanted(&event) {
            return event;
          }
        }
      }
    }

    /// A verb that makes an act, and the name of that act.
    async fn made(&mut self, verb: Verb) -> String {
      let Answer::Act(id) = self.ask(verb).await else { panic!("no act") };
      id
    }

    /// A word run on the root, with no wait for its end: its rung.
    async fn run(&mut self, word: &str) -> String {
      self.made(Verb::Run { life: self.life.clone(), word: word.to_owned(), on: None }).await
    }

    /// A word run on the root to its end, which it reaches with no fault.
    async fn ran(&mut self, word: &str) {
      let rung = self.run(word).await;
      let value = self.done(&rung).await;
      assert!(value.is_null(), "{word} ended with {value}");
    }

    /// A prompt of the root to its model: the act.
    async fn prompt(&mut self, message: &str) -> String {
      let (shape, message) = ("str".to_owned(), message.to_owned());
      self.made(Verb::Prompt { life: self.life.clone(), shape, message, to: None, on: None }).await
    }

    /// What an act came to, once it is done.
    async fn done(&mut self, id: &str) -> Value {
      let ended = |event: &Event| match event {
        Event::Act(act) => act.id == id && matches!(act.phase, Phase::Done(_)),
        _ => false,
      };
      let Event::Act(act) = self.until(ended).await else { unreachable!() };
      let Phase::Done(value) = act.phase else { unreachable!() };
      value
    }

    /// That no act waits for a wake, since the open put each act that the kill cut to the outside.
    async fn unheld(&mut self) {
      let pending = self.ask(Verb::Pending { life: self.life.clone() }).await;
      assert_eq!(pending, Answer::Values(Vec::new()), "the open left work that waits for a wake");
    }
  }

  /// The life of the yard after a kill, opened by a new daemon: its module as it stood, no turn and
  /// no command twice, and at most `TAIL` facts after the last checkpoint. The daemon and the
  /// window stay for the test to go on.
  async fn reopened(yard: &Yard) -> (Daemon, Window) {
    let daemon = Daemon::started(yard).await;
    let mut window = Window::attached(yard).await;
    window.ran(KEPT).await;
    let facts = yard.facts();
    let since = facts.iter().rev().take_while(|fact| fact[0][0] != "checkpoint").count();
    assert!(since <= TAIL, "the open replays {since} facts after the last checkpoint");
    let reply = |fact: &&Value| fact[0][1].as_str().is_some_and(|id| id.starts_with("reply"));
    let ended: Vec<&Value> =
      facts.iter().filter(reply).filter(|fact| fact[0][0] == "done").collect();
    let replies = ended.iter().filter(|fact| fact[0][3]["is"] == "tuple").count();
    // A reply that the open asks again may be answered by the model before its end is in the
    // record, so the record holds each turn once, but for the replies that are still open.
    let open = facts
      .iter()
      .filter(reply)
      .filter(|fact| fact[0][0] == "reply" && !ended.iter().any(|done| done[0][1] == fact[0][1]))
      .count();
    let answered = yard.answered();
    assert!(
      replies <= answered && answered <= replies + open,
      "each turn that the model answered is in the record once: {answered} answered, {replies} in the record, {open} open"
    );
    let ran = yard.lines("out.txt");
    for line in &ran {
      let times = ran.iter().filter(|one| *one == line).count();
      assert_eq!(times, 1, "the command that wrote {line} ran {times} times");
    }
    (daemon, window)
  }

  /// A yard with a daemon, its lifeline, and a window on its life, which kept a name in its module.
  async fn begun() -> (Yard, Lifeline, Daemon, Window) {
    let yard = Yard::new();
    let lifeline = Lifeline::new(&yard);
    let daemon = Daemon::started(&yard).await;
    let mut window = Window::attached(&yard).await;
    window.ran(KEEP).await;
    (yard, lifeline, daemon, window)
  }

  /// A command that holds the life until the daemon goes: it says that it runs on the lifeline,
  /// which it holds open, and runs with no end.
  fn held(lifeline: &Lifeline, mark: &str) -> String {
    let at = lifeline.path();
    let command =
      format!("echo {mark} >> out.txt; exec 9>>{at}; echo up >&9; exec tail -f /dev/null");
    format!("await bash({command:?}, timeout=600)")
  }

  #[tokio::test]
  async fn a_daemon_killed_during_an_open_leaves_a_life_that_opens_again() {
    for signal in SIGNALS {
      let (yard, lifeline, daemon, window) = begun().await;
      // The life ends with its daemon, so the next daemon opens it from its record.
      drop(window);
      daemon.killed(libc::SIGKILL).await;
      let daemon = Daemon::started(&yard).await;
      let mut window = Window::dialed(&yard).await;
      window.hello().await;
      window
        .said(Verb::Open {
          life: yard.life(),
          cwd: None,
          actor: None,
          roster: None,
          extensions: None,
          config: None,
          claude: None,
          answers: false,
          attach: None,
        })
        .await;
      daemon.killed(signal).await;
      lifeline.ended().await;
      reopened(&yard).await;
    }
  }

  #[tokio::test]
  async fn a_daemon_killed_at_a_hello_leaves_a_life_that_opens_again() {
    for signal in SIGNALS {
      let (yard, lifeline, daemon, _window) = begun().await;
      let mut second = Window::dialed(&yard).await;
      let hello =
        Verb::Hello(Hello { protocol: PROTOCOL, kind: Kind::Window, name: None, role: None });
      second.said(hello).await;
      daemon.killed(signal).await;
      lifeline.ended().await;
      reopened(&yard).await;
    }
  }

  #[tokio::test]
  async fn a_daemon_killed_while_a_reply_is_asked_answers_it_once_after() {
    for signal in SIGNALS {
      let (yard, mut lifeline, daemon, mut window) = begun().await;
      fs::write(yard.fake("hold"), "asked").unwrap();
      let prompt = window.prompt("HOLD the reply").await;
      lifeline.heard("held").await;
      daemon.killed(signal).await;
      lifeline.ended().await;
      fs::remove_file(yard.fake("hold")).unwrap();
      let (_daemon, mut window) = reopened(&yard).await;
      window.unheld().await;
      assert!(window.done(&prompt).await.is_string(), "the reply that was asked is answered");
      assert_eq!(yard.answered(), 1);
    }
  }

  #[tokio::test]
  async fn a_daemon_killed_leaves_no_process_that_the_command_line_of_a_model_started() {
    let (yard, mut lifeline, daemon, mut window) = begun().await;
    fs::write(yard.fake("orphan"), "").unwrap();
    fs::write(yard.fake("hold"), "asked").unwrap();
    window.prompt("HOLD the reply").await;
    lifeline.heard("held").await;
    daemon.killed(libc::SIGKILL).await;
    lifeline.ended().await;
  }

  #[tokio::test]
  async fn a_daemon_killed_while_a_reply_streams_answers_it_once_after() {
    for signal in SIGNALS {
      let (yard, lifeline, daemon, mut window) = begun().await;
      fs::write(yard.fake("hold"), "stream").unwrap();
      let prompt = window.prompt("HOLD the reply").await;
      window.until(|event| matches!(event, Event::Text { .. })).await;
      daemon.killed(signal).await;
      lifeline.ended().await;
      fs::remove_file(yard.fake("hold")).unwrap();
      let (_daemon, mut window) = reopened(&yard).await;
      window.unheld().await;
      assert!(window.done(&prompt).await.is_string(), "the reply that streamed is answered");
      assert_eq!(yard.answered(), 1);
    }
  }

  #[tokio::test]
  async fn a_daemon_killed_after_a_part_of_a_word_ran_keeps_that_part_and_runs_it_once() {
    for signal in SIGNALS {
      let (yard, mut lifeline, daemon, mut window) = begun().await;
      let word =
        format!("part = 1\nawait bash('echo part >> out.txt')\n{}", held(&lifeline, "held"));
      window.run(&word).await;
      lifeline.heard("up").await;
      daemon.killed(signal).await;
      lifeline.ended().await;
      let (_daemon, mut window) = reopened(&yard).await;
      window.ran("assert part == 1").await;
      assert_eq!(yard.lines("out.txt"), ["part", "held"]);
    }
  }

  #[tokio::test]
  async fn a_daemon_killed_during_a_command_of_a_model_runs_neither_twice() {
    for signal in SIGNALS {
      let (yard, mut lifeline, daemon, mut window) = begun().await;
      yard.words(&[&held(&lifeline, "bash")]);
      let prompt = window.prompt("run it").await;
      lifeline.heard("up").await;
      daemon.killed(signal).await;
      lifeline.ended().await;
      let (_daemon, mut window) = reopened(&yard).await;
      // The open ends the command that the kill cut as interrupted, with no wake, and the model
      // reads that end in the one turn that it is asked after.
      window.unheld().await;
      assert!(window.done(&prompt).await.is_string(), "the model goes on after the command");
      assert_eq!(yard.lines("out.txt"), ["bash"]);
      assert_eq!(yard.answered(), 2);
    }
  }

  /// A handover asked of the daemon by `furb host upgrade`, until the daemon that ran gave its
  /// lives, wrote the checkpoint of each and closed each window: the new daemon then opens them.
  async fn handed(yard: &Yard, daemon: Daemon, mut window: Window) -> Daemon {
    let upgrade = Daemon::spawned(yard.furb(&["host", "upgrade"]), false).await;
    window.closed().await;
    daemon.ended().await;
    upgrade
  }

  #[tokio::test]
  async fn a_daemon_killed_during_the_write_of_a_checkpoint_leaves_a_life_that_opens_again() {
    for signal in SIGNALS {
      let (yard, lifeline, daemon, window) = begun().await;
      handed(&yard, daemon, window).await.killed(signal).await;
      // A kill during the write of a checkpoint leaves a part of its line at the end of the
      // record, which no clock reaches: the test cuts the last checkpoint so.
      let text = fs::read_to_string(yard.life()).unwrap();
      let at = text.rfind("[[\"checkpoint\"").expect("the handover wrote a checkpoint");
      let end = text[at..].find('\n').map_or(text.len(), |one| at + one);
      fs::write(yard.life(), &text[..at + (end - at) / 2]).unwrap();
      lifeline.ended().await;
      reopened(&yard).await;
    }
  }

  #[tokio::test]
  async fn a_daemon_killed_during_a_handover_leaves_a_life_that_opens_again() {
    for signal in SIGNALS {
      let (yard, lifeline, daemon, window) = begun().await;
      handed(&yard, daemon, window).await.killed(signal).await;
      lifeline.ended().await;
      reopened(&yard).await;
    }
  }
}
