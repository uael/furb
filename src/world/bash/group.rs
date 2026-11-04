//! The group of the processes of a command, which ends together.

use std::process::Command;

use tokio::process::Child;

/// The command leads a process group of its own, or on Windows opens no window.
pub(crate) fn grouped(command: &mut Command) {
  #[cfg(unix)]
  {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
  }
  #[cfg(windows)]
  {
    use std::os::windows::process::CommandExt;
    const NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(NO_WINDOW);
  }
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
  #[cfg(windows)]
  job: Option<win32job::Job>,
}

impl Group {
  /// The group of a command that just started. On Windows the job takes the command before its
  /// shell has read a word, since the shell starts slower than the job takes it, and it ends every
  /// process it holds when it closes.
  pub(crate) fn of(child: &Child) -> std::io::Result<Self> {
    let gone = || std::io::Error::other("the command is over already");
    #[cfg(unix)]
    {
      let pid = child.id().and_then(|pid| i32::try_from(pid).ok());
      let leader = pid.and_then(rustix::process::Pid::from_raw).ok_or_else(gone)?;
      Ok(Group { leader: Some(leader) })
    }
    #[cfg(windows)]
    {
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
    self.leader.take();
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
