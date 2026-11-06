"""The scripts of the skill converge-tui: the differ of two captures of the TUI, and the filter of the gaps of its
judges."""

# ruff: noqa: S603
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

SKILL = Path(__file__).resolve().parents[2] / ".agents" / "skills" / "converge-tui"
RULINGS = "## A ruling\n\n> The footer of the feed names the model of its chain."
QUOTE = "The footer of the feed names the model"


def grid(*rows: str, colors: tuple[str, str] = ("#ffffff", "#000000"), attributes: int = 0) -> str:
  return json.dumps({"cols": 40, "rows": len(rows), "lines": [[[row, *colors, attributes]] for row in rows]})


def shoot(tmp: Path, was: dict | None, now: dict, fails: str = "", *, whole: bool = False) -> dict:
  out = tmp / "out"
  for part, shots in (("before", was or {}), ("head", now)):
    for name, (png, cells) in shots.items():
      folder, base = out / part / name.split("/")[0], name.split("/")[1]
      folder.mkdir(parents=True, exist_ok=True)
      (folder / f"{base}.png").write_bytes(png)
      if cells is not None:
        (folder / f"{base}.json").write_text(cells, encoding="utf-8")
  if fails:
    (out / "fails.txt").write_text(fails, encoding="utf-8")
  words = [str(out), str(tmp / "images"), str(tmp), str(out / "head"), str(out / "before") if was is not None else ""]
  got = subprocess.run(
    [sys.executable, "-I", str(SKILL / "diff.py"), *words, "1" if whole else "0", "main", "head"],
    capture_output=True,
    encoding="utf-8",
    check=True,
  )
  assert len(got.stdout) < 200 and "\n" not in got.stdout.strip()
  return json.loads((out / "shots.json").read_text(encoding="utf-8"))


def test_the_list_of_the_captures_names_each_image_by_the_digest_of_its_bytes(tmp_path: Path) -> None:
  seen = shoot(tmp_path, None, {"demo/01-welcome": (b"\x89PNG one", None)})
  digest = hashlib.sha256(b"\x89PNG one").hexdigest() + ".png"
  assert seen["shots"] == {"demo/01-welcome": digest}
  assert (tmp_path / "images" / digest).read_bytes() == b"\x89PNG one"


def test_a_capture_whose_cells_did_not_change_goes_to_no_judge(tmp_path: Path) -> None:
  was = {"demo/02-feed": (b"one", grid("⠋ Working  # 212ms $0.0024   x", "runs  9.0s", "saved 1793047160463.html"))}
  now = {"demo/02-feed": (b"two", grid("⠴ Working  # 1215ms $0.0024  x", "runs  15s", "saved 1793047721926.html"))}
  seen = shoot(tmp_path, was, now)
  assert (seen["kept"], seen["changed"]) == (["demo/02-feed"], [])


def test_a_capture_whose_text_changed_comes_as_its_frame_with_each_changed_row_marked(tmp_path: Path) -> None:
  was = {"demo/02-feed": (b"one", grid("Main", "old row", "end"))}
  now = {"demo/02-feed": (b"two", grid("Main", "new row", "end"))}
  [change] = shoot(tmp_path, was, now)["changed"]
  assert (change["name"], change["kind"], change["images"]) == ("demo/02-feed", "text", [])
  assert change["text"].splitlines() == [
    "     styles: a #ffffff on #000000",
    "   1 Main",
    "+  2 new row",
    "     main 1-3: old",
    "     colors 1-3: a1-3",
    "   3 end",
  ]


def test_a_row_that_is_gone_or_new_comes_whole_and_rows_far_from_a_change_are_named(tmp_path: Path) -> None:
  rows = [f"row {n}" for n in range(1, 9)]
  was = {"demo/02-feed": (b"one", grid(*rows))}
  now = {"demo/02-feed": (b"two", grid(*rows[:6], "one more", *rows[6:], "the end"))}
  [change] = shoot(tmp_path, was, now)["changed"]
  assert change["text"].splitlines() == [
    "     styles: a #ffffff on #000000",
    "     rows 1-4 as main",
    "   5 row 5",
    "   6 row 6",
    "+  7 one more",
    "     colors: a1-8",
    "   8 row 7",
    "   9 row 8",
    "+ 10 the end",
    "     colors: a1-7",
  ]


def test_a_capture_whose_change_is_the_change_of_another_names_it(tmp_path: Path) -> None:
  was = {f"demo/0{n}-shot": (b"one", grid("old")) for n in (1, 2)}
  now = {f"demo/0{n}-shot": (bytes([n]), grid("new")) for n in (1, 2)}
  first, second = shoot(tmp_path, was, now)["changed"]
  assert (first["kind"], second["kind"], second["text"]) == ("text", "same", "the same change as demo/01-shot")


def test_a_capture_whose_colors_alone_changed_comes_with_both_images(tmp_path: Path) -> None:
  was = {"demo/02-feed": (b"one", grid("Main", colors=("#ffffff", "#000000")))}
  now = {"demo/02-feed": (b"two", grid("Main", colors=("#888888", "#000000"), attributes=2))}
  [change] = shoot(tmp_path, was, now)["changed"]
  assert change["kind"] == "colors"
  assert change["text"].splitlines() == [
    "     styles: a #ffffff on #000000, b #888888 on #000000 dim",
    "~  1 Main",
    "     colors main 1-4: a1-4",
    "     colors head 1-4: b1-4",
  ]
  digests = [hashlib.sha256(b).hexdigest() + ".png" for b in (b"one", b"two")]
  assert change["images"] == [["main", digests[0]], ["head", digests[1]]]


def test_a_new_capture_comes_whole_with_its_image(tmp_path: Path) -> None:
  [change] = shoot(tmp_path, {}, {"life/r-chain1": (b"one", grid("Main", "row"))})["changed"]
  assert (change["kind"], change["text"]) == ("new", "   1 Main\n   2 row")
  assert change["images"] == [["head", hashlib.sha256(b"one").hexdigest() + ".png"]]


def test_a_capture_of_a_pass_that_changed_comes_with_its_image(tmp_path: Path) -> None:
  was = {"demo/02-feed": (b"one", grid("old"))}
  [change] = shoot(tmp_path, was, {"demo/02-feed": (b"two", grid("new"))}, whole=True)["changed"]
  assert change["images"] == [["head", hashlib.sha256(b"two").hexdigest() + ".png"]]


def test_a_failed_step_of_the_capture_is_a_fault_and_only_a_failed_check_is_a_gap(tmp_path: Path) -> None:
  out = tmp_path / "out"
  out.mkdir()
  (out / "build.log").write_text("cargo\nfailed here\n", encoding="utf-8")
  none = out / "none.log"
  fails = f"head\tbuild\t{out / 'build.log'}\nbefore\tdemo\t{none}\nhead\tcheck: bun run check\t{none}\n"
  seen = shoot(tmp_path, {}, {}, fails)
  assert seen["faults"] == [
    ["build", "the step build of the capture failed: cargo failed here", ""],
    ["demo", "the step demo of the capture failed: ", ""],
  ]
  assert seen["failed"] == [["check: bun run check", "the step check: bun run check of the capture failed: ", ""]]


def test_the_list_of_the_captures_is_a_file_and_stdout_holds_one_short_line(tmp_path: Path) -> None:
  rows = [f"row {n} " + "x" * 100 for n in range(3000)]
  was = {"demo/02-feed": (b"one", grid(*rows))}
  now = {"demo/02-feed": (b"two", grid(*[row + "y" for row in rows]))}
  [change] = shoot(tmp_path, was, now)["changed"]
  assert len(change["text"]) > 300000


def test_a_gap_counts_only_when_it_names_a_capture_of_its_batch_and_quotes_the_rulings(tmp_path: Path) -> None:
  rulings = tmp_path / "rulings.md"
  rulings.write_text(RULINGS, encoding="utf-8")
  answers = [
    [
      ["demo/01-welcome", "The text is cut.", QUOTE],
      ["demo/01-welcome", "  the text is CUT.  ", "> The **footer** of the feed names the model"],
      ["demo/99-none", "No such capture.", QUOTE],
      ["demo/01-welcome", "A taste of the judge.", "it should be blue"],
      ["demo/01-welcome", "Too short a quote.", "TUI"],
      "not a tuple",
    ],
    "no list",
  ]
  got = subprocess.run(
    [sys.executable, "-I", str(SKILL / "gaps.py"), str(rulings), "demo/01-welcome"],
    input=json.dumps(answers),
    capture_output=True,
    encoding="utf-8",
    check=True,
  )
  assert json.loads(got.stdout) == [["demo/01-welcome", "The text is cut.", QUOTE]]


def test_the_records_of_a_capture_keep_the_pinned_record_at_its_pinned_size(tmp_path: Path) -> None:
  source, out = tmp_path / "sessions", tmp_path / "out"
  (out / "records").mkdir(parents=True)
  source.mkdir()
  old, new = source / "old.jsonl", source / "new.jsonl"
  old.write_bytes(b"one\ntwo\nthree\n")
  new.write_bytes(b"four\n")
  os.utime(old, (1, 1))
  (out / "pins.json").write_text(json.dumps({"old.jsonl": 8}), encoding="utf-8")
  subprocess.run([sys.executable, "-I", str(SKILL / "records.py"), str(source), str(out), "1", "1000"], check=True)
  assert json.loads((out / "records.json").read_text(encoding="utf-8")) == {"old.jsonl": 8}
  assert (out / "records" / "old.jsonl").read_bytes() == b"one\ntwo\n"


def test_the_slice_of_a_capture_is_the_newest_long_log_at_its_end_until_a_pin_names_its_moment(tmp_path: Path) -> None:
  source, out = tmp_path / "sessions", tmp_path / "out"
  (out / "records").mkdir(parents=True)
  source.mkdir()
  short, old, new = source / "short.jsonl.log", source / "old.jsonl.log", source / "new.jsonl.log"
  short.write_bytes(b"[]\n")
  for one in (old, new):
    with one.open("wb") as made:
      made.truncate(300_000_000)
  os.utime(old, (1, 1))
  words = [sys.executable, "-I", str(SKILL / "records.py"), str(source), str(out), "1", "1000"]
  subprocess.run(words, check=True)
  assert json.loads((out / "slice.json").read_text(encoding="utf-8")) == {"log": str(new), "moment": "end"}
  fixture = out / "slice.jsonl"
  fixture.write_text('{"recorded": true, "cut": 7, "of": 7}\n["chain"]\n', encoding="utf-8")
  subprocess.run([sys.executable, "-I", str(SKILL / "records.py"), "pin", str(out), str(fixture)], check=True)
  pins = json.loads((out / "records.json").read_text(encoding="utf-8"))
  assert pins["slice"] == {"log": "new.jsonl.log", "moment": 7}
  (out / "pins.json").write_text(json.dumps({"slice": {"log": "old.jsonl.log", "moment": 3}}), encoding="utf-8")
  subprocess.run(words, check=True)
  assert json.loads((out / "slice.json").read_text(encoding="utf-8")) == {"log": str(old), "moment": 3}
