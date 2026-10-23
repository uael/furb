"""Check that each link in a README that a package carries works on the page of the package on its registry.

PyPI, npm and crates.io each show the README of a package, and none of them reads a relative link as GitHub does: a
link to `docs/models.md` breaks there. So each link and each image of such a README is absolute: a file of this
repository is https://github.com/uael/furb/blob/main/<path>, a folder is https://github.com/uael/furb/tree/main/<path>,
and an image is https://raw.githubusercontent.com/uael/furb/main/<path>, which GitHub shows as well.

The script finds each package from its manifest: a pyproject.toml that names a readme, a Cargo.toml of a package that
is published, with the readme that it names or the README.md beside it, and a package.json that is not private, with
the README.md beside it. It lists each package and the README that it carries, and fails on a relative link, and on
an absolute link to a path of this repository that no file of git holds. A link in code is no link. --fix writes each
relative link that points at a file as its absolute link.

Usage: uv run python script/links.py [--fix]
"""

import json
import posixpath
import re
import shutil
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
REPOSITORY = "https://github.com/uael/furb"
RAW = "https://raw.githubusercontent.com/uael/furb/main/"
IMAGES = (".png", ".gif", ".jpg", ".jpeg", ".svg", ".webp")
FENCE = re.compile(r"(?ms)^ {0,3}(```|~~~).*?^ {0,3}\1[^\n]*$")
CODE = re.compile(r"(`+)(?!`).+?(?<!`)\1(?!`)", re.S)
LINKS = (
  re.compile(r"\]\(\s*<?([^\s)>]+)"),
  re.compile(r"(?m)^ {0,3}\[[^\]\n]+\]:\s*<?([^\s>]+)"),
  re.compile(r"""\b(?:src|href)\s*=\s*["']([^"']*)["']"""),
)
OURS = re.compile(rf"(?:{re.escape(REPOSITORY)}/(?:blob|tree)/main/|{re.escape(RAW)})([^#?]*)")


def tracked() -> set[str]:
  out = subprocess.run(  # noqa: S603
    [shutil.which("git") or "git", "ls-files", "-z"], cwd=ROOT, capture_output=True, check=True, text=True
  ).stdout
  return {x for x in out.split("\0") if x}


def stands(path: str, files: set[str]) -> bool:
  path = path.strip("/")
  return path == "" or path in files or any(x.startswith(path + "/") for x in files)


def packages(files: set[str]) -> list[tuple[str, str, str]]:
  found = []
  for manifest in sorted(files):
    folder, name = posixpath.split(manifest)
    readme: object = None
    if name == "pyproject.toml":
      project = tomllib.loads((ROOT / manifest).read_text()).get("project", {})
      label, readme = str(project.get("name", "")), project.get("readme")
      readme = readme.get("file") if isinstance(readme, dict) else readme
    elif name == "Cargo.toml":
      package = tomllib.loads((ROOT / manifest).read_text()).get("package")
      if not isinstance(package, dict) or package.get("publish", True) is False:
        continue
      label, readme = str(package.get("name", "")), package.get("readme", "README.md")
    elif name == "package.json":
      package = json.loads((ROOT / manifest).read_text())
      if package.get("private"):
        continue
      label, readme = str(package.get("name", "")), "README.md"
    else:
      continue
    if isinstance(readme, str) and posixpath.join(folder, readme) in files:
      found.append((label, manifest, posixpath.normpath(posixpath.join(folder, readme))))
  return found


def absolute(target: str, files: set[str]) -> str:
  path, mark, anchor = target.partition("#")
  if path.lower().endswith(IMAGES):
    return RAW + path
  return f"{REPOSITORY}/{'blob' if path in files else 'tree'}/main/{path}{mark}{anchor}"


def check(readme: str, files: set[str], *, fix: bool) -> list[str]:
  text = (ROOT / readme).read_text()
  masked = CODE.sub(lambda m: " " * len(m[0]), FENCE.sub(lambda m: " " * len(m[0]), text))
  wrong, edits = [], []
  for pattern in LINKS:
    for m in pattern.finditer(masked):
      target, line = m[1], text.count("\n", 0, m.start(1)) + 1
      if target.startswith("#") or re.match(r"[a-zA-Z][a-zA-Z0-9+.-]*:", target):
        if (ours := OURS.match(target)) and not stands(ours[1], files):
          wrong.append(f"{readme}:{line}: {target} points at no file of the repository")
        continue
      path, mark, anchor = target.partition("#")
      resolved = posixpath.normpath(posixpath.join(posixpath.dirname(readme), path.split("?")[0]))
      if resolved.startswith("..") or not stands(resolved, files):
        wrong.append(f"{readme}:{line}: {target} is relative and points at no file of the repository")
        continue
      fixed = absolute(resolved + mark + anchor, files)
      edits.append((m.start(1), m.end(1), fixed))
      if not fix:
        wrong.append(f"{readme}:{line}: {target} is relative, so it breaks on a registry: write {fixed}")
  wrong.sort(key=lambda x: int(x.split(":")[1]))
  if fix and edits:
    for start, end, fixed in sorted(edits, reverse=True):
      text = text[:start] + fixed + text[end:]
    (ROOT / readme).write_text(text)
  return wrong


def main() -> int:
  fix, files = "--fix" in sys.argv[1:], tracked()
  found = packages(files)
  wrong = []
  for readme in dict.fromkeys(x[2] for x in found):
    wrong += check(readme, files, fix=fix)
  for label, manifest, readme in found:
    sys.stdout.write(f"{label} ({manifest}) carries {readme}\n")
  if wrong:
    sys.stderr.write(
      "\n".join(wrong) + "\nuv run python script/links.py --fix writes each relative link as absolute.\n"
    )
    return 1
  sys.stdout.write("Each link of each README of a package works on its registry.\n")
  return 0


if __name__ == "__main__":
  sys.exit(main())
