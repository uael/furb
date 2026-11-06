//! A child process of a life, in a group of its own that ends with it, and the task that reaps it.

use std::{process::Output, time::Duration};

use tokio::{
  process::{Child, Command},
  task::JoinHandle,
};

use crate::ear::runtime;
#[cfg(unix)]
use rustix::io;
#[cfg(unix)]
use rustix::process;
use std::io as std_io;
use std::process as std_process;
use tokio::time;

/// What a command joins before it starts: on Unix the group that the watch of [`watched`] leads,
/// so the group is watched before the command runs a word, and on Windows nothing.
struct Lead {
  #[cfg(unix)]
  watch: process::Pid,
  #[cfg(unix)]
  lifeline: std_io::PipeWriter,
}

/// Why a child did not start, or started and was not held, which a refusal says after its name. A
/// failed search of the provider carries it, so it is as open as that failure.
#[derive(Debug, thiserror::Error)]
pub enum Unborn {
  /// The machine did not start it.
  #[error("did not start: {0}")]
  Start(std_io::Error),
  /// Its group could not hold it: the watch of the group did not start, or the group did not take
  /// the child that started, which was then ended, or why it was not.
  #[error(
    "could not be held: {source}{}",
    .ended.as_ref().map(|late| format!(", and it could not be ended: {late}")).unwrap_or_default()
  )]
  Held { source: std_io::Error, ended: Option<std_io::Error> },
}

/// A child of this command, started on the runtime of the World in a group of its own, and that
/// group, or why it did not start or was not held. A watch leads the group, so each process of it
/// ends when the [`Group`] drops, and on Unix when this process dies by a signal and runs no drop,
/// as by SIGKILL. A child that started and that the group could not hold is ended, and reaped.
pub fn spawn(command: &mut Command) -> Result<(Child, Group), Unborn> {
  let _inside = runtime().enter();
  let lead =
    grouped(command.as_std_mut()).map_err(|source| Unborn::Held { source, ended: None })?;
  let mut child = command.spawn().map_err(Unborn::Start)?;
  match Group::of(&child, lead) {
    Ok(group) => Ok((child, group)),
    Err(source) => {
      let ended = child.start_kill().err();
      drop(reaped(child));
      Err(Unborn::Held { source, ended })
    }
  }
}

/// What a child of this command writes, once it ends within `limit`, or none when it does not
/// start or runs past `limit`, when its group ends it.
pub(super) async fn output(command: &mut Command, limit: Duration) -> Option<Output> {
  let (child, _group) = spawn(command).ok()?;
  time::timeout(limit, child.wait_with_output()).await.ok()?.ok()
}

/// The command set to join a group of its own, which a watch leads on Unix, or on Windows set to
/// open no window.
#[cfg_attr(
  windows,
  expect(clippy::unnecessary_wraps, reason = "only a watch of Unix can fail to start")
)]
fn grouped(command: &mut std_process::Command) -> std_io::Result<Lead> {
  #[cfg(unix)]
  {
    use std::os::unix::process::CommandExt;
    let (watch, lifeline) = watched()?;
    command.process_group(watch.as_raw_nonzero().get());
    Ok(Lead { watch, lifeline })
  }
  #[cfg(windows)]
  {
    use std::os::windows::process::CommandExt;
    const NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(NO_WINDOW);
    Ok(Lead {})
  }
}

/// The watch of a new group: a sh that leads it and reads one line from a pipe that this process
/// alone writes. A line lets the group go. The end of the pipe with no line, as when this process
/// dies by a signal and runs no drop, ends every process of the group, so no child outlives the
/// life that runs it. Tokio reaps the watch once it ends, on the runtime that [`spawn`] entered.
#[cfg(unix)]
fn watched() -> std_io::Result<(process::Pid, std_io::PipeWriter)> {
  let (read, write) = std_io::pipe()?;
  let mut watch = Command::new("/bin/sh");
  watch.args(["-c", "read -r line || kill -KILL 0"]).process_group(0);
  watch.stdin(read).stdout(std_process::Stdio::null()).stderr(std_process::Stdio::null());
  let child = watch.spawn()?;
  let pid = child.id().and_then(|pid| i32::try_from(pid).ok());
  let watch = pid.and_then(process::Pid::from_raw);
  Ok((watch.ok_or_else(|| std_io::Error::other("the watch of the group is gone"))?, write))
}

/// Every process a child started: on Unix the process group it leads, and on Windows, where a
/// shell of Git does not keep the tree of its processes, a job that holds the child and every
/// process it starts.
///
/// Every process of it ends when the group goes before it is let go: at a stop, at the drop of the
/// group, or at the end of the part of the World that holds it.
pub struct Group {
  #[cfg(unix)]
  leader: Option<process::Pid>,
  /// The end of the pipe that the watch of the group reads. It closes with no line when this
  /// process dies by a signal and runs no drop, and the watch then ends the group.
  #[cfg(unix)]
  lifeline: Option<std_io::PipeWriter>,
  #[cfg(windows)]
  job: Option<win32job::Job>,
}

impl Group {
  /// The group of a child that just started in the group of its lead. On Windows the job takes
  /// the child before its shell has read a word, since the shell starts slower than the job takes
  /// it, and it ends every process it holds when it closes.
  #[cfg_attr(
    windows,
    expect(
      clippy::needless_pass_by_value,
      reason = "the lead of Unix moves its watch into the group"
    )
  )]
  fn of(child: &Child, lead: Lead) -> std_io::Result<Self> {
    let gone = || std_io::Error::other("the child is over already");
    #[cfg(unix)]
    {
      child.id().ok_or_else(gone)?;
      Ok(Self { leader: Some(lead.watch), lifeline: Some(lead.lifeline) })
    }
    #[cfg(windows)]
    {
      let Lead {} = lead;
      let mut ends = win32job::ExtendedLimitInfo::new();
      ends.limit_kill_on_job_close();
      let job = win32job::Job::create_with_limit_info(&ends).map_err(std_io::Error::other)?;
      let process = child.raw_handle().ok_or_else(gone)?;
      job.assign_process(process as isize).map_err(std_io::Error::other)?;
      Ok(Self { job: Some(job) })
    }
  }

  /// Every process of the group, ended, or why a signal did not reach them.
  pub fn slay(&mut self) -> std_io::Result<()> {
    #[cfg(unix)]
    if let Some(leader) = self.leader {
      match process::kill_process_group(leader, process::Signal::KILL) {
        // The group has no process left, so no process is left to end.
        Ok(()) | Err(io::Errno::SRCH) => {}
        Err(no) => return Err(no.into()),
      }
    }
    // The kill ended the watch with the rest of the group, so a line written to it later finds no
    // reader and raises SIGPIPE, which a host that does not ignore the signal takes as its own end.
    #[cfg(unix)]
    self.lifeline.take();
    #[cfg(windows)]
    self.job.take();
    Ok(())
  }

  /// The group let go once its child is over, and every process of it that outlives the child left
  /// to run.
  pub(super) fn free(&mut self) {
    #[cfg(unix)]
    {
      self.leader.take();
      if let Some(mut lifeline) = self.lifeline.take() {
        // A watch that is gone, as at the end of the group, takes no line and needs none.
        drop(std_io::Write::write_all(&mut lifeline, b"\n"));
      }
    }
    #[cfg(windows)]
    if let Some(job) = self.job.take()
      && job.set_extended_limit_info(&win32job::ExtendedLimitInfo::new()).is_err()
    {
      // A job that cannot be let go keeps its limit, so it ends each process when it drops.
      self.job = Some(job);
    }
  }
}

impl Drop for Group {
  fn drop(&mut self) {
    // A drop has no caller to tell a failure to.
    drop(self.slay());
  }
}

/// A child, awaited by one task of the runtime that gives its code, so it is reaped as it exits,
/// whether its holder still waits for it or dropped before. An error of the wait is given, not read
/// as an exit with no code.
pub(super) fn reaped(mut child: Child) -> JoinHandle<std_io::Result<Option<i32>>> {
  runtime().spawn(async move { child.wait().await.map(coded) })
}

/// The code of an exit, and for a child that a signal ended, 128 and the number of the signal, as
/// a shell gives, so no code says only that the timeout ended the command.
fn coded(status: std_process::ExitStatus) -> Option<i32> {
  #[cfg(unix)]
  {
    use std::os::unix::process::ExitStatusExt;
    status.code().or_else(|| status.signal().map(|signal| 128 + signal))
  }
  #[cfg(not(unix))]
  status.code().map(signalled)
}

/// The code of an exit of the sh of Git for Windows, which a signal ends with the number of the
/// signal times 256, as 128 and that number, as a shell gives; and every other code as it is.
#[cfg_attr(all(unix, not(test)), expect(dead_code, reason = "only Windows and the tests call it"))]
fn signalled(code: i32) -> i32 {
  if code % 256 == 0 && (1..=64).contains(&(code / 256)) { 128 + code / 256 } else { code }
}

#[cfg(test)]
#[path = "child.test.rs"]
mod test;
