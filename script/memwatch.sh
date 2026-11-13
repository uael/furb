#!/bin/sh
# The one guard of memory. No step of the work waits on memory: a hook, a build and a test run start at once, since a
# wait on memory stalled them while the machine had room. This watcher runs beside the work instead. While the memory
# is tight, it ends the largest test process: a test binary of cargo, a pytest, a bun test or a capture of the gallery.
# It ends one only when the work can be the cause, since a tight memory that the work does not hold is the memory of
# other work, and each process that it ends is a run lost. The work can be the cause when that process holds
# FURB_WATCH_MIN KB, 4194304 (4 GB), as a test that runs away does, or when the work holds FURB_WATCH_LANE KB, 8388608
# (8 GB). A capture of the gallery holds 1.1 to 1.5 GB, and a test process 274 MB in the mean. The work is each test
# process and each process of this user whose command names a tree of the clone that holds this script, as git
# worktree list gives the trees, so it is the same on a runner of CI, on Linux and on macOS. FURB_WATCH_LANE=0 makes
# the work the cause at each look, as on a runner where nothing else runs.
# A daemon of furb is no part of the work, since no end of a test process frees its memory.
# It ends no other process, so a daemon of furb, a window and a process of the owner stay. It prints each process that
# it ended, with its size.
# The memory is tight on macOS at the warn level of the kernel or above, which sysctl reports, and on Linux when less
# than FURB_WATCH_FREE percent of the memory is available, 20. A machine that gives no reading is never tight.
# FURB_WATCH_POLL: the seconds between two looks, 2. FURB_WATCH_AFTER: the seconds to wait after it ended a process,
# so that the memory and the level of the kernel settle before the next look, 30. With --once, the script looks one
# time and exits.
floor=${FURB_WATCH_FREE:-20}
poll=${FURB_WATCH_POLL:-2}
after=${FURB_WATCH_AFTER:-30}
min=${FURB_WATCH_MIN:-4194304}
cap=${FURB_WATCH_LANE:-8388608}
# Each process that it ends is a line of this log too, as its time in seconds of the epoch, its pid and its command, so
# that script/budget.py names a kill as one of the guard only from this evidence. FURB_WATCH_LOG moves it.
log=${FURB_WATCH_LOG:-${TMPDIR:-/tmp}/furb-memwatch.log}
root=$(cd "$(dirname "$0")/.." && pwd -P)
# tight: whether the memory is tight, with the reading that says so in strain.
tight() {
  # Only macOS gives this key.
  level=$(sysctl -n kern.memorystatus_vm_pressure_level 2>/dev/null)
  if [ -n "$level" ]; then
    strain="pressure level $level"
    [ "$level" -ge 2 ]
  else
    free=$(awk '/^(MemTotal|MemAvailable):/ { v[$1] = $2 } END { print int(100 * v["MemAvailable:"] / v["MemTotal:"]) }' /proc/meminfo 2>/dev/null)
    strain="memory $free% free"
    [ -n "$free" ] && [ "$free" -lt "$floor" ]
  fi
}
# trees: each tree of the clone that holds this script, one on each line. A machine with no git, or a folder that is no
# clone, gives the folder of this script alone.
trees() {
  git -C "$root" worktree list --porcelain 2>/dev/null | sed -n 's/^worktree //p'
  echo "$root"
}
# census: the memory in KB of the work, then the pid, the size in KB and the command of the largest test process of this
# user, with 0 for a pid when there is none. A daemon of furb never counts as that process, and neither does this script
# or the awk that reads the list. A test binary of cargo, a pytest and a bun test count as the program of their command,
# so a linker, a shell or a hook that names one as an argument stays. The test runner of python gives its tests to
# workers whose command is the exec of a line of their input. A command names a tree when the path of the tree stands
# in it as a whole path.
census() {
  ps -axo uid=,pid=,rss=,command= | TREES=$(trees) awk -v uid="$(id -u)" '
    function names(s,   n, i, rest, c) {
      for (n in tree) {
        if (tree[n] == "") continue
        rest = s
        while ((i = index(rest, tree[n])) > 0) {
          c = substr(rest, i + length(tree[n]), 1)
          if (c == "" || c == "/" || c == " ") return 1
          rest = substr(rest, i + 1)
        }
      }
      return 0
    }
    BEGIN { split(ENVIRON["TREES"], tree, "\n") }
    $1 != uid || $4 ~ /(^|\/)awk$/ || index($0, "memwatch") { next }
    {
      command = $0; sub(/^ *[0-9]+ +[0-9]+ +[0-9]+ +/, "", command)
      test = ($4 ~ /\/target\/.*\/deps\/[^\/]+-[0-9a-f]+$/ || $4 ~ /(^|\/)pytest$/ ||
        $4 ~ /(^|\/)python[0-9.]*$/ && ($5 ~ /(^|\/)pytest$/ || $5 == "-m" && $6 == "pytest") || $4 ~ /(^|\/)bun$/ && $5 == "test" ||
        $0 ~ /tui\/script\/(screenshots|stage)\.ts|exec\(eval\(sys\.stdin\.readline/)
      host = $0 ~ /[ \/]furb host( |$)/
      if ((test || names(command)) && !host) lane += $3
      if (test && !host && $3 > size) { size = $3; pid = $2; keep = command }
    }
    END { print lane + 0, pid + 0, size + 0, substr(keep, 1, 150) }'
}
look() {
  tight || { said=; return; }
  read -r lane pid size command <<EOF
$(census)
EOF
  if [ "$pid" = 0 ]; then
    [ -n "$said" ] || echo "memwatch: $strain, and no test process to end"
    said=1
    return
  fi
  if [ "$size" -lt "$min" ] && [ "$lane" -lt "$cap" ]; then
    [ -n "$said" ] || echo "memwatch: $strain, and the work is not the cause (it holds $((lane / 1024)) MB, its largest test process $((size / 1024)) MB): none ended"
    said=1
    return
  fi
  kill -9 "$pid" 2>/dev/null || return
  said= ended=1
  echo "memwatch: $strain, ended pid $pid ($((size / 1024)) MB): $command"
  echo "$(date +%s) $pid $command" >> "$log"
}
while :; do
  ended=
  look
  [ "$1" = --once ] && exit 0
  if [ -n "$ended" ]; then sleep "$after"; else sleep "$poll"; fi
done
