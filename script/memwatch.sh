#!/bin/sh
# The one guard of memory. No step of the work waits on memory: a hook, a build and a test run start at once, since a
# wait on memory stalled them while the machine had room. This watcher runs beside the work instead. While the memory
# is tight, it ends the largest test process: a test binary of cargo, a pytest, a bun test or a capture of the gallery.
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
# largest: the pid, the size in KB and the command of the largest test process of this user, or nothing. A daemon of
# furb never counts, and neither does this script or the awk that reads the list. The test runner of python gives its
# tests to workers whose command is the exec of a line of their input.
largest() {
  ps -axo uid=,pid=,rss=,command= | awk -v uid="$(id -u)" '
    $1 != uid || $4 ~ /(^|\/)awk$/ || index($0, "memwatch") || $0 ~ /[ \/]furb host( |$)/ { next }
    $0 ~ /\/target\/[^ ]*\/deps\/[^ \/]+-[0-9a-f]+( |$)|[ \/]pytest( |$)|bun test( |$)|tui\/script\/(screenshots|stage)\.ts|exec\(eval\(sys\.stdin\.readline/ && $3 > size {
      size = $3; pid = $2; command = $0; sub(/^ *[0-9]+ +[0-9]+ +[0-9]+ +/, "", command)
    }
    END { if (pid) print pid, size, substr(command, 1, 150) }'
}
look() {
  tight || { said=; return; }
  line=$(largest)
  if [ -z "$line" ]; then
    [ -n "$said" ] || echo "memwatch: $strain, and no test process to end"
    said=1
    return
  fi
  pid=${line%% *} rest=${line#* }
  size=${rest%% *} command=${rest#* }
  kill -9 "$pid" 2>/dev/null || return
  said= ended=1
  echo "memwatch: $strain, ended pid $pid ($((size / 1024)) MB): $command"
}
while :; do
  ended=
  look
  [ "$1" = --once ] && exit 0
  if [ -n "$ended" ]; then sleep "$after"; else sleep "$poll"; fi
done
