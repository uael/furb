#!/bin/sh
# Runs a heavy step, as a build, a test run or a capture, only while the machine has the memory for it. With no cap the
# steps of the worktrees add up: 28 rustc processes took 13.8 GB, and twice the swap and the compressor filled and the
# machine died. A step takes one of the slots that all the worktrees share, a folder of FURB_HEAVY_DIR that holds the pid
# of its holder, and takes it only while the memory is free, the swap does not grow and the memory is not tight: on macOS,
# while the pressure level of the kernel is normal, its own signal before the compressor and the swap fill; on Linux,
# while the swap in use stays under a share of its total. macOS grows the total of the swap just above its use and keeps
# the pages of idle processes in it, so the share means nothing on macOS. With no command, the script
# prints the memory that it reads and exits 0 when a step can start, or 75 when it cannot: the desk of the conductor
# reads the same numbers so.
# FURB_HEAVY_SLOTS: the slots, one for each 9 GB of memory and at least 1. FURB_HEAVY_FREE: the percent of the memory that
# stays free, 20. FURB_HEAVY_SWAP_MB: the MB that the swap may grow by in the last minute, 256. FURB_HEAVY_WAIT: the
# seconds to wait for a slot, 1200, and then exit 75. FURB_HEAVY_POLL: the seconds between two looks, 1.
# FURB_HEAVY_SWAP_SHARE: on Linux, the percent of the total of the swap that may be in use, 80.
floor=${FURB_HEAVY_FREE:-20}
share=${FURB_HEAVY_SWAP_SHARE:-80}
growth=${FURB_HEAVY_SWAP_MB:-256}
limit=${FURB_HEAVY_WAIT:-1200}
poll=${FURB_HEAVY_POLL:-1}
# reading: the percent of free memory and the MB of swap in use, in free and swap, and whether the memory is tight, in
# tight, which strain says. macOS gives them through memory_pressure and sysctl, and Linux through /proc/meminfo.
reading() {
  if command -v memory_pressure >/dev/null 2>&1; then
    free=$(memory_pressure -Q | sed -n 's/.*free percentage: \([0-9]*\)%.*/\1/p')
    swap=$(sysctl -n vm.swapusage | sed -n 's/.*used = \([0-9]*\).*/\1/p')
    # A level that sysctl does not give holds, as a level of critical.
    level=$(sysctl -n kern.memorystatus_vm_pressure_level 2>/dev/null)
    level=${level:-4}
    tight=$((level >= 2)) strain="pressure level $level"
  else
    read -r free swap total <<EOF
$(awk '/^(MemTotal|MemAvailable|SwapTotal|SwapFree):/ { v[$1] = $2 }
  END { print int(100 * v["MemAvailable:"] / v["MemTotal:"]), int((v["SwapTotal:"] - v["SwapFree:"]) / 1024), int(v["SwapTotal:"] / 1024) }' /proc/meminfo)
EOF
    used=$((total > 0 ? 100 * swap / total : 0))
    tight=$((used >= share)) strain="swap $used% of its total"
  fi
  free=${free:-0} swap=${swap:-0}
}
# A step that this script started holds its slot already, so a step inside it takes no second one. A runner of CI runs
# one job alone, so its steps take no slot either.
[ -z "${FURB_HEAVY_SLOT:-}" ] && [ "${CI:-}" != true ] || [ $# -eq 0 ] || exec "$@"
dir=${FURB_HEAVY_DIR:-/tmp/furb-heavy-$(id -u)}
mkdir -p "$dir" || exit 1
# room: whether the memory holds one more step. Each look of each waiter and of the desk adds the swap to $dir/swap,
# which keeps the looks of the last minute, so a step waits while the swap grew in that minute, however slowly.
room() {
  reading
  now=$(date +%s)
  { echo "$now $swap"; cat "$dir/swap" 2>/dev/null; } | awk -v now="$now" '$1 > now - 60' >"$dir/swap.$$"
  grew=$((swap - $(awk 'NR == 1 || $2 < low { low = $2 } END { print low }' "$dir/swap.$$")))
  mv "$dir/swap.$$" "$dir/swap"
  [ "$free" -ge "$floor" ] && [ "$grew" -le "$growth" ] && [ "$tight" -eq 0 ]
}
said() { echo "memory $free% free, swap $swap MB in use, grown $grew MB in the last minute, $strain"; }
room
sleep "$poll"
if [ $# -eq 0 ]; then
  room && { said; exit 0; }
  said
  exit 75
fi
memory=$(sysctl -n hw.memsize 2>/dev/null || awk '/MemTotal/ { print $2 * 1024 }' /proc/meminfo)
slots=${FURB_HEAVY_SLOTS:-$((memory / 9663676416))}
[ "$slots" -ge 1 ] || slots=1

# A holder writes its pid at once, so a folder that nobody touched for a minute belongs to a process that stopped.
old() { [ -n "$(find "$1" -prune -mmin +1 2>/dev/null)" ]; }
stale() {
  holder=$(cat "$1/pid" 2>/dev/null)
  # A pid that the system gave to another process since is no holder: only a heavy.sh process holds a slot, and ps names
  # no process for a pid that ended.
  if [ -n "$holder" ]; then ! ps -o command= -p "$holder" 2>/dev/null | grep -q heavy.sh; else old "$1"; fi
}

# The look at the slots and the memory runs under a guard that stands for milliseconds, so two waiters never free one
# stale slot twice, and a step starts only on the memory that it sees now.
take() {
  got=
  until mkdir "$dir/guard" 2>/dev/null; do
    if old "$dir/guard"; then rmdir "$dir/guard" 2>/dev/null; fi
    sleep 0.2
  done
  n=1
  room || n=$((slots + 1))
  while [ -z "$got" ] && [ "$n" -le "$slots" ]; do
    s=$dir/slot$n
    if mkdir "$s" 2>/dev/null; then got=$s
    elif stale "$s" && rm -rf "$s" && mkdir "$s" 2>/dev/null; then got=$s
    fi
    n=$((n + 1))
  done
  [ -z "$got" ] || echo $$ >"$got/pid"
  rmdir "$dir/guard"
}

start=$(date +%s)
# The first wait logs at once, so a step that waits shows it before its first 30 seconds.
told=$((start - 30))
take
while [ -z "$got" ]; do
  now=$(date +%s)
  # A wait whose caller ended serves no one, and adopted by init it would run to its limit: it ends with its caller.
  if [ $(ps -o ppid= -p $$) != "$PPID" ]; then
    echo "heavy: the caller of the wait ended, so the wait ends" >&2
    exit 143
  fi
  if [ $((now - start)) -ge "$limit" ]; then
    echo "heavy: no step started in ${limit}s, with $(said): run it again" >&2
    exit 75
  fi
  if [ $((now - told)) -ge 30 ]; then
    echo "heavy: waiting $((now - start))s, with $(said), and the $slots slots held by pids $(cat "$dir"/slot*/pid 2>/dev/null | tr '\n' ' ')" >&2
    told=$now
  fi
  sleep "$poll"
  take
done
# A holder that a signal ends frees its slot, and one that nothing can catch leaves a pid that stale finds dead.
trap 'rm -rf "$got"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM HUP
FURB_HEAVY_SLOT=$got "$@"
exit $?
