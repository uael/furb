"""The bench of a loop: one word of furb that triages support tickets, against Claude Code on the same model.

The rig makes the tickets from a seed, with a known label for each. The arm `furb` runs one word on a life of furb: the
word loops on the tickets and asks the model one prompt per ticket, on a chain of its own, whose shape is `str`, the
label alone. The arm `claude` gives every ticket to the claude command line in one session, through a wrapper that
points it at the same server, as `~/.claude/strata-claude` does, and the session writes a label for each ticket to a
file. Both arms read the same instructions, `BRIEF`.

The server measures each arm: the rig reads the totals of its `/metrics` before and after the arm, as Strata gives
them, so the tokens and the seconds are those of the server. Run each arm alone on the server, since the totals count
every client. The scores compare each label with the known one.

    python script/bench/loop.py make <dir> [--seed 7] [--count 200]
    python script/bench/loop.py furb <dir> --model local:<model>/<effort>
    python script/bench/loop.py claude <dir> [--claude ~/.claude/strata-claude]
    python script/bench/loop.py score <dir>

The arms read `FURB_LOCAL_URL` and `FURB_LOCAL_KEY` for the address and the key of the server.
"""

import argparse
import json
import os
import random
import subprocess
import sys
import time
import urllib.request
from pathlib import Path

LABELS = {
  "billing": "a question or a problem about a charge, an invoice, a payment method or a price, with no request for money back",
  "refund": "the customer asks for money back",
  "shipping": "where an order is, a late, lost or damaged delivery, or the address of a delivery",
  "account": "a login, a password, a code of two-factor sign-in, a locked account or the email of the account",
  "bug": "the app or the site shows an error, stops, or does not work as it should",
  "cancel": "the customer wants to end a subscription or to close the account",
  "product": "a question about what a product does, its features, its sizes or what it works with",
  "feedback": "praise, a suggestion, or a complaint with no request",
}
"""LABELS gives each label and what it means, which both arms read."""

BRIEF = (
  "You triage the tickets of the support of a web shop. Give each ticket one label of this list, the one that names"
  " what the customer wants most:\n"
  + "\n".join(f"- {name}: {meaning}" for name, meaning in LABELS.items())
  + "\nJudge each ticket yourself, as a person of the support reads it. A label is the name alone, in lower case."
)
"""BRIEF is the instructions that both arms read."""

FURB = """import json
tickets = json.loads(read("tickets.json", HIDDEN).content)
brief = {brief!r}
labels = {{}}
for one in tickets:
  got = await prompt(str, brief + "\\n\\nThe ticket:\\n" + one["text"] + "\\n\\nClose with its label alone.", on=chain(one["id"]))
  labels[one["id"]] = str(got)
  write(Text("labels.json", json.dumps(labels, indent=1) + "\\n"))
len(labels)"""
"""FURB is the word of the arm furb: it loops on the tickets and asks one prompt of the shape str for each."""

CLAUDE = (
  BRIEF
  + "\n\nThe tickets are in tickets.json, a list of objects with an id and a text. Write labels.json: one JSON object"
  " that maps the id of each ticket to its label."
)
"""CLAUDE is what the arm claude asks: the same instructions, and the files that hold the tickets and the labels."""

OPENERS = ["Hi,", "Hello there.", "Good morning,", "Hey support,", "Hi team,", "", "Dear support,", "Hello,"]
CLOSERS = ["Thanks.", "Thank you!", "Regards, {name}", "Cheers, {name}", "", "Thanks in advance.", "{name}"]
ASIDES = [
  "I have been a customer for {years} years.",
  "I am writing from my phone, sorry for typos.",
  "This is my second message about it.",
  "My sister recommended your shop.",
  "I need this sorted before the weekend.",
  "",
  "",
  "",
]
TEMPLATES = {
  "billing": [
    "My invoice for order {order} shows {amount} but the price on the page was lower. Which one is right?",
    "I see two charges of {amount} on my card for order {order}. Why was I charged twice?",
    "How do I change the card that my monthly plan uses? The old one expired.",
    "Can you send me an invoice with my company name on it for order {order}?",
    "Why does my bill include a fee of {amount} that I never saw at checkout?",
    "My payment for the {product} failed three times, but the bank says the card is fine.",
    "Do you take PayPal, or only cards? I want to pay for the {product} that way.",
  ],
  "refund": [
    "I returned the {product} two weeks ago and I still have not got my {amount} back. Please refund me.",
    "The {product} broke after a day. I do not want a new one, I want my money back.",
    "Please give me back the {amount} for order {order}, I changed my mind and never opened it.",
    "You charged me twice for order {order}. Please return the extra {amount} to my card.",
    "The {product} I got is not what the photos showed. I want a full refund.",
    "When will the refund for order {order} reach my account? You said 5 days and it is 12 now.",
    "I cancelled order {order} before it shipped, so please refund the {amount}.",
  ],
  "shipping": [
    "Where is my order {order}? The tracking has not moved since {date}.",
    "The box of order {order} arrived crushed and the {product} inside is scratched. What now?",
    "Can I change the delivery address of order {order}? I moved last week.",
    "The courier says order {order} was delivered on {date}, but nothing came.",
    "Do you ship the {product} to Canada, and how long does it take?",
    "My order {order} was due on {date} and it is still not here.",
    "Only half of order {order} arrived. Where is the {product}?",
  ],
  "account": [
    "I cannot log in. The site says my password is wrong but I just reset it.",
    "My account is locked after too many tries. Please unlock it.",
    "The code of the two-factor sign-in never reaches my phone.",
    "How do I change the email of my account? I lost access to the old one.",
    "I never got the email to reset my password, I checked the spam folder too.",
    "Someone else signed in to my account from another country. Please help me secure it.",
    "I made two accounts by mistake. Can you merge them into one?",
  ],
  "bug": [
    "The app closes each time I open my cart.",
    "When I click Pay, the page shows 'Error 500' and nothing happens.",
    "The filter for size does not work: it shows every size anyway.",
    "Your site shows the prices in dollars though I chose euros, and the button to switch does nothing.",
    "The search box gives no result for '{product}', but the {product} is on the home page.",
    "The app on my phone shows a blank screen after the last update.",
    "I add the {product} to my cart and it is gone when I reload the page.",
  ],
  "cancel": [
    "Please cancel my monthly plan. I do not need it anymore.",
    "I want to close my account and delete my data.",
    "How do I stop the subscription before the next charge on {date}?",
    "Cancel my membership please, I am moving abroad.",
    "I want to end my yearly plan now. Do not renew it.",
    "Please delete my account. I will not use your shop again.",
    "Stop my subscription box, starting this month.",
  ],
  "product": [
    "Does the {product} work with an iPhone?",
    "What is the difference between the {product} and the {product2}?",
    "Is the {product} waterproof, or only water resistant?",
    "Which size of the {product} should I take if I am 180 cm tall?",
    "How long does the battery of the {product} last?",
    "Does the {product} come with a charger in the box?",
    "Can the {product} be used outdoors in winter?",
  ],
  "feedback": [
    "Just wanted to say the {product} is great and the delivery was fast. Well done!",
    "Your new site is much harder to use than the old one. I liked the old menu better.",
    "It would be nice if you added a dark mode to the app.",
    "The person on the chat yesterday was very kind and patient. Please thank her.",
    "I think the packaging uses too much plastic. You could do better.",
    "Suggestion: let us save more than one address of delivery.",
    "Five stars for the {product}, it is the best I have owned.",
  ],
}
"""TEMPLATES gives the tickets of each label, which the seed fills."""
PRODUCTS = ["headphones", "rain jacket", "coffee grinder", "desk lamp", "running shoes", "backpack", "smart watch"]
NAMES = ["Ana", "Bruno", "Chloe", "Dev", "Emma", "Farid", "Grace", "Hugo", "Ines", "Jonas"]


def make(folder: Path, seed: int, count: int) -> None:
  """Write `count` tickets from `seed` in `folder`: the tickets that both arms read, and the known labels apart."""
  rng = random.Random(seed)  # noqa: S311
  names = list(LABELS)
  rows = []
  for n in range(count):
    label = names[n % len(names)]
    product, product2 = rng.sample(PRODUCTS, 2)
    words = {
      "order": f"#{rng.randint(10000, 99999)}",
      "amount": f"{rng.randint(5, 400)}.{rng.randint(0, 99):02d} EUR",
      "date": f"{rng.choice(['March', 'June', 'October'])} {rng.randint(1, 28)}",
      "product": product,
      "product2": product2,
      "name": rng.choice(NAMES),
      "years": rng.randint(2, 9),
    }
    parts = [rng.choice(OPENERS), rng.choice(ASIDES), rng.choice(TEMPLATES[label]), rng.choice(CLOSERS)]
    rows.append((" ".join(p.format(**words) for p in parts if p), label))
  rng.shuffle(rows)
  tickets = [{"id": f"t{i:03d}", "text": text} for i, (text, _) in enumerate(rows, 1)]
  folder.mkdir(parents=True, exist_ok=True)
  (folder / "tickets.json").write_text(json.dumps(tickets, indent=1) + "\n", encoding="utf-8")
  known = {one["id"]: label for one, (_, label) in zip(tickets, rows, strict=True)}
  (folder / "answers.json").write_text(json.dumps(known, indent=1) + "\n", encoding="utf-8")


def totals() -> dict:
  """The totals of the server since it started: its requests, the tokens it read and wrote, and its milliseconds."""
  url = os.environ["FURB_LOCAL_URL"].rstrip("/").removesuffix("/v1") + "/metrics"
  ask = urllib.request.Request(url, headers={"Authorization": f"Bearer {os.environ.get('FURB_LOCAL_KEY', '')}"})  # noqa: S310
  with urllib.request.urlopen(ask, timeout=30) as got:  # noqa: S310
    return json.load(got)["totals"]


def arm(folder: Path, name: str, command: list[str], env: dict[str, str], cap: float) -> None:
  """Run one arm in a copy of the tickets of its own for `cap` seconds at most, and keep what the server counted for
  it beside its labels."""
  home = folder / name
  home.mkdir(exist_ok=True)
  (home / "tickets.json").write_text((folder / "tickets.json").read_text(encoding="utf-8"), encoding="utf-8")
  before, start = totals(), time.time()
  try:
    got = subprocess.run(command, cwd=home, env=env, capture_output=True, text=True, timeout=cap, check=False)  # noqa: S603
    code, out, err = got.returncode, got.stdout, got.stderr
  except subprocess.TimeoutExpired as late:
    code, out, err = None, str(late.stdout or ""), str(late.stderr or "")
  after, end = totals(), time.time()
  (home / "stdout.txt").write_text(out, encoding="utf-8")
  (home / "stderr.txt").write_text(err, encoding="utf-8")
  keys = ("requests", "prompt_tokens", "reused", "output_tokens", "prompt_ms", "decode_ms")
  server = {k: after[k] - before[k] for k in keys}
  run = {"code": code, "wall_s": round(end - start, 1), "server": server}
  (home / "run.json").write_text(json.dumps(run, indent=1) + "\n", encoding="utf-8")
  sys.stdout.write(json.dumps(run) + "\n")


def score(folder: Path) -> dict:
  """The share of right labels of each arm, and its tokens and seconds per ticket as the server counted them."""
  known = json.loads((folder / "answers.json").read_text(encoding="utf-8"))
  out = {}
  for name in ("furb", "claude"):
    home = folder / name
    if not (home / "run.json").is_file():
      continue
    run = json.loads((home / "run.json").read_text(encoding="utf-8"))
    try:
      given = json.loads((home / "labels.json").read_text(encoding="utf-8"))
    except OSError, ValueError:
      given = {}
    clean = {k: str(v).strip().strip(".'\"`").lower() for k, v in given.items()}
    right = sum(clean.get(k) == v for k, v in known.items())
    s, n = run["server"], len(known)
    out[name] = {
      "right": right,
      "labeled": sum(k in clean for k in known),
      "share": round(right / n, 3),
      "requests": s["requests"],
      "prompt_tokens_per_ticket": round(s["prompt_tokens"] / n),
      "reused_per_ticket": round(s["reused"] / n),
      "output_tokens_per_ticket": round(s["output_tokens"] / n),
      "server_s_per_ticket": round((s["prompt_ms"] + s["decode_ms"]) / 1000 / n, 2),
      "wall_s_per_ticket": round(run["wall_s"] / n, 2),
      "code": run["code"],
    }
  (folder / "score.json").write_text(json.dumps(out, indent=1) + "\n", encoding="utf-8")
  return out


def main() -> int:
  """Parse the command line and run its step."""
  given = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
  given.add_argument("step", choices=["make", "furb", "claude", "score"])
  given.add_argument("folder", type=Path)
  given.add_argument("--seed", type=int, default=7)
  given.add_argument("--count", type=int, default=200)
  given.add_argument("--model", help="The actor of the arm furb, as local:<model>/<effort>.")
  given.add_argument("--cap", type=float, default=14400.0, help="The seconds that an arm may take.")
  given.add_argument("--claude", default=str(Path.home() / ".claude" / "strata-claude"))
  args = given.parse_args()
  folder = args.folder.absolute()
  env = dict(os.environ)
  if args.step == "make":
    make(folder, args.seed, args.count)
  elif args.step == "furb":
    if not args.model:
      given.error("the arm furb needs --model")
    word = FURB.format(brief=BRIEF)
    furb = str(Path(sys.executable).with_name("furb"))
    record = str(folder / "furb" / "record.jsonl")
    arm(folder, "furb", [furb, "run", word, "--model", args.model, "--record", record], env, args.cap)
  elif args.step == "claude":
    command = [args.claude, "-p", CLAUDE, "--permission-mode", "bypassPermissions", "--output-format", "json"]
    arm(folder, "claude", command, env, args.cap)
  else:
    sys.stdout.write(json.dumps(score(folder), indent=1) + "\n")
  return 0


if __name__ == "__main__":
  sys.exit(main())
