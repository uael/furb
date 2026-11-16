//! The proofs of the protocol: the role of each verb, the bound of a frame, the version of a hello,
//! and a round trip of each verb on each transport of the system.
#![expect(clippy::string_slice, reason = "a panic is how a test fails")]
#![expect(clippy::unreachable, reason = "a panic is how a test fails")]
#![cfg(test)]
#![expect(
  clippy::disallowed_methods,
  reason = "the tests block on the runtime that tokio starts for them"
)]

use crate::yard;

use std::collections::BTreeSet;
use yard::Yard;

use std::{
  env, fs, future,
  path::PathBuf,
  process,
  sync::atomic::{AtomicU32, Ordering},
};

use furb_host::{
  Host, Outbox, ProtocolError, fixed,
  protocol::{
    Answer, Axis, BOUND, Config, Dispose, Draft, Effect, Frame, Gesture, Hello, KITS, Kind, Layer,
    Mount, Node, PROTOCOL, Peer, Request, Role, Run, Shape, Slot, Tone, Tree, Verb, ViewEffect,
    ViewEvent, schema,
  },
  serve,
  transport::{Dialer, Identity, Local, Who},
};
use furb_wire::{Block, Exchange, Line, Maker, Mark, Page};
use serde_json::{Value, json};
use tokio::io::{
  AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, DuplexStream, duplex,
};

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
    future::ready(answer)
  }
}

/// One of each verb but the hello.
fn verbs() -> Vec<Verb> {
  let life = || "a.jsonl".to_owned();
  let id = || "prompt1".to_owned();
  let on = || Some("chain1".to_owned());
  vec![
    Verb::List { config: None },
    Verb::Open {
      life: life(),
      cwd: None,
      actor: Some("claude-cli:opus/high".to_owned()),
      roster: None,
      skills: Some(false),
      config: Some("config".to_owned()),
      claude: Some("claude".to_owned()),
      answers: true,
      attach: Some(0),
    },
    Verb::Attach { life: life(), from: 0 },
    Verb::Detach { life: life() },
    Verb::Pin { life: life(), holder: "job".to_owned() },
    Verb::Unpin { life: life(), holder: "job".to_owned() },
    Verb::End { life: life(), cancel: true },
    Verb::Presence { life: life() },
    Verb::Writes { life: life(), draft: Some(Draft { on: Some(id()) }) },
    Verb::Get { life: life(), act: id() },
    Verb::Peek { life: life(), act: id() },
    Verb::Transcript { life: life(), on: on(), of: Some("bash".to_owned()) },
    Verb::Turns { life: life(), on: on() },
    Verb::Program { life: life(), on: on() },
    Verb::Standing { life: life() },
    Verb::Filled { life: life(), act: "reply1".to_owned() },
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
    Verb::Pause { life: life(), act: id(), why: Some("later".to_owned()) },
    Verb::Wake { life: life(), act: id() },
    Verb::Cancel { life: life(), act: id(), why: None },
    Verb::Run { life: life(), word: "1 + 1".to_owned(), on: on() },
    Verb::Handover,
  ]
  .into_iter()
  .chain(acts())
  .chain(views().into_iter().filter(|verb| verb.speaker() != Some(Kind::Skill)))
  .collect()
}

/// The verbs of the acts of an operator and of the models that a window answers.
fn acts() -> Vec<Verb> {
  let life = || "a.jsonl".to_owned();
  let id = || "prompt1".to_owned();
  let on = || Some("chain1".to_owned());
  vec![
    Verb::Close { life: life(), act: id(), value: "2.0".to_owned() },
    Verb::Names { life: life(), on: on() },
    Verb::Roster { life: life(), roster: vec!["claude-cli:sonnet".to_owned()] },
    Verb::Config { life: life(), config: Config::Get },
    Verb::Config {
      life: life(),
      config: Config::Set {
        key: vec!["providers".to_owned(), "codex".to_owned()],
        value: Some(json!(false)),
        layer: Some(Layer::Local),
      },
    },
    Verb::Image { life: life(), name: "image-1.png".to_owned(), data: "iVBORw0KGgo=".to_owned() },
    Verb::Seen { life: life(), about: "chain1".to_owned() },
    Verb::Resolve { life: life(), thread: id() },
    Verb::Queue {
      life: life(),
      entry: "q1".to_owned(),
      shape: "None".to_owned(),
      message: "hi".to_owned(),
      on: on(),
    },
    Verb::Unqueue { life: life(), entry: "q1".to_owned() },
    Verb::Queued { life: life(), on: on() },
    Verb::Desk { life: life(), notices: vec![id()] },
    Verb::Threads { life: life(), on: on(), at: Some(3), read: Some(2) },
    Verb::AsOf { life: life(), at: 3 },
    Verb::Moments { life: life(), on: None },
    Verb::Tracks { life: life() },
    Verb::Answered {
      life: life(),
      ask: 1,
      turn: Some(json!(["assistant", "1", null, null])),
      why: None,
    },
    Verb::Wrote { life: life(), ask: 1, text: "1".to_owned(), thinking: String::new() },
  ]
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
        node(
          "in",
          Shape::Input { value: String::new(), hint: Some("name".to_owned()), inert: false },
        ),
        node("b", Shape::Button { label: "go".to_owned(), inert: false }),
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
    self.hello_as(protocol, Kind::Window, None).await
  }

  async fn hello_as(&mut self, protocol: u32, kind: Kind, name: Option<&str>) -> Frame {
    let name = name.map(str::to_owned);
    self.ask(0, Verb::Hello(Hello { protocol, kind, name, role: None })).await
  }
}

/// A peer served on a stream in memory, under a role.
fn served(role: Role) -> Client<DuplexStream> {
  let (ours, theirs) = duplex(1 << 16);
  let who = Who { operator: "uael".to_owned(), role };
  tokio::spawn(async move { serve(&Echo, theirs, who, fixed(role)).await });
  Client { lines: BufReader::new(ours) }
}

async fn hello_asking(client: &mut Client<DuplexStream>, role: Role) -> Frame {
  let hello = Hello { protocol: PROTOCOL, kind: Kind::Window, name: None, role: Some(role) };
  client.ask(0, Verb::Hello(hello)).await
}

/// The peer of a window as a welcome names it.
fn welcomed(role: Role) -> Peer {
  Peer { operator: "uael".to_owned(), role, kind: Kind::Window, name: None }
}

/// The peer that the welcome of a daemon of this protocol names.
fn peer(frame: Frame) -> Peer {
  match frame {
    Frame::Reply { id: 0, answer: Answer::Welcome { protocol: PROTOCOL, peer, .. } } => peer,
    other => panic!("no welcome: {other:?}"),
  }
}

#[tokio::test]
async fn a_reader_is_refused_each_verb_that_writes() {
  let mut client = served(Role::Read);
  assert_eq!(peer(client.hello(PROTOCOL).await), welcomed(Role::Read));
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

#[tokio::test]
async fn a_peer_that_asked_read_is_refused_each_verb_that_writes_on_a_transport_to_write() {
  let mut client = served(Role::Write);
  assert_eq!(peer(hello_asking(&mut client, Role::Read).await), welcomed(Role::Read));
  let verb = verbs().into_iter().find(|verb| verb.needs() == Role::Write).unwrap();
  match client.ask(1, verb).await {
    Frame::Refused { id, error, .. } => {
      assert_eq!(
        (id, error),
        (Some(1), ProtocolError::Role { needs: Role::Write, has: Role::Read })
      );
    }
    other => panic!("a write verb of a reader gave {other:?}"),
  }
}

#[tokio::test]
async fn a_peer_never_has_more_than_the_role_that_its_transport_proved() {
  let mut client = served(Role::Read);
  assert_eq!(peer(hello_asking(&mut client, Role::Write).await), welcomed(Role::Read));
}

#[test]
fn each_verb_that_writes_needs_the_role_to_write() {
  let writes: Vec<_> = verbs().into_iter().filter(|verb| verb.needs() == Role::Write).collect();
  let names: Vec<_> =
    writes.iter().map(|verb| serde_json::to_value(verb).unwrap()["verb"].clone()).collect();
  let expected = [
    "open",
    "pin",
    "unpin",
    "end",
    "writes",
    "prompt",
    "answer",
    "pause",
    "wake",
    "cancel",
    "run",
    "handover",
    "close",
    "roster",
    "config",
    "image",
    "seen",
    "resolve",
    "queue",
    "unqueue",
    "answered",
    "wrote",
    "view.event",
    "view.event",
    "view.event",
  ];
  assert_eq!(names, expected.map(|name| json!(name)));
}

#[tokio::test]
async fn a_frame_of_a_view_is_refused_to_the_kind_of_peer_that_does_not_say_it() {
  for (kind, name) in [(Kind::Window, None), (Kind::Skill, Some("k"))] {
    let mut client = served(Role::Write);
    client.hello_as(PROTOCOL, kind, name).await;
    for (id, verb) in (1..).zip(views()) {
      let needs = verb.speaker().expect("a frame of a view has the kind that says it");
      match client.ask(id, verb.clone()).await {
        Frame::Reply { id: got, .. } if needs == kind => assert_eq!(got, id),
        Frame::Refused { id: got, error, .. } if needs != kind => {
          assert_eq!((got, error), (Some(id), ProtocolError::Speaker { needs, has: kind }));
        }
        other => panic!("{verb:?} said by {kind:?} gave {other:?}"),
      }
    }
  }
}

#[tokio::test]
async fn a_skill_of_each_version_that_kits_keep_is_welcome_and_says_what_it_said() {
  let kits = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/kits");
  let mut versions: Vec<u32> = fs::read_dir(&kits)
    .unwrap()
    .flatten()
    .filter_map(|one| one.path().file_stem()?.to_str()?.parse().ok())
    .collect();
  versions.sort_unstable();
  assert_eq!(
    versions.first(),
    Some(&KITS),
    "{} holds what a skill of the first version says",
    kits.display()
  );
  for version in versions {
    let said = fs::read_to_string(kits.join(format!("{version}.jsonl"))).unwrap();
    let mut client = served(Role::Write);
    for line in said.lines() {
      client.say(line).await;
      let heard = client.heard().await.expect("the daemon answers each line");
      assert!(matches!(heard, Frame::Reply { .. }), "{version}: {line} gave {heard:?}");
    }
  }
}

#[tokio::test]
async fn a_skill_that_gives_no_name_is_refused_at_its_hello() {
  let mut client = served(Role::Write);
  let refused = client.hello_as(PROTOCOL, Kind::Skill, None).await;
  assert_eq!(refused, Frame::refused(Some(0), ProtocolError::Nameless));
  assert_eq!(client.heard().await, None);
}

#[tokio::test]
async fn a_frame_past_the_bound_closes_the_peer() {
  let mut client = served(Role::Write);
  assert_eq!(peer(client.hello(PROTOCOL).await), welcomed(Role::Write));
  client.say(&"x".repeat(BOUND + 1)).await;
  let large = ProtocolError::Large { bound: BOUND };
  assert_eq!(client.heard().await, Some(Frame::refused(None, large)));
  assert_eq!(client.heard().await, None);
}

/// The digest of the shape of the schema of each version, the version 1 first: the SHA-256 of
/// the schema with no `x-protocol`, no `description` and no `title`, so the words of a doc
/// comment take no new version. Its length is the version, so a new shape takes one.
const SCHEMAS: [&str; PROTOCOL as usize] = [
  "43cd8affad009b9de1c73c355ae4822f1de6149d95d919a5f02171f8c7072604",
  "3d67e6b3dc718819064aae275d0f84cdd16c0f39b92d2a82a945461a19589713",
  "b7fcc6ef6baf1e729e50bc4c81bd3ea06e83a906a66e62709d6aa40607e5a2cf",
  "31c217c979811fbf74d11684749ed59de0ca097e387b59820826b22478069d41",
  "3bf734627359e905bb82818100a056d1000631abb9650b1d8adda626e84fe3b8",
  "ab1e26ad2062fc235028a54bc0b393b108885469e2e7e4a7fd414e325d13b340",
  "ac081ea7adb80dcd8b017c2b13ccc2a191bedc76f48399ea4e4af332e8eebbfb",
  "0eba18a0cee335a618490601a27a6088e88594de3163b840fb38b5b587f774d4",
  "69e3c729852b81913c0d5b655b932779333b094b1d59b10e317f5aed228afeca",
  "d4701b917dda8f385353dc9a29459ac18d730319489ed375481c2631116b1b1d",
  "5b29ad06731843e45d7c3605d43ddcdd15f50836129f7379fabb6c625eeea993",
  "576de143b7304411f1c42b25ae96231bd0047eb769a5565e9630fea6d7d570df",
  "61fea00841a87095db3f78b57c2870504c8bd89d0a9701b6b39391c9a20e8f73",
  "74ff2fe32edc31516b2111ce32773e5c96cd686dcecf2cfbd7913bc402051da9",
  "2799c36fb8e8a61cc3a3bde7817ccdeab3937e2e93959c61f0975cca4d195a41",
  "35b67fc67cff393f0d4d6f877666046e8be15b8cb87c14cc511eb5ea02b6aaa6",
  "30a7854bf1d7f3c8b1ebd382e6f51253450d0b3f09253fb724ab25d665a0de68",
  "7ee1043d9b32ce2ed7063360435d8e0376f0e7a18dcc567106de2520d7c29d3d",
  "b74eeedc5d00c8b06c07b4554af3b54da957dc551f4682be6e3994e14aef5927",
  "006798970b52d20bec8adb5ac17a32c361bde014c3b9ed93a374d24ac447cdd6",
  "128115a53a6b6f067e7037463a0c554c4a7b9011ac553a07632352137437e079",
  "41d12969b0906eab21447fc9eabe4485214225788a693cd574bc43e5794b248a",
  "3a010b848b98d4913b704db8478423c07d3bce91e848031ccc5d865a7b91cf1a",
  "d0fb86a0285276f89fd99b811a32323962f6592b8ce90876d327d5952d889f03",
  "78794bb20925509df72f485e160383e517d59f2e6cd0edc80c9e30217a418f81",
  "d0a96684cbf985d5abd69c71a638b2e1933e4cf016176710dd3b13c50f6aa412",
  "5dfed59683581709d2f2531cd1dfdda2aa9221821bf8aac772f03ccc72e2a383",
  "f1b017fa071552310d3ae5a71aca430acda71ceaf1b90a4d7966e7586ea17d38",
  "ad3dad0dd169cb4dee5c007ff809b9dd9fc03b1b69226620a1529bc3496e7fc5",
  "afa6a63760800b3bf4ef28664998d249ba7e70c74303bd276055fc9fd33f4130",
  "7d3c9dbf00b91ac7e84589fbef485f85aa2f5541ea30e82ca975b0514208d2b9",
  "ea708f4ddfeb6f4f9f9ea7d609c28e6efd38fbea622ecebf32a0c49444ce7cc4",
  "9dfb40bcf853f5a09baf707eaf62dbd84bb296e272d15e14c6dee5d3409f147f",
  "9ae47cfaaabc1ce821ce560227303a7fc4d613e2b4862ad2707e8d39df1eaab1",
  "62b1ed170f89f2c4c615cd82afbd37d02a5451522be0a0477e558667c338927f",
  "d36fe66224a3f790472dc69dcfaf133072497f9023cdf9c484f9e81f7b95f435",
  "1659b7abe689d4cbc388e25dd507ed51a9795b11c815d65ad3e3d3055268fdfc",
  "58167377ba623490d5f65437de43a7353a52ca51dad36fe06428fa3cb41faf4c",
  "b90eed08738f6f20ac18ec01214140cf1c999b6475b9f0413835f2ce76ff0b51",
  "d626048a378ab3ed206077189abeffe41292ca07b4ec938b16da137b3573f746",
  "296e881cd0a6f79be2405c0f9dcdfe2c6231ae0c688abc6bd9709137d4c2de14",
  "5d8fb3175520a1b46c8ad4d372f01c15da47d42c6d9c1f94a739a22b3964dbc6",
  "fa1328d658d84b4328523a29bfe03ffbff85fb8cdada21544e72779dc8ceed61",
  "1b35343807eed5384560f6185623434aee29a927c47d2742fcc2ed3ec6bd3ef0",
  "41e72f0e3eda00c23eca576fb2a06555601b37e7aa518e23e2322837c52699de",
  "e42cca377948beaeaccfd1645a38a8bf4e5033c78427db62e2c1e5421ac679a6",
  "c20655145422c1da586ff8389dbd794952089fc4d6ec6033a7999514d50e3523",
  "bb8b24ccdbb83a68b96b4ddb95b63f83c76b1ec40c64f92ceb40b80822ef3ac7",
  "4bee8ef6def645cda7faede10e4f6a2eaf49556496019f1e61c4558bf4e3757e",
  "37466866cc8d5984a376d0e1ca0f5b65e6d721e4f6ae5d1029f7cd21661d4e54",
  "de17187cd96629902fa3cb5f7f49d9566ec7126e96e924f9dbd55ae08fafaf46",
  "694e7aa039c47a3b0d1afab90f27ceb8e440bced1a7d643bc05b8d3da19d7d88",
  "6e051db77ce483d750781133401dadfb8bef33acbe648b5b98ed13c15e57d8e4",
  "2ca7f65718f5b6c2d199edee9e260660f52430a1bc126b0a77b91f684a77d48a",
  "27c447f5ad204359f89fe9b7d16c01b0b6e325a513ec0c7f5423b5aa85d68617",
  "212c429ba56c6118b33f4603cdbe3caeeca4012b2e68655ed32dda595046fc04",
  "547faea9d9869df5e572e39610e9e328e67954489efd07dc8be4370681977c74",
  "0f66159791c055a2368917f32604959b6d957c9b9d23c111c7742e4a41ce0b6f",
  "2dd3519af3492f49ba3cf80edd452fe9fe587274912652fbc23fa6db0f6eebf0",
  "6bcb0957f39bdecb249607ec7ad99daf7b28d9da9e3f35d1e96fdfe9f36af197",
  "77146731fcd86c6cb59acb7a5157f433658db2a425d5b3236645200d86d2ea60",
  "c0764f8f7d009dfff476ee55dc820d266651c23107c3a93ff2e2870e511707a8",
  "16102ab141d7d716c6432608ad0d253efca3da79643a90226af18891874d58bd",
  "f6f82b860175abbc0738a1cd6d5f22583daefdb8499e32c3344a32db1f6c4f1e",
  "f312e36822df10c0e18a72b70f9608eced55ceaa84d1a66a5d3c6cf36fdf986c",
  "e3d33351ba458b4375794376ed56552e869d0d852d106ae1ca69dead56e5c133",
  "e6cffa54b47466681e401e764de574db44b60cdc986737837ebe565b0f69fe01",
  "aa7f0e80898a22f5faf5792f51f91553c375551aa2385c992f0faaad53e4e7dd",
  "1d7840f32379efa95f15fa852c542b759db7da1b2946dce1fdf776367b018fc8",
  "651e6084ac770305c16e59b1dd6b915c6f4ce6f2a440a20a6e56ecab52ad4ac9",
  "9e86b346336a99d764a72e1789329d89681825523be537bf6707e43110bdeb61",
  "ba29ac1dc10b94988126922e4dea33f20ca950036621f4a7d5665f6400012bf5",
  "181e4160b90a1173dfcd006bdc8f6dc77c6910bcc251cba7b1a39a89191f07fb",
  "6ca077c5eea911301022f9604b4df3ab91457917332aea215ef51468e6d0e778",
  "bd8bfc97b59275d6b38a8ed022408208f6a6a9c26a37c852c9d56f3ac719ab29",
  "bf8da88a23a0f43494ea24928315317d2846492357662815ebb660f4e8dbd294",
];

/// A schema with no word that never crosses the wire: each `description` and `title` of a schema,
/// and no property that bears such a name.
fn shape(schema: &mut Value) {
  match schema {
    Value::Object(map) => {
      map.remove("description");
      map.remove("title");
      for (key, one) in map.iter_mut() {
        match (key.as_str(), one.as_object_mut()) {
          ("properties" | "definitions" | "$defs", Some(named)) => {
            named.values_mut().for_each(shape);
          }
          _ => shape(one),
        }
      }
    }
    Value::Array(all) => all.iter_mut().for_each(shape),
    _ => {}
  }
}

#[test]
fn each_change_of_the_schema_raises_the_protocol() {
  use sha2::{Digest, Sha256};
  let mut made = schema();
  made.as_object_mut().unwrap().remove("x-protocol");
  shape(&mut made);
  let digest = format!("{:x}", Sha256::digest(made.to_string()));
  assert_eq!(
    SCHEMAS.last().copied(),
    Some(digest.as_str()),
    "the schema of the protocol changed: raise PROTOCOL and end SCHEMAS with {digest}"
  );
  let distinct: BTreeSet<_> = SCHEMAS.iter().collect();
  assert_eq!(distinct.len(), SCHEMAS.len(), "each version has a schema of its own");
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
  assert_eq!(
    client.ask(4, Verb::List { config: None }).await,
    Frame::refused(Some(4), ProtocolError::First)
  );
  assert_eq!(client.heard().await, None);
}

#[tokio::test]
async fn a_frame_that_is_no_request_is_refused_and_the_peer_stays() {
  let mut client = served(Role::Write);
  client.hello(PROTOCOL).await;
  client.say("{\"id\": 1, \"verb\": \"fly\"}").await;
  let heard = client.heard().await;
  let Some(Frame::Refused { id, error: ProtocolError::Unread { .. }, why }) = heard else {
    panic!("no refusal: {heard:?}")
  };
  assert_eq!(id, Some(1), "a verb that the daemon does not know is refused to its request");
  assert!(why.contains("fly"), "the refusal names the verb: {why}");
  assert!(matches!(client.ask(2, Verb::List { config: None }).await, Frame::Reply { id: 2, .. }));
}

#[tokio::test]
async fn an_open_that_names_the_old_field_is_refused_with_the_fields_that_it_takes() {
  let mut client = served(Role::Write);
  client.hello(PROTOCOL).await;
  client.say(r#"{"id": 1, "verb": "open", "life": "/a.jsonl", "extensions": false}"#).await;
  let heard = client.heard().await;
  let Some(Frame::Refused { id, why, .. }) = heard else { panic!("no refusal: {heard:?}") };
  assert_eq!(id, Some(1), "the open that names extensions is refused to its request");
  assert!(
    why.contains("extensions") && why.contains("skills"),
    "the refusal names the field that it does not take and the field skills: {why}"
  );
  assert!(matches!(client.ask(2, Verb::List { config: None }).await, Frame::Reply { id: 2, .. }));
}

#[tokio::test]
async fn an_open_that_names_skills_gets_its_skills() {
  let mut client = served(Role::Write);
  client.hello(PROTOCOL).await;
  client.say(r#"{"id": 1, "verb": "open", "life": "/a.jsonl", "skills": false}"#).await;
  let heard = client.heard().await;
  let Some(Frame::Reply { id: 1, answer: Answer::Value(opened) }) = heard else {
    panic!("no answer: {heard:?}")
  };
  assert_eq!(opened["skills"], json!(false), "the open holds the skills that it named");
}

#[test]
fn a_verb_of_a_view_that_names_a_field_that_it_does_not_take_is_refused() {
  let line = |extra: &str| format!(r#"{{"id": 1, "verb": "view.dispose", "viewer": "v"{extra}}}"#);
  assert!(serde_json::from_str::<Request>(&line("")).is_ok());
  let no = serde_json::from_str::<Request>(&line(r#", "viewr": "w""#)).unwrap_err();
  assert!(no.to_string().contains("viewr"), "the refusal names the field: {no}");
}

/// A folder of its own for the transport of one test, short enough for the path of a socket.
fn folder() -> Yard {
  static NEXT: AtomicU32 = AtomicU32::new(0);
  let n = NEXT.fetch_add(1, Ordering::Relaxed);
  Yard::unmade(format!("fh{}-{n}", process::id()))
}

#[tokio::test]
async fn each_verb_makes_a_round_trip_on_the_local_transport() {
  let place = folder();
  let local = Local::new(place.to_path_buf());
  let mut listener = local.listen().await.unwrap();
  let door = local.clone();
  tokio::spawn(async move {
    let stream = listener.accept().await.unwrap();
    let who = door.identify(&stream).await.unwrap();
    let granted = fixed(who.role);
    serve(&Echo, stream, who, granted).await
  });
  let mut client = Client { lines: BufReader::new(local.dial().await.unwrap()) };
  let Frame::Reply { answer: Answer::Welcome { peer, .. }, .. } = client.hello(PROTOCOL).await
  else {
    panic!("no welcome");
  };
  assert_eq!((peer.role, peer.operator.is_empty()), (Role::Write, false));
  for (id, verb) in (1..).zip(verbs()) {
    let answer = Answer::Value(serde_json::to_value(&verb).unwrap());
    assert_eq!(client.ask(id, verb).await, Frame::Reply { id, answer });
  }
}

#[tokio::test]
async fn a_second_daemon_cannot_listen_where_one_listens() {
  let place = folder();
  let local = Local::new(place.to_path_buf());
  let _first = local.listen().await.unwrap();
  assert!(matches!(local.listen().await, Err(furb_host::TransportError::Taken { .. })));
}

#[cfg(unix)]
#[tokio::test]
async fn a_folder_open_to_others_holds_no_socket() {
  use std::os::unix::fs::PermissionsExt;
  let place = folder();
  fs::create_dir_all(&place).unwrap();
  fs::set_permissions(&place, fs::Permissions::from_mode(0o755)).unwrap();
  let refused = Local::new(&place).listen().await;
  assert!(matches!(refused, Err(furb_host::TransportError::Exposed { .. })));
}

#[test]
fn the_schema_on_disk_is_the_schema_of_the_protocol() {
  let place = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("protocol.schema.json");
  let made = serde_json::to_string_pretty(&schema()).unwrap() + "\n";
  if env::var_os("FURB_BLESS").is_some() {
    fs::write(&place, &made).unwrap();
  }
  let held = fs::read_to_string(&place).unwrap_or_default();
  assert!(held == made, "{} is stale: run FURB_BLESS=1 cargo test -p furb-host", place.display());
}

/// A daemon that answers each verb with a page of one exchange, whose one line waits to run.
struct Paged;

impl Host for Paged {
  fn answer(
    &self,
    _: &Peer,
    _: Verb,
    _: &Outbox,
  ) -> impl Future<Output = Result<Answer, ProtocolError>> + Send {
    let line = Line::of(1, "x = 1".to_owned(), Mark::Waits);
    let block = Block { maker: Maker::Operator, lines: vec![line], more: true, thought: None };
    let exchange = Exchange {
      at: 0,
      message: "prompt1".to_owned(),
      said_at: 0,
      by: Maker::Operator,
      text: "hi".to_owned(),
      python: true,
      runs: true,
      block: Some(block),
      said: Vec::new(),
    };
    future::ready(Ok(Answer::Page(Page { exchanges: vec![exchange], ..Page::default() })))
  }
}

#[tokio::test]
async fn a_kit_of_the_first_version_reads_a_page_as_its_version_said_it() {
  for (protocol, kind, name, mark) in
    [(KITS, Kind::Skill, Some("k"), Mark::Ran), (PROTOCOL, Kind::Window, None, Mark::Waits)]
  {
    let (ours, theirs) = duplex(1 << 16);
    let who = Who { operator: "uael".to_owned(), role: Role::Read };
    tokio::spawn(async move { serve(&Paged, theirs, who, fixed(Role::Read)).await });
    let mut client = Client { lines: BufReader::new(ours) };
    client.hello_as(protocol, kind, name).await;
    let Frame::Reply { answer: Answer::Page(mut page), .. } =
      client.ask(1, Verb::List { config: None }).await
    else {
      panic!("the daemon answers a page");
    };
    let block = page.exchanges.remove(0).block.expect("the exchange holds its block");
    assert_eq!(block.lines[0].mark, mark, "{protocol}");
  }
}

/// A daemon that answers each verb with an act whose name holds more than a peer reads in a line.
struct Huge;

impl Host for Huge {
  fn answer(
    &self,
    _: &Peer,
    _: Verb,
    _: &Outbox,
  ) -> impl Future<Output = Result<Answer, ProtocolError>> + Send {
    future::ready(Ok(Answer::Act("x".repeat(BOUND))))
  }
}

#[tokio::test]
async fn an_answer_past_the_bound_goes_as_its_refusal_and_the_peer_stays() {
  let (ours, theirs) = duplex(1 << 16);
  let who = Who { operator: "uael".to_owned(), role: Role::Read };
  tokio::spawn(async move { serve(&Huge, theirs, who, fixed(Role::Read)).await });
  let mut client = Client { lines: BufReader::new(ours) };
  client.hello_as(PROTOCOL, Kind::Window, None).await;
  for id in [1, 2] {
    let refused = Frame::refused(Some(id), ProtocolError::Large { bound: BOUND });
    assert_eq!(client.ask(id, Verb::List { config: None }).await, refused);
  }
}

/// Each value of the protocol of now that a kit of the first version did not know, with what the
/// daemon does for that kit: `Frame::told_to` maps each value that a kit can read, and the rest
/// never reaches a kit of that version.
const NEW: [(&str, &str); 6] = [
  ("Mark|is=waits", "mapped: a kit before 66 reads ran"),
  ("Reply|is=withdrawn", "mapped: a kit before 54 reads refused"),
  ("Saying|resolved", "mapped: a kit before 68 reads answer"),
  ("Frame|frame=granted", "mapped: a kit before 60 hears no change of its role"),
  ("Frame|frame=served", "only a window hears which kits serve views"),
  ("Answer|answer=tracks", "only the verb tracks of 56 gives it, which no kit of 44 says"),
];

#[test]
fn each_value_that_a_kit_of_the_first_version_did_not_know_is_mapped_or_never_reaches_it() {
  let kits = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/kits");
  let first: Value =
    serde_json::from_str(&fs::read_to_string(kits.join(format!("{KITS}.schema.json"))).unwrap())
      .unwrap();
  let now = serde_json::to_value(schema()).unwrap();
  // What a kit says is the golden of its lines, which each later daemon takes: only what the daemon
  // says is read here.
  let read = |schema: &Value| -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    for (name, one) in schema["definitions"].as_object().unwrap() {
      if name != "Request" {
        flat(name, one, &mut paths);
      }
    }
    paths
  };
  let (then, now) = (read(&first), read(&now));
  let gone: Vec<&String> = then.difference(&now).collect();
  assert_eq!(gone, Vec::<&String>::new(), "a part that a kit of {KITS} reads is gone");
  let mapped: BTreeSet<&str> = NEW.iter().map(|(path, _)| *path).collect();
  let new: Vec<&String> = now
    .difference(&then)
    .filter(|path| {
      let (owner, last) = path.rsplit_once(['|', '.']).unwrap_or((path, ""));
      then.contains(owner) && path[owner.len()..].starts_with('|') && !last.is_empty()
    })
    .filter(|path| !mapped.contains(path.as_str()))
    .collect();
  assert_eq!(new, Vec::<&String>::new(), "a value of now has no line in NEW nor in Frame::told_to");
}

/// The keys whose constant names the variant of a union.
const TAGGED: [&str; 7] = ["frame", "answer", "event", "is", "kind", "slot", "effect"];

/// Each path of a schema under a name: each property, as `.key`, and each variant of a union, as
/// `|key=value` by the constant of its tag, `|key` for an object of one key, or `|value` for a
/// string of a list of strings.
fn flat(at: &str, one: &Value, paths: &mut BTreeSet<String>) {
  paths.insert(at.to_owned());
  if let Some(properties) = one["properties"].as_object() {
    for (key, part) in properties {
      flat(&format!("{at}.{key}"), part, paths);
    }
  }
  for union in ["oneOf", "anyOf"] {
    for part in one[union].as_array().into_iter().flatten() {
      let tag = TAGGED.iter().find_map(|key| {
        let value = part["properties"][key]["const"].as_str()?;
        Some(format!("{key}={value}"))
      });
      let single = part["properties"].as_object().filter(|keys| keys.len() == 1);
      let tag = tag.or_else(|| single.and_then(|keys| keys.keys().next().cloned()));
      match tag {
        Some(tag) => flat(&format!("{at}|{tag}"), part, paths),
        None => {
          for value in part["enum"].as_array().into_iter().flatten().filter_map(Value::as_str) {
            paths.insert(format!("{at}|{value}"));
          }
        }
      }
    }
  }
}

/// A frame of an event says the event that the log holds, in the same bytes as when the frame held
/// a copy of it, so no window reads another frame.
#[test]
fn a_frame_of_an_event_says_the_same_bytes() {
  let fact =
    furb_host::Event::Fact(serde_json::json!(["done", "run1", "run1", 7, {"a": [1.5, null]}]));
  let frame = Frame::Event { life: "/l.jsonl".to_owned(), seq: 3, event: fact.into() };
  // The bytes of the frame when it held a copy of its event.
  let held = r#"{"frame":"event","life":"/l.jsonl","seq":3,"event":{"event":"fact","data":["done","run1","run1",7,{"a":[1.5,null]}]}}"#;
  assert_eq!(serde_json::to_string(&frame).unwrap(), held);
}
