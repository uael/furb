//! A session with a server of MCP: its start, which agrees on a version of the protocol, and what the server offers,
//! each list read whole across its pages.

use std::time::Duration;

use serde_json::{Value, json};
use tokio::sync::mpsc;

use super::{
  config::Transport,
  wire::{Failure, Heard, Peer},
};

/// The version of the protocol that furb asks for.
pub(super) const VERSION: &str = "2025-06-18";

/// The most pages of one list that furb reads.
const PAGES: usize = 100;

/// A session with a server: the connection, and what the server said of itself at the start.
pub(super) struct Session {
  pub(super) peer: Peer,
  pub(super) info: Info,
}

/// What a server says of itself at the start: its version, its instructions, and what it can do.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Info {
  pub(super) version: String,
  pub(super) instructions: String,
  pub(super) capabilities: Value,
}

/// What a server offers: its tools, its resources, the templates of its resources, and its prompts, each as the
/// server gives it.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Offered {
  pub(super) tools: Vec<Value>,
  pub(super) resources: Vec<Value>,
  pub(super) templates: Vec<Value>,
  pub(super) prompts: Vec<Value>,
}

/// A list that a server tells has changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Changed {
  Tools,
  Resources,
  Prompts,
}

impl Changed {
  /// The list that a notification of a server says has changed.
  pub(super) fn of(method: &str) -> Option<Self> {
    match method {
      "notifications/tools/list_changed" => Some(Self::Tools),
      "notifications/resources/list_changed" => Some(Self::Resources),
      "notifications/prompts/list_changed" => Some(Self::Prompts),
      _ => None,
    }
  }
}

/// A session with the server of a transport, started within a bound: the connection opened, the version agreed and
/// the start told; and what the connection hears of its own accord.
pub(super) async fn start(
  transport: &Transport,
  token: Option<String>,
  roots: Value,
  bound: Duration,
) -> Result<(Session, mpsc::UnboundedReceiver<Heard>), Failure> {
  let Ok(opened) = tokio::time::timeout(bound, Peer::open(transport, token, roots)).await else {
    return Err(Failure::Late(bound));
  };
  let (peer, heard) = opened?;
  let asked = json!({
    "protocolVersion": VERSION,
    "capabilities": {"roots": {"listChanged": false}},
    "clientInfo": {"name": "furb", "version": env!("CARGO_PKG_VERSION")},
  });
  let got = peer.request("initialize", asked, bound).await?;
  peer.agreed(got["protocolVersion"].as_str().unwrap_or(VERSION));
  peer.notify("notifications/initialized", None).await?;
  let info = Info {
    version: got["serverInfo"]["version"].as_str().unwrap_or_default().to_owned(),
    instructions: got["instructions"].as_str().unwrap_or_default().to_owned(),
    capabilities: got["capabilities"].clone(),
  };
  Ok((Session { peer, info }, heard))
}

/// What a server offers, each list that its capabilities name, within a bound for each request.
pub(super) async fn offered(session: &Session, bound: Duration) -> Result<Offered, Failure> {
  let mut offered = Offered::default();
  for changed in [Changed::Tools, Changed::Resources, Changed::Prompts] {
    relisted(session, changed, &mut offered, bound).await?;
  }
  Ok(offered)
}

/// A list that changed, read again into what a server offers, when its capabilities name it.
pub(super) async fn relisted(
  session: &Session,
  changed: Changed,
  offered: &mut Offered,
  bound: Duration,
) -> Result<(), Failure> {
  let capable = |name: &str| session.info.capabilities.get(name).is_some_and(|one| !one.is_null());
  match changed {
    Changed::Tools if capable("tools") => {
      offered.tools = listed(&session.peer, "tools/list", "tools", bound).await?;
    }
    Changed::Resources if capable("resources") => {
      offered.resources = listed(&session.peer, "resources/list", "resources", bound).await?;
      offered.templates =
        listed(&session.peer, "resources/templates/list", "resourceTemplates", bound)
          .await
          .unwrap_or_default();
    }
    Changed::Prompts if capable("prompts") => {
      offered.prompts = listed(&session.peer, "prompts/list", "prompts", bound).await?;
    }
    _ => {}
  }
  Ok(())
}

/// Every item of a list across its pages: the request again with the cursor of each next page.
async fn listed(
  peer: &Peer,
  method: &str,
  key: &str,
  bound: Duration,
) -> Result<Vec<Value>, Failure> {
  let mut items = Vec::new();
  let mut cursor: Option<String> = None;
  for _ in 0..PAGES {
    let asked = cursor.as_ref().map_or_else(|| json!({}), |cursor| json!({"cursor": cursor}));
    let got = peer.request(method, asked, bound).await?;
    items.extend(got[key].as_array().cloned().unwrap_or_default());
    cursor = got["nextCursor"].as_str().filter(|next| !next.is_empty()).map(str::to_owned);
    if cursor.is_none() {
      break;
    }
  }
  Ok(items)
}
