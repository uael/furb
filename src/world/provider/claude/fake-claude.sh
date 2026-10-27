#!/bin/sh
# The claude command line of the tests, which answers from a script. It says the stream of the real one: each block
# streams in parts and settles whole, and the result says the usage and the running cost. It writes, beside itself,
# the words of each process it was, the id of each, each line of input it read, and how many turns it answered in
# all. The word of a turn is the JSON text in the file word.N beside it, for the Nth turn of all, and else a close of
# "reply n", for the nth turn of its process. A line that holds FAIL ends it with a failure, a line that holds WAIT
# gets no answer, a line that holds LONG gets a line longer than a reader takes, a line that holds ERROR gets a result
# that says an error, a line that holds LIMIT gets, once, a result that says the usage limit ends a second from now, a line that holds BARE gets a result with no block before it, a line that holds THINK gets a
# thought before the text, and a line that holds LOUD closes the stdout, writes more on the stderr than its pipe holds,
# and ends it with a failure, and a line that holds GAP streams its text with its first part left out, and settles the
# text whole. A line that holds LOOP gets a message that calls a tool before the last one, as a turn of many
# requests does, and a result whose usage is the sum of the two. A line that holds MUTE closes the stdout and lives
# on, and a line that holds ORPHAN starts a process that
# holds the stdout and the stderr open, writes the id of that process in the file orphan beside it, says so on the
# stderr, and ends it with a failure. When the file loud stands beside it, it writes more on the stderr than its pipe
# holds before it reads any input, when the file odd stands beside it, it writes a line that is no JSON before it reads
# any input, and when the file deaf stands beside it, it reads no input and lives on. When the file cost stands beside
# it, each turn costs the dollars that it holds. It writes the MAX_THINKING_TOKENS of each process beside it too, or
# unset. It writes the CLAUDE_CODE_OAUTH_TOKEN and the CLAUDE_CONFIG_DIR of each process beside it too, in token.N and
# configdir.N, or unset. When the file limited.TOKEN stands beside it, each turn of a process whose token is TOKEN gets
# a result that says the usage limit ends in as many seconds as the file holds, and when the file signedout.TOKEN
# stands beside it, a result that says the account is signed out.
here=$(dirname "$0")
# A process that furb starts to ask for the models (FURB_CLAUDE_DISCOVER) answers control requests and no turn, and
# leaves no trace of a process of a turn. It writes a line in discoveries for each one. When the file models stands
# beside it, each of its lines is an alias, its window and its efforts joined by commas, or - for none, and it
# answers from them; else it answers each request with an error, as a command line that has no discovery does.
if [ -n "${FURB_CLAUDE_DISCOVER-}" ]; then
  echo "$$" >> "$here/discoveries"
  while IFS= read -r line; do
    id=$(printf '%s' "$line" | sed 's/.*"request_id":"\([^"]*\)".*/\1/')
    body=
    if [ -f "$here/models" ]; then
      case $line in
        *'"initialize"'*)
          list=$(awk '{n = split($3, e, ","); l = ""; if ($3 != "-") for (i = 1; i <= n; i++) l = l (i > 1 ? "," : "") "\"" e[i] "\""; printf "%s{\"value\":\"%s\",\"supportsEffort\":%s,\"supportedEffortLevels\":[%s]}", (NR > 1 ? "," : ""), $1, ($3 == "-" ? "false" : "true"), l}' "$here/models")
          body="{\"models\":[{\"value\":\"default\",\"supportsEffort\":true},$list]}" ;;
        *set_model*) printf '%s' "$line" | sed 's/.*"model":"\([^"]*\)".*/\1/' > "$here/model"; body='{}' ;;
        *get_context_usage*)
          body=$(awk -v m="$(cat "$here/model")" '$1 == m {printf "{\"maxTokens\":%s,\"model\":\"resolved-%s\"}", $2, m}' "$here/models") ;;
      esac
    fi
    if [ -n "$body" ]; then
      printf '%s\n' "{\"type\":\"control_response\",\"response\":{\"subtype\":\"success\",\"request_id\":\"$id\",\"response\":$body}}"
    else
      printf '%s\n' "{\"type\":\"control_response\",\"response\":{\"subtype\":\"error\",\"request_id\":\"$id\",\"error\":\"unsupported\"}}"
    fi
  done
  exit 0
fi
printf '%s\0' "$@" > "$here/args.$$"
printf '%s' "${MAX_THINKING_TOKENS-unset}" > "$here/thinking.$$"
who=${CLAUDE_CODE_OAUTH_TOKEN-unset}
printf '%s' "$who" > "$here/token.$$"
printf '%s' "${CLAUDE_CONFIG_DIR-unset}" > "$here/configdir.$$"
echo "$$" >> "$here/pids"
id=none
last=
for arg in "$@"; do
  case $last in --session-id|--resume) id=$arg ;; esac
  if [ "$arg" = --fork-session ]; then id="$id-forked"; fi
  last=$arg
done
say() { printf '%s\n' "{\"type\":\"stream_event\",\"event\":$1}"; }
settle() { printf '%s\n' "{\"type\":\"assistant\",\"message\":{\"content\":[$1]${2:+,\"usage\":$2}}}"; }
part() { say "{\"type\":\"content_block_delta\",\"index\":$at,\"delta\":{\"type\":\"text_delta\",\"text\":$1}}"; }
if [ -f "$here/loud" ]; then head -c 262144 /dev/zero | tr '\0' e >&2; fi
if [ -f "$here/odd" ]; then echo "no line of json"; fi
if [ -f "$here/deaf" ]; then exec sleep 60; fi
n=0
while IFS= read -r line; do
  printf '%s\n' "$line" >> "$here/in"
  if [ -f "$here/limited.$who" ]; then
    echo "{\"type\":\"result\",\"is_error\":true,\"result\":\"Claude AI usage limit reached|$(( $(date +%s) + $(cat "$here/limited.$who") ))\"}"
    continue
  fi
  if [ -f "$here/signedout.$who" ]; then
    echo '{"type":"result","is_error":true,"result":"Invalid API key · Please run /login"}'
    continue
  fi
  case $line in
    *FAIL*) echo "deliberate failure" >&2; exit 2 ;;
    *WAIT*) continue ;;
    *LOUD*) exec 1>&-; head -c 262144 /dev/zero | tr '\0' e >&2; exit 3 ;;
    *MUTE*) exec 1>&-; exec sleep 60 ;;
    *ORPHAN*) sleep 60 & echo $! > "$here/orphan"; echo "orphan left" >&2; exit 4 ;;
    *LONG*) head -c 33554433 /dev/zero; echo; continue ;;
    *ERROR*) echo '{"type":"result","is_error":true,"result":"the model is overloaded"}'; continue ;;
    *DETAIL*) echo '{"type":"result","subtype":"error_during_execution","is_error":true,"result":"","api_error_status":401,"errors":["Invalid API key","Please run /login"]}'; continue ;;
    *LIMIT*)
      if [ ! -f "$here/limited" ]; then
        touch "$here/limited"
        echo "{\"type\":\"result\",\"is_error\":true,\"result\":\"Claude AI usage limit reached|$(( $(date +%s) + 1 ))\"}"
        continue
      fi ;;
    *BARE*) echo '{"type":"result","result":"close(1)","usage":{}}'; continue ;;
  esac
  n=$((n + 1))
  count=$(( $(cat "$here/count" 2>/dev/null || echo 0) + 1 ))
  echo "$count" > "$here/count"
  used="{\"input_tokens\":10,\"output_tokens\":5,\"cache_read_input_tokens\":20,\"cache_creation_input_tokens\":3}"
  case $line in
    *LOOP*)
      say '{"type":"message_start"}'
      settle '{"type":"tool_use","id":"t1","name":"Bash","input":{}}' '{"input_tokens":1000,"output_tokens":900,"cache_read_input_tokens":2000,"cache_creation_input_tokens":0}'
      used='{"input_tokens":1010,"output_tokens":905,"cache_read_input_tokens":2020,"cache_creation_input_tokens":3}' ;;
  esac
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
  settle "{\"type\":\"text\",\"text\":$word}" "{\"input_tokens\":10,\"output_tokens\":5,\"cache_read_input_tokens\":20,\"cache_creation_input_tokens\":3}"
  paid="0.0$n"
  if [ -f "$here/cost" ]; then paid=$(awk "BEGIN { print $n * $(cat "$here/cost") }"); fi
  printf '%s\n' "{\"type\":\"result\",\"session_id\":\"$id\",\"total_cost_usd\":$paid,\"usage\":$used}"
done
