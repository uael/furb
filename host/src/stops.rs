//! The asks of the system that the daemon stop: SIGTERM and SIGINT, or on Windows a Ctrl+C, a
//! Ctrl+Break and the close of its console. At the first ask, the daemon ends each life with its
//! image and then ends; a second ask ends it at once. A kill that no process can catch, as
//! SIGKILL, leaves each life at its last image and the facts that its record took after it.

use std::io;

#[cfg(unix)]
use tokio::signal::unix::{Signal, SignalKind, signal};
#[cfg(windows)]
use tokio::signal::windows::{CtrlBreak, CtrlC, CtrlClose, ctrl_break, ctrl_c, ctrl_close};

/// The asks to stop that the daemon hears from its start.
pub(crate) struct Stops {
  #[cfg(unix)]
  heard: [Signal; 2],
  #[cfg(windows)]
  heard: (CtrlC, CtrlBreak, CtrlClose),
}

impl Stops {
  /// The asks to stop, heard from now on.
  pub(crate) fn heard() -> io::Result<Self> {
    #[cfg(unix)]
    let heard = [signal(SignalKind::terminate())?, signal(SignalKind::interrupt())?];
    #[cfg(windows)]
    let heard = (ctrl_c()?, ctrl_break()?, ctrl_close()?);
    Ok(Self { heard })
  }

  /// The next ask to stop.
  pub(crate) async fn next(&mut self) {
    #[cfg(unix)]
    {
      let [term, int] = &mut self.heard;
      tokio::select! {
        _ = term.recv() => {}
        _ = int.recv() => {}
      }
    }
    #[cfg(windows)]
    {
      let (c, brk, close) = &mut self.heard;
      tokio::select! {
        _ = c.recv() => {}
        _ = brk.recv() => {}
        _ = close.recv() => {}
      }
    }
  }
}
