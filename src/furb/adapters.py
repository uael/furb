"""The adapters of the engines of this major, by the digest of an engine.

A record replays on the engine that made it, and an engine of the major may hold a defect of replay that a later
build fixed at its root. Each entry holds the changes that fix such a defect in that engine, each a text of the engine
and the text in its place, which the store makes in the engine of a record that names the digest, before the life
replays it. An engine that no key names opens as it is. The Kernel of a record takes what kernel gives.
"""

ADAPTERS = {
  # 5f66c73c (engine.py of 8f4d10c9b, #1313) repeats the first started or done of an act of the outside before the
  # facts that the record holds before it; the ranks of 93eba807f (#1329) say those facts first.
  "5f66c73c214963e47e4c1048ef2286d80e02537c56152a456191e81b6b8fc1c4": [
    (
      "  due = [e for (e,) in record[::-1] if first.get(e[1]) is not e]\n",
      "  due = [e for (e,) in record[::-1] if first.get(e[1]) is not e]\n"
      "  ranks = {id(e): n for n, (e,) in enumerate(record)}\n",
    ),
    (
      '            taken[name] = f[0] == "started"\n            repeat(*f)\n',
      '            taken[name] = f[0] == "started"\n'
      "            while owed() and not question(due[-1]) and ranks[id(due[-1])] < ranks[id(f)]:\n"
      "              recount(due.pop())\n"
      "            repeat(*f)\n",
    ),
  ]
}


def kernel(held: str, ours: str) -> str:
  """The Kernel of a record, as it runs in this build: since 9146a67b8 a Kernel ends at its birth each run that an
  earlier Kernel took and lost, and the opening of the sandbox does not, so a Kernel of the major that has no lost
  takes the lost of the Kernel of this build, which its birth runs first."""
  head = "def kernel(names: Names) -> Ear:\n"
  if "def lost(" in held or head not in held:
    return held
  return held.replace(head, ours[ours.index("def lost(") : ours.index(head)] + head + "  yield from lost(names)\n", 1)
