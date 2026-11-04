//! The group of the processes of a command, which ends together.

use std::process::Command;

use tokio::process::Child;

/// What a command joins before it starts: on Unix the group that the watch of [`watched`] leads,
/// so the group is watched before the command runs a word, and on Windows nothing.
pub(crate) struct Lead {
  #[cfg(unix)]
  watch: rustix::process::Pid,
  #[cfg(unix)]
  lifeline: std::io::PipeWriter,
}

/// The command set to join a group of its own, which a watch leads on Unix, or on Windows set to
/// open no window.
pub(crate) fn grouped(command: &mut Command) -> std::io::Result<Lead> {
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
/// dies by a signal and runs no drop, ends every process of the group, so no command outlives the
/// life that runs it.
#[cfg(unix)]
fn watched() -> std::io::Result<(rustix::process::Pid, std::io::PipeWriter)> {
  let (read, write) = std::io::pipe()?;
  let mut watch = tokio::process::Command::new("/bin/sh");
  watch.args(["-c", "read -r line || kill -KILL 0"]).process_group(0);
  watch.stdin(read).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
  // Tokio reaps the watch once it ends, on the runtime of the World, with no wait here.
  let _inside = crate::ear::runtime().enter();
  let child = watch.spawn()?;
  let pid = child.id().and_then(|pid| i32::try_from(pid).ok());
  let watch = pid.and_then(rustix::process::Pid::from_raw);
  Ok((watch.ok_or_else(|| std::io::Error::other("the watch of the group is gone"))?, write))
}

/// Every process a command started: on Unix the process group it leads, and on Windows, where a
/// shell of Git does not keep the tree of its processes, a job that holds the command and every
/// process it starts.
///
/// Every process of it ends when the group goes before the command is over: at a stop, or at the
/// end of the ear.
pub(crate) struct Group {
  #[cfg(unix)]
  leader: Option<rustix::process::Pid>,
  /// The end of the pipe that the watch of the group reads. It closes with no line when this
  /// process dies by a signal and runs no drop, and the watch then ends the group.
  #[cfg(unix)]
  lifeline: Option<std::io::PipeWriter>,
  #[cfg(windows)]
  job: Option<win32job::Job>,
}

impl Group {
  /// The group of a command that just started in the group of its lead. On Windows the job takes
  /// the command before its shell has read a word, since the shell starts slower than the job takes
  /// it, and it ends every process it holds when it closes.
  pub(crate) fn of(child: &Child, lead: Lead) -> std::io::Result<Self> {
    let gone = || std::io::Error::other("the command is over already");
    #[cfg(unix)]
    {
      child.id().ok_or_else(gone)?;
      Ok(Group { leader: Some(lead.watch), lifeline: Some(lead.lifeline) })
    }
    #[cfg(windows)]
    {
      let Lead {} = lead;
      let mut ends = win32job::ExtendedLimitInfo::new();
      ends.limit_kill_on_job_close();
      let job = win32job::Job::create_with_limit_info(&ends).map_err(std::io::Error::other)?;
      let process = child.raw_handle().ok_or_else(gone)?;
      job.assign_process(process as isize).map_err(std::io::Error::other)?;
      Ok(Group { job: Some(job) })
    }
  }

  /// Every process of the group, ended, or why a signal did not reach them.
  #[cfg_attr(windows, expect(clippy::unnecessary_wraps, reason = "only a signal of Unix can fail"))]
  pub(super) fn slay(&mut self) -> std::io::Result<()> {
    #[cfg(unix)]
    if let Some(leader) = self.leader {
      match rustix::process::kill_process_group(leader, rustix::process::Signal::KILL) {
        // The group has no process left, so no process is left to end.
        Ok(()) | Err(rustix::io::Errno::SRCH) => {}
        Err(no) => return Err(no.into()),
      }
    }
    #[cfg(windows)]
    self.job.take();
    Ok(())
  }

  /// The group let go once the command is over, and every process of it that outlives the command
  /// left to run.
  pub(super) fn free(&mut self) {
    #[cfg(unix)]
    {
      self.leader.take();
      if let Some(mut lifeline) = self.lifeline.take() {
        // A watch that is gone, as at the end of the group, takes no line and needs none.
        drop(std::io::Write::write_all(&mut lifeline, b"\n"));
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
