//! The host of furb: each life on a thread of its own, which a [`LifeHandle`] reaches from any
//! thread, and the one protocol by which a peer reaches the daemon.
//!
//! [`Life::spawn`] opens a life on a thread that owns its engine, since the engine is not `Send`,
//! with a runtime of tokio of its own. A handle says a [`Command`] to the life and takes its
//! [`Reply`], and reads the [`Event`]s of the life in order, each under its [`Seq`], from any point
//! of its log. The host gives each life the same ears, so each life of a machine opens the same
//! way, whatever client asked for it.
//!
//! [`protocol`] says each frame between a peer and the daemon, [`transport`] how a peer reaches
//! the daemon and who it is, and [`serve`] the door of the daemon to one peer, which checks the
//! role of each verb before a [`Host`] answers it.

mod ears;
mod error;
mod event;
mod life;
pub mod protocol;
mod rebirth;
mod serve;
pub mod transport;
pub mod view;

pub use error::{LifeError, ProtocolError, TransportError};
pub use event::{Event, Seq};
pub use life::{Command, Life, LifeHandle, Reply};
pub use serve::{Host, Outbox, serve};
