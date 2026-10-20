#!/bin/sh
# The claude command line of the tests, which answers from a script. It says the stream of the real one: each block
# streams in parts and settles whole, and the result says the usage and the running cost. It writes, beside itself,
# the words of each process it was, the id of each, each line of input it read, and how many turns it answered in
# all. The word of a turn is the JSON text in the file word.N beside it, for the Nth turn of all, and else a close of
# "reply n", for the nth turn of its process. A line that holds FAIL ends it with a failure, a line that holds WAIT
# gets no answer, a line that holds LONG gets a line longer than a reader takes, a line that holds ERROR gets a result
# that says an error, a line that holds BARE gets a result with no block before it, a line that holds THINK gets a
# thought before the text, and a line that holds LOUD closes the stdout, writes more on the stderr than its pipe holds,
# and ends it with a failure, and a line that holds GAP streams its text with its first part left out, and settles the
# text whole. A line that holds MUTE closes the stdout and lives on, and a line that holds ORPHAN starts a process that
# holds the stdout and the stderr open, writes the id of that process in the file orphan beside it, says so on the
# stderr, and ends it with a failure. When the file loud stands beside it, it writes more on the stderr than its pipe
# holds before it reads any input, when the file odd stands beside it, it writes a line that is no JSON before it reads
# any input, and when the file deaf stands beside it, it reads no input and lives on.
here=$(dirname "$0")
printf '%s\0' "$@" > "$here/args.$$"
echo "$$" >> "$here/pids"
id=none
last=
for arg in "$@"; do
  case $last in --session-id|--resume) id=$arg ;; esac
  if [ "$arg" = --fork-session ]; then id="$id-forked"; fi
  last=$arg
done
say() { printf '%s\n' "{\"type\":\"stream_event\",\"event\":$1}"; }
settle() { printf '%s\n' "{\"type\":\"assistant\",\"message\":{\"content\":[$1]}}"; }
part() { say "{\"type\":\"content_block_delta\",\"index\":$at,\"delta\":{\"type\":\"text_delta\",\"text\":$1}}"; }
if [ -f "$here/loud" ]; then head -c 262144 /dev/zero | tr '\0' e >&2; fi
if [ -f "$here/odd" ]; then echo "no line of json"; fi
if [ -f "$here/deaf" ]; then exec sleep 60; fi
n=0
while IFS= read -r line; do
  printf '%s\n' "$line" >> "$here/in"
  case $line in
    *FAIL*) echo "deliberate failure" >&2; exit 2 ;;
    *WAIT*) continue ;;
    *LOUD*) exec 1>&-; head -c 262144 /dev/zero | tr '\0' e >&2; exit 3 ;;
    *MUTE*) exec 1>&-; exec sleep 60 ;;
    *ORPHAN*) sleep 60 & echo $! > "$here/orphan"; echo "orphan left" >&2; exit 4 ;;
    *LONG*) head -c 33554433 /dev/zero; echo; continue ;;
    *ERROR*) echo '{"type":"result","is_error":true,"result":"the model is overloaded"}'; continue ;;
    *BARE*) echo '{"type":"result","result":"close(1)","usage":{}}'; continue ;;
  esac
  n=$((n + 1))
  count=$(( $(cat "$here/count" 2>/dev/null || echo 0) + 1 ))
  echo "$count" > "$here/count"
  say '{"type":"message_start"}'
  at=0
  case $line in
    *THINK*)
      say '{"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":""}}'
      say '{"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"hmm"}}'
      say '{"type":"content_block_delta","index":0,"delta":{"type":"signature_delta","signature":"sig"}}'
      say '{"type":"content_block_stop","index":0}'
      settle '{"type":"thinking","thinking":"hmm","signature":"sig"}'
      at=1 ;;
  esac
  say "{\"type\":\"content_block_start\",\"index\":$at,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}"
  if [ -f "$here/word.$count" ]; then
    word=$(cat "$here/word.$count")
    part "$word"
  else
    word="\"close(\\\"reply $n\\\")\""
    case $line in *GAP*) ;; *) part '"close("' ;; esac
    part "\"\\\"reply $n\\\")\""
  fi
  say "{\"type\":\"content_block_stop\",\"index\":$at}"
  say '{"type":"message_stop"}'
  settle "{\"type\":\"text\",\"text\":$word}"
  printf '%s\n' "{\"type\":\"result\",\"session_id\":\"$id\",\"total_cost_usd\":0.0$n,\"usage\":{\"input_tokens\":10,\"output_tokens\":5,\"cache_read_input_tokens\":20,\"cache_creation_input_tokens\":3}}"
done
