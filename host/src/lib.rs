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
//! the daemon and who it is, [`access`] the role of each login of the network, and [`serve`] the
//! door of the daemon to one peer, which checks the role of each verb before a [`Host`] answers it.
//!
//! [`run`] serves the daemon of a machine, which holds each life by the path of its record for as
//! long as a window or a pin holds it, and [`upgrade`] puts the daemon of a new build in its place.
//! The daemon starts the process of each skill that a life plays, which reads where it dials in
//! [`processes`], and watches each life from outside it: a life that holds an open act and says
//! no fact for [`STALL`] is told to its windows and to its root, and a chain whose rung or prompt
//! to a model has nothing that works beside it for [`HUNG`] is told to its windows and to itself.

pub mod access;
mod answers;
mod changes;
mod daemon;
mod ears;
mod error;
mod event;
mod holders;
mod life;
pub mod processes;
pub mod protocol;
mod queue;
mod rebirth;
mod records;
mod serve;
mod stall;
pub mod transport;
mod views;

pub use daemon::{GRACE, Settings, Tailnet, run, upgrade};
pub use error::{AccessError, DaemonError, LifeError, ProtocolError, TransportError};
pub use event::{Event, Seq};
pub use life::{Command, Life, LifeHandle, Reply};
pub use queue::FollowUp;
pub use serve::{Host, Outbox, serve};
pub use stall::{HUNG, STALL};
