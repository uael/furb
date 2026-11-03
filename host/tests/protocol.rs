//! The proofs of the protocol: the role of each verb, the bound of a frame, the version of a hello,
//! and a round trip of each verb on each transport of the system.

#![expect(
  clippy::disallowed_methods,
  reason = "the tests block on the runtime that tokio starts for them"
)]

#[cfg(test)]
mod protocol {
  use std::{
    path::PathBuf,
    sync::atomic::{AtomicU32, Ordering},
  };

  use furb_host::{
    Host, Outbox, ProtocolError,
    protocol::{
      Answer, Axis, BOUND, Dispose, Effect, Frame, Gesture, Hello, Kind, Mount, Node, PROTOCOL,
      Peer, Request, Role, Run, Shape, Slot, Tone, Tree, Verb, ViewEffect, ViewEvent, schema,
    },
    serve,
    transport::{Dialer, Identity, Local, Who},
  };
  use serde_json::json;
  use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, duplex};

  /// A host that answers each verb with the verb itself, as JSON.
  struct Echo;

  impl Host for Echo {
    fn answer(
      &self,
      _: &Peer,
      verb: Verb,
      _: &Outbox,
    ) -> impl Future<Output = Result<Answer, ProtocolError>> + Send {
      let answer = serde_json::to_value(&verb)
        .map(Answer::Value)
        .map_err(|no| ProtocolError::Refused { why: no.to_string() });
      std::future::ready(answer)
    }
  }

  /// One of each verb but the hello.
  fn verbs() -> Vec<Verb> {
    let life = || "a.jsonl".to_owned();
    let id = || "prompt1".to_owned();
    let on = || Some("chain1".to_owned());
    vec![
      Verb::List,
      Verb::Open { life: life(), cwd: None },
      Verb::Attach { life: life(), from: 0 },
      Verb::Detach { life: life() },
      Verb::Pin { life: life(), holder: "job".to_owned() },
      Verb::Unpin { life: life(), holder: "job".to_owned() },
      Verb::Presence { life: life() },
      Verb::Get { life: life(), act: id() },
      Verb::Peek { life: life(), act: id() },
      Verb::Transcript { life: life(), on: on(), of: Some("bash".to_owned()) },
      Verb::Turns { life: life(), on: on() },
      Verb::Program { life: life(), on: on() },
      Verb::Standing { life: life() },
      Verb::Cwd { life: life(), on: on() },
      Verb::Inspect { life: life(), name: "x".to_owned(), on: on() },
      Verb::Pending { life: life() },
      Verb::Prompt {
        life: life(),
        shape: "str".to_owned(),
        message: "hi".to_owned(),
        to: None,
        on: on(),
      },
      Verb::Answer { life: life(), act: id(), line: "yes".to_owned() },
      Verb::Pause { life: life(), act: id() },
      Verb::Wake { life: life(), act: id() },
      Verb::Cancel { life: life(), act: id() },
      Verb::Chain { life: life(), label: Some("work".to_owned()), on: on() },
      Verb::Run { life: life(), word: "1 + 1".to_owned(), on: on() },
    ]
    .into_iter()
    .chain(views())
    .collect()
  }

  /// A node of a view under an id.
  fn node(id: &str, shape: Shape) -> Node {
    Node { id: id.to_owned(), shape }
  }

  /// A tree that holds each node and each tone.
  fn every_node() -> Node {
    let text =
      |id: &str, tone| node(id, Shape::Text { runs: vec![Run { text: id.to_owned(), tone }] });
    let tones = [
      Tone::Text,
      Tone::Muted,
      Tone::Faint,
      Tone::Accent,
      Tone::Success,
      Tone::Warning,
      Tone::Danger,
    ];
    node(
      "root",
      Shape::Box {
        axis: Axis::Column,
        gap: 1,
        border: true,
        children: vec![
          node(
            "line",
            Shape::Box {
              axis: Axis::Row,
              gap: 0,
              border: false,
              children: tones
                .iter()
                .enumerate()
                .map(|(n, &tone)| text(&format!("t{n}"), tone))
                .collect(),
            },
          ),
          node("l", Shape::List { items: vec![text("i", Tone::Text)] }),
          node(
            "tb",
            Shape::Table { header: vec!["a".to_owned()], rows: vec![vec![text("c", Tone::Text)]] },
          ),
          node("in", Shape::Input { value: String::new(), hint: Some("name".to_owned()) }),
          node("b", Shape::Button { label: "go".to_owned() }),
          node("p", Shape::Progress { done: Some(0.5) }),
          node("spin", Shape::Progress { done: None }),
          node("co", Shape::Code { text: "x = 1".to_owned(), language: Some("python".to_owned()) }),
          node("d", Shape::Diff { text: "-a\n+b\n".to_owned() }),
          node("m", Shape::Markdown { text: "# hi".to_owned() }),
        ],
      },
    )
  }

  /// Each frame of a view, with each slot, each gesture and each effect, on a tree of each node.
  fn views() -> Vec<Verb> {
    let viewer = || "v1".to_owned();
    let root = every_node();
    let slots = [
      Slot::Dock { on: "chain1".to_owned() },
      Slot::Sidebar,
      Slot::Row { act: "bash1".to_owned() },
      Slot::Detail { act: "bash1".to_owned() },
      Slot::Mark { act: "bash1".to_owned() },
      Slot::Footer,
      Slot::Screen,
      Slot::Overlay,
    ];
    let effects = [
      Effect::Open { url: "https://example.com".to_owned() },
      Effect::Draft { text: "x".to_owned() },
      Effect::Notice { text: "done".to_owned() },
    ];
    let mounts = slots.into_iter().map(|slot| {
      Verb::ViewMount(Mount {
        viewer: viewer(),
        life: "life.jsonl".to_owned(),
        slot,
        skill: "k".to_owned(),
      })
    });
    let tree = Verb::ViewTree(Tree {
      viewer: viewer(),
      slot: Slot::Dock { on: "chain1".to_owned() },
      version: 2,
      tree: root,
    });
    let events = [Gesture::Press, Gesture::Change, Gesture::Submit].map(|kind| {
      Verb::ViewEvent(ViewEvent {
        viewer: viewer(),
        node: "b".to_owned(),
        kind,
        value: (kind != Gesture::Press).then(|| "typed".to_owned()),
      })
    });
    let effected = effects.map(|effect| Verb::ViewEffect(ViewEffect { viewer: viewer(), effect }));
    mounts
      .chain([tree])
      .chain(events)
      .chain(effected)
      .chain([Verb::ViewDispose(Dispose { viewer: viewer() })])
      .collect()
  }

  /// A peer of a stream: it writes lines and reads frames.
  struct Client<S> {
    lines: BufReader<S>,
  }

  impl<S: AsyncRead + AsyncWrite + Unpin> Client<S> {
    async fn say(&mut self, line: &str) {
      let stream = self.lines.get_mut();
      stream.write_all(line.as_bytes()).await.unwrap();
      stream.write_all(b"\n").await.unwrap();
    }

    async fn ask(&mut self, id: u64, verb: Verb) -> Frame {
      self.say(&serde_json::to_string(&Request { id, verb }).unwrap()).await;
      self.heard().await.expect("the daemon answers each request")
    }

    async fn heard(&mut self) -> Option<Frame> {
      let mut line = String::new();
      let read = self.lines.read_line(&mut line).await.unwrap();
      (read > 0).then(|| serde_json::from_str(&line).unwrap())
    }

    async fn hello(&mut self, protocol: u32) -> Frame {
      self.ask(0, Verb::Hello(Hello { protocol, kind: Kind::Window, name: None })).await
    }
  }

  /// A peer served on a stream in memory, under a role.
  fn served(role: Role) -> Client<tokio::io::DuplexStream> {
    let (ours, theirs) = duplex(1 << 16);
    let who = Who { login: "uael".to_owned(), role };
    tokio::spawn(async move { serve(&Echo, theirs, who).await });
    Client { lines: BufReader::new(ours) }
  }

  fn welcomed(role: Role) -> Frame {
    let peer = Peer { login: "uael".to_owned(), role, kind: Kind::Window, name: None };
    Frame::Reply { id: 0, answer: Answer::Welcome { protocol: PROTOCOL, peer } }
  }

  #[tokio::test]
  async fn a_reader_is_refused_each_verb_that_writes() {
    let mut client = served(Role::Read);
    assert_eq!(client.hello(PROTOCOL).await, welcomed(Role::Read));
    for (id, verb) in (1..).zip(verbs()) {
      let needs = verb.needs();
      match client.ask(id, verb.clone()).await {
        Frame::Reply { id: got, .. } if needs == Role::Read => assert_eq!(got, id),
        Frame::Refused { id: got, error, .. } if needs == Role::Write => {
          assert_eq!((got, error), (Some(id), ProtocolError::Role { needs, has: Role::Read }));
        }
        other => panic!("{verb:?} gave {other:?}"),
      }
    }
  }

  #[test]
  fn each_verb_that_writes_needs_the_role_to_write() {
    let writes: Vec<_> = verbs().into_iter().filter(|verb| verb.needs() == Role::Write).collect();
    let names: Vec<_> =
      writes.iter().map(|verb| serde_json::to_value(verb).unwrap()["verb"].clone()).collect();
    let expected =
      ["open", "pin", "unpin", "prompt", "answer", "pause", "wake", "cancel", "chain", "run"];
    assert_eq!(names, expected.map(|name| json!(name)));
  }

  #[tokio::test]
  async fn a_frame_past_the_bound_closes_the_peer() {
    let mut client = served(Role::Write);
    assert_eq!(client.hello(PROTOCOL).await, welcomed(Role::Write));
    client.say(&"x".repeat(BOUND + 1)).await;
    let large = ProtocolError::Large { bound: BOUND };
    assert_eq!(client.heard().await, Some(Frame::refused(None, large)));
    assert_eq!(client.heard().await, None);
  }

  #[tokio::test]
  async fn a_hello_of_another_version_is_refused_with_both_versions() {
    let mut client = served(Role::Write);
    let error = ProtocolError::Protocol { ours: PROTOCOL, theirs: PROTOCOL + 1 };
    let refused = client.hello(PROTOCOL + 1).await;
    assert_eq!(refused, Frame::refused(Some(0), error));
    let Frame::Refused { why, .. } = refused else { unreachable!() };
    assert!(
      why.contains(&PROTOCOL.to_string()) && why.contains(&(PROTOCOL + 1).to_string()),
      "{why}"
    );
    assert_eq!(client.heard().await, None);
  }

  #[tokio::test]
  async fn a_first_frame_that_is_no_hello_is_refused() {
    let mut client = served(Role::Write);
    assert_eq!(client.ask(4, Verb::List).await, Frame::refused(Some(4), ProtocolError::First));
    assert_eq!(client.heard().await, None);
  }

  #[tokio::test]
  async fn a_frame_that_is_no_request_is_refused_and_the_peer_stays() {
    let mut client = served(Role::Write);
    client.hello(PROTOCOL).await;
    client.say("{\"id\": 1, \"verb\": \"fly\"}").await;
    assert!(matches!(
      client.heard().await,
      Some(Frame::Refused { id: None, error: ProtocolError::Unread { .. }, .. })
    ));
    assert!(matches!(client.ask(2, Verb::List).await, Frame::Reply { id: 2, .. }));
  }

  /// A folder of its own for the transport of one test, short enough for the path of a socket.
  fn folder() -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("fh{}-{n}", std::process::id()))
  }

  #[tokio::test]
  async fn each_verb_makes_a_round_trip_on_the_local_transport() {
    let local = Local::new(folder());
    let mut listener = local.listen().await.unwrap();
    let door = local.clone();
    tokio::spawn(async move {
      let stream = listener.accept().await.unwrap();
      let who = door.identify(&stream).unwrap();
      serve(&Echo, stream, who).await
    });
    let mut client = Client { lines: BufReader::new(local.dial().await.unwrap()) };
    let Frame::Reply { answer: Answer::Welcome { peer, .. }, .. } = client.hello(PROTOCOL).await
    else {
      panic!("no welcome");
    };
    assert_eq!((peer.role, peer.login.is_empty()), (Role::Write, false));
    for (id, verb) in (1..).zip(verbs()) {
      let answer = Answer::Value(serde_json::to_value(&verb).unwrap());
      assert_eq!(client.ask(id, verb).await, Frame::Reply { id, answer });
    }
  }

  #[tokio::test]
  async fn a_second_daemon_cannot_listen_where_one_listens() {
    let local = Local::new(folder());
    let _first = local.listen().await.unwrap();
    assert!(matches!(local.listen().await, Err(furb_host::TransportError::Taken { .. })));
  }

  #[cfg(unix)]
  #[tokio::test]
  async fn a_folder_open_to_others_holds_no_socket() {
    use std::os::unix::fs::PermissionsExt;
    let place = folder();
    std::fs::create_dir_all(&place).unwrap();
    std::fs::set_permissions(&place, std::fs::Permissions::from_mode(0o755)).unwrap();
    let refused = Local::new(&place).listen().await;
    assert!(matches!(refused, Err(furb_host::TransportError::Exposed { .. })));
  }

  #[test]
  fn the_schema_on_disk_is_the_schema_of_the_protocol() {
    let place = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("protocol.schema.json");
    let made = serde_json::to_string_pretty(&schema()).unwrap() + "\n";
    if std::env::var_os("FURB_BLESS").is_some() {
      std::fs::write(&place, &made).unwrap();
    }
    let held = std::fs::read_to_string(&place).unwrap_or_default();
    assert!(held == made, "{} is stale: run FURB_BLESS=1 cargo test -p furb-host", place.display());
  }
}
