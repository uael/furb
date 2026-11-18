//! The fake claude command line of the tests, which answers from a script that `fake-claude.sh`
//! says, in a yard of each test. The tests of the claude command line beside its module and the
//! suite of the crate share it: each includes it as a module, and gives it `Base`, the yard of the
//! tests, and `pause`.

#![allow(dead_code, reason = "each side takes the part of the fake that it needs")]

use std::{
  fs, io,
  os::unix::fs::PermissionsExt,
  path::PathBuf,
  process::{self as std_process, Command, Stdio},
  sync::{
    Arc, Mutex, PoisonError, Weak,
    atomic::{AtomicUsize, Ordering},
  },
  time::Duration,
};

use rustix::process;

use super::{Base, pause};
use furb::world::claude::{Account, Claude, Switch};

/// The fake command line, which the tests of the command line of furb run too.
pub(crate) const FAKE: &str = include_str!("../../src/world/provider/claude/fake-claude.sh");

/// A yard of the test that holds the fake command line and what it wrote.
pub(crate) struct Yard {
  pub(crate) at: Base,
  /// The folder of the fake that the yard links to, which goes with the last yard that holds it.
  _fake: Arc<Fake>,
}

impl Yard {
  pub(crate) fn new(name: &str) -> Self {
    // The name holds the id of the process, as every yard of the tests of the crate does: two runs
    // of the suite at once, as a hook and a check in two checkouts, would otherwise remove the
    // files of each other.
    let at = Base::new(format!("furb-claude-{name}-{}", std_process::id()));
    let fake = ran();
    fs::hard_link(&fake.bin, at.join("claude")).expect("the fake is linked");
    Self { at, _fake: fake }
  }

  /// The fake as the command line, with a stall of these milliseconds.
  pub(crate) fn claude(&self, stall: u64) -> Claude {
    Claude::with(Some(self.at.join("claude")), Some(Duration::from_millis(stall)))
  }

  /// The fake as the command line that runs as these accounts.
  pub(crate) fn claude_as(&self, accounts: Vec<Account>, switch: Switch) -> Claude {
    let stall = Some(Duration::from_millis(10_000));
    Claude::with_accounts(Some(self.at.join("claude")), Vec::new(), accounts, switch, stall)
  }

  /// The token each process the fake was started with, in the order they started.
  pub(crate) fn tokens(&self) -> Vec<String> {
    let token = |pid: &String| fs::read_to_string(self.at.join(format!("token.{pid}")));
    self.pids().iter().map(|pid| token(pid).unwrap_or_default()).collect()
  }

  /// The id of each process the fake was, in the order they started.
  pub(crate) fn pids(&self) -> Vec<String> {
    let pids = fs::read_to_string(self.at.join("pids")).unwrap_or_default();
    pids.lines().map(str::to_owned).collect()
  }

  /// The words a process was started with.
  pub(crate) fn args(&self, pid: &str) -> Vec<String> {
    let args = fs::read_to_string(self.at.join(format!("args.{pid}"))).unwrap_or_default();
    args.split_terminator('\0').map(str::to_owned).collect()
  }

  /// Each line of input the fake read.
  pub(crate) fn heard(&self) -> Vec<String> {
    let heard = fs::read_to_string(self.at.join("in")).unwrap_or_default();
    heard.lines().map(str::to_owned).collect()
  }

  /// A command that starts the fake beside it, as `cswap` starts claude, and that fails and says
  /// that it found no claude while the fake is away, as `cswap` says while an update replaces
  /// claude.
  pub(crate) fn prefix(&self) -> PathBuf {
    let prefix = self.at.join("prefix");
    let script = "#!/bin/sh\nhere=$(dirname \"$0\")\n[ -x \"$here/claude\" ] || \
      { echo \"Error: 'claude' was not found on PATH.\" >&2; exit 1; }\nexec \"$here/claude\" \"$@\"\n";
    fs::write(&prefix, script).expect("the prefix is written");
    fs::set_permissions(&prefix, fs::Permissions::from_mode(0o755)).expect("the prefix runs");
    prefix
  }

  /// The fake taken away, as an update of claude takes its program away for a moment.
  pub(crate) fn away(&self) {
    fs::rename(self.at.join("claude"), self.at.join("away")).expect("the fake goes away");
  }

  /// The fake back where it was.
  pub(crate) fn back(&self) {
    fs::rename(self.at.join("away"), self.at.join("claude")).expect("the fake comes back");
  }
}

/// The folder of the fake and the fake in it, which the yards that stand at once share.
struct Fake {
  _at: Base,
  bin: PathBuf,
}

/// The fake, written once for the yards that stand at once, after it ran once, in a folder that the
/// last of them removes. The system examines a new program the first time it runs it. On macOS
/// this takes more than a tenth of a second when the machine is idle, and much more when many tests
/// start new programs at the same time. The result holds for the file, so each yard links to this
/// file, and no clock of a test includes that examination.
fn ran() -> Arc<Fake> {
  static RAN: Mutex<Weak<Fake>> = Mutex::new(Weak::new());
  static MADE: AtomicUsize = AtomicUsize::new(0);
  let mut held = RAN.lock().unwrap_or_else(PoisonError::into_inner);
  if let Some(fake) = held.upgrade() {
    return fake;
  }
  // Each folder has a name of its own, so the removal of an earlier one, which its last yard runs,
  // meets none later.
  let n = MADE.fetch_add(1, Ordering::Relaxed);
  let fake = {
    let at = Base::new(format!("furb-claude-fake-{}-{n}", std_process::id()));
    let bin = at.join("claude");
    fs::write(&bin, FAKE).expect("the fake is written");
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).expect("the fake runs");
    // A new program is busy for a moment while the fork of another thread holds it open to write.
    // The crate starts its programs in the same way.
    let mut tries = 0;
    let status = loop {
      match Command::new(&bin).stdin(Stdio::null()).stdout(Stdio::null()).status() {
        Err(no) if no.kind() == io::ErrorKind::ExecutableFileBusy && tries < 50 => tries += 1,
        got => break got,
      }
      pause(Duration::from_millis(10));
    };
    assert!(status.expect("the fake starts").success(), "the fake ran once");
    Arc::new(Fake { _at: at, bin })
  };
  *held = Arc::downgrade(&fake);
  fake
}

/// The word after a flag among the words of a process.
pub(crate) fn after(args: &[String], flag: &str) -> Option<String> {
  args.iter().position(|one| one == flag).and_then(|at| args.get(at + 1).cloned())
}

/// Whether the process of an id is gone, within a time, as a process ends a moment after it is
/// killed. A process that ended and that nothing reaped yet is gone too.
pub(crate) fn gone(pid: &str) -> bool {
  let Some(pid) = pid.parse().ok().and_then(process::Pid::from_raw) else { return false };
  for _ in 0..100 {
    let there = process::test_kill_process(pid).is_ok();
    let status =
      fs::read_to_string(format!("/proc/{}/status", pid.as_raw_nonzero())).unwrap_or_default();
    if !there || status.contains("State:\tZ") {
      return true;
    }
    pause(Duration::from_millis(20));
  }
  false
}
