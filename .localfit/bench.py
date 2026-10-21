import ast, json, os, re, subprocess, sys, time
from pathlib import Path

MODEL = os.environ.get("BENCH_MODEL", "local:qwen3.8-flash-next-iq2_xs")
HERE = Path(__file__).resolve().parent

CALC = "def add(a, b):\n  return a - b\n\n\ndef mean(xs):\n  return sum(xs) / (len(xs) + 1)\n\n\ndef clamp(x, lo, hi):\n  return max(hi, min(lo, x))\n"
TEST = "from calc import add, clamp, mean\n\nassert add(2, 3) == 5\nassert mean([1, 2, 3]) == 2\nassert clamp(5, 0, 3) == 3\nassert clamp(-1, 0, 3) == 0\nassert clamp(2, 0, 3) == 2\nprint('ok')\n"
INI = "[server]\nhost = localhost\nport = 8080\ntimeout = 30\nretries = 3\n"
TASKS = {
  "count": ({"lines.txt": "".join(f"entry {i}\n" for i in range(1, 138))}, "int",
            "How many lines does lines.txt hold? Close with the number.", ""),
  "calc": ({"calc.py": CALC, "test_calc.py": TEST}, "str",
           "calc.py has three bugs. Fix them until `python3 test_calc.py` prints ok, then close with a short report.", ""),
  "edit": ({"settings.ini": INI}, "str",
           "In settings.ini, set timeout to 45 with one replace on the Text that read gives, and change nothing else. Close with the new line.", ""),
  "ask": ({}, "str",
          "Ask the operator which color to use, with a prompt of str inside your word, then write the answer into color.txt and close with it.", "teal\n"),
}


def checked(task, work, value):
  if task == "count":
    return value.strip() == "137"
  if task == "calc":
    got = subprocess.run([sys.executable, "-I", "test_calc.py"], cwd=work, capture_output=True, text=True)
    return got.stdout.strip() == "ok"
  if task == "edit":
    return (work / "settings.ini").read_text() == INI.replace("timeout = 30", "timeout = 45")
  if task == "ask":
    color = work / "color.txt"
    return color.exists() and color.read_text().strip() == "teal"


def facts(furb, record):
  out = subprocess.run([furb, "facts", "--record", str(record)], capture_output=True, text=True).stdout
  got = []
  for line in out.splitlines():
    try:
      fact = ast.literal_eval(line.partition(": ")[2])
    except Exception:
      continue
    if isinstance(fact, list) and len(fact) >= 3:
      got.append(fact)
  return got


def un(v):
  if isinstance(v, dict) and v.get('is') == 'tuple':
    return [un(x) for x in v['args'][0]]
  if isinstance(v, dict) and v.get('is') == 'float':
    return float(v['args'][0])
  if isinstance(v, list):
    return [un(x) for x in v]
  return v


def asked(f):
  return isinstance(f[1], str) and re.fullmatch(re.escape(str(f[0])) + r"\d+", f[1]) is not None


def cause(word, found):
  text = "\n".join(found)
  if "```" in word:
    return "fence"
  if "<tool_call>" in word or "<function=" in word:
    return "markup"
  if any(line.strip() == "=" or line.startswith("= ") for line in word.splitlines()):
    return "stray ="
  if "statement" in text or "syntax" in text.lower() or "Expected" in text:
    return "prose"
  return "type"


def look(furb, record):
  all_facts = facts(furb, record)
  dones = {f[1]: un(f[3]) for f in all_facts if f[0] == "done" and len(f) > 3}
  made = {}
  for f in all_facts:
    if asked(f) and f[0] not in ("reply", "gate", "run", "wants"):
      made[f[2]] = made.get(f[2], 0) + 1
  runs = {f[4]: dones.get(f[1], "pending") for f in all_facts if f[0] == "run" and asked(f)}
  gates = {f[2]: dones.get(f[1], []) for f in all_facts if f[0] == "gate" and asked(f)}
  steps = []
  for f in all_facts:
    if f[0] == "reply" and asked(f) and f[3] == "chain1":
      v = dones.get(f[1], "pending")
      step = {"reply": f[1], "rung": f[2]}
      if isinstance(v, dict) and "is" in v:
        step["failed"] = str(v.get("args"))[:400]
        step["cut"] = "most it writes" in step["failed"] or "before its end" in step["failed"]
      elif isinstance(v, list) and len(v) > 3:
        step["word"] = v[1]
        step["usage"] = v[2]
        step["thought"] = sum(len(json.dumps(b)) for b in (v[3] or []) if isinstance(b, dict) and b.get("type") == "reasoning")
        found = gates.get(f[2]) or []
        step["findings"] = found
        if found:
          step["cause"] = cause(v[1], found)
        r = runs.get(f[2], "pending")
        step["ran"] = None if r is None else str(r)[:300]
        step["acts"] = made.get(f[2], 0)
        step["silent"] = not found and r is None and step["acts"] == 0
      else:
        step["pending"] = True
      steps.append(step)
  words = [s for s in steps if "word" in s]
  last = words[-3:]
  stop = ""
  if any(f[0] == "pause" for f in all_facts):
    stop = "the chain paused"
  elif len(steps) >= 3 and all("failed" in s for s in steps[-3:]):
    stop = "3 failed replies in a row"
  elif len(last) == 3 and all(s["findings"] for s in last):
    stop = "3 refused words in a row"
  elif len(last) == 3 and len({s["word"] for s in last}) == 1:
    stop = "the same word 3 times"
  elif len(last) == 3 and all(s["silent"] for s in last):
    stop = "3 words told nothing"
  causes = {}
  for s in words:
    if s.get("cause"):
      causes[s["cause"]] = causes.get(s["cause"], 0) + 1
  return steps, {
    "replies": len(steps),
    "words": len(words),
    "refused": causes,
    "silent": sum(s["silent"] for s in words),
    "raised": sum(1 for s in words if s["ran"] not in (None, "pending") and "Cancelled" not in s["ran"]),
    "cut": sum(1 for s in steps if s.get("cut")),
    "failed": sum(1 for s in steps if "failed" in s),
    "read_last": words[-1]["usage"][0] if words else 0,
    "written": sum(s["usage"][1] for s in words),
    "stop": stop,
  }


def run(task, furb, out, limit):
  files, shape, message, fed = TASKS[task]
  base = Path(out) / task
  subprocess.run(["rm", "-rf", str(base)])
  work = base / "work"
  work.mkdir(parents=True)
  for name, text in files.items():
    (work / name).write_text(text)
  record = base / "record.jsonl"
  env = dict(os.environ)
  env["FURB_LOCAL_URL"] = "http://192.168.1.13:8080"
  env["FURB_LOCAL_KEY"] = json.loads((Path.home() / ".claude/strata-local.json").read_text())["env"]["ANTHROPIC_AUTH_TOKEN"]
  began = time.time()
  log = open(base / "furb.out", "w")
  proc = subprocess.Popen(
    [furb, "prompt", "--record", str(record), "--cwd", str(work), "--model", MODEL, "--to", MODEL, "--shape", shape, message],
    cwd=work, env=env, stdin=subprocess.PIPE, stdout=log, stderr=subprocess.STDOUT, text=True, start_new_session=True,
  )
  proc.stdin.write(fed)
  proc.stdin.close()
  stop, summary = "", {}
  while proc.poll() is None:
    time.sleep(15)
    if record.exists():
      _, summary = look(furb, record)
      print(json.dumps({"t": round(time.time() - began), **summary}), flush=True)
      stop = summary["stop"]
    if not stop and time.time() - began > limit:
      stop = f"over {limit} seconds"
    if stop:
      try:
        os.killpg(proc.pid, 15)
      except OSError:
        proc.terminate()
      try:
        proc.wait(20)
      except subprocess.TimeoutExpired:
        proc.kill()
      break
  log.close()
  steps, summary = look(furb, record) if record.exists() else ([], {})
  json.dump(steps, open(base / "steps.json", "w"), indent=1)
  value = (base / "furb.out").read_text().strip().splitlines()[-1:] or [""]
  result = {
    "task": task, "ok": not stop and proc.returncode == 0 and checked(task, work, value[0]),
    "exit": proc.returncode, "seconds": round(time.time() - began), **summary, "stop": stop or summary.get("stop", ""),
  }
  print("RESULT " + json.dumps(result), flush=True)


if __name__ == "__main__":
  run(sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4]) if len(sys.argv) > 4 else 900)
