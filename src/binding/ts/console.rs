//! The end of a process that the console of Windows asks for, which a host hears before the process ends.
use napi::bindgen_prelude::Function;
use napi::threadsafe_function::ThreadsafeFunction;
use napi_derive::napi;
use std::sync::Mutex;

/// The callback of the host, called on its own thread with the name of the event.
type Heard = ThreadsafeFunction<String, (), String, napi::Status, false, true>;
static HEARD: Mutex<Option<Heard>> = Mutex::new(None);

/// Call back when the console of Windows ends this process: at Ctrl+Break, at the close of the console, at a logoff
/// and at a shutdown, with the name of the event: `break`, `close`, `logoff` or `shutdown`. The system holds the
/// process until the callback ends it, or, at a close, a logoff or a shutdown, until the system's own limit. A later
/// callback replaces an earlier one. Ctrl+C stays SIGINT, which the host hears as a signal. A system that is not
/// Windows has no such console, and gives this callback no event.
#[napi(
  ts_args_type = "callback: (event: \"break\" | \"close\" | \"logoff\" | \"shutdown\") => void"
)]
pub fn on_console_end(callback: Function<String, ()>) -> napi::Result<()> {
  // A weak callback keeps no host alive that has nothing else to do.
  let heard =
    callback.build_threadsafe_function().callee_handled::<false>().weak::<true>().build()?;
  *HEARD.lock().map_err(|_| napi::Error::from_reason("The console callback is poisoned."))? =
    Some(heard);
  #[cfg(windows)]
  windows::listen()?;
  Ok(())
}

#[cfg(windows)]
mod windows {
  use super::HEARD;
  use napi::threadsafe_function::ThreadsafeFunctionCallMode;
  use std::sync::OnceLock;
  use windows_sys::Win32::System::Console::{
    CTRL_BREAK_EVENT, CTRL_CLOSE_EVENT, CTRL_LOGOFF_EVENT, CTRL_SHUTDOWN_EVENT,
    SetConsoleCtrlHandler,
  };
  use windows_sys::core::BOOL;

  /// Add the handler of this module to the handlers of the process, once.
  pub fn listen() -> napi::Result<()> {
    static ADDED: OnceLock<bool> = OnceLock::new();
    // The system calls the handlers from the last added to the first, so this one comes before those of the
    // runtime, which end the process at once.
    if *ADDED.get_or_init(|| unsafe { SetConsoleCtrlHandler(Some(heard), 1) } != 0) {
      Ok(())
    } else {
      Err(napi::Error::from_reason(format!(
        "The console handler was not added: {}",
        std::io::Error::last_os_error()
      )))
    }
  }

  /// The system calls this on a thread of its own. It gives the event to the host and keeps the thread, since the
  /// system ends the process once a handler returns. An event with no callback, and Ctrl+C, go on to the next handler.
  unsafe extern "system" fn heard(event: u32) -> BOOL {
    let name = match event {
      CTRL_BREAK_EVENT => "break",
      CTRL_CLOSE_EVENT => "close",
      CTRL_LOGOFF_EVENT => "logoff",
      CTRL_SHUTDOWN_EVENT => "shutdown",
      _ => return 0,
    };
    let called = HEARD.lock().ok().and_then(|slot| {
      slot
        .as_ref()
        .map(|callback| callback.call(name.to_owned(), ThreadsafeFunctionCallMode::NonBlocking))
    });
    if called != Some(napi::Status::Ok) {
      return 0;
    }
    loop {
      std::thread::park();
    }
  }
}
