"""The adapters of the engines of this major, by the digest of an engine.

A record replays on the engine that made it, and an engine of the major may hold a defect of replay that a later
build fixed at its root. Each entry holds the changes that fix such a defect in that engine, each a text of the engine
and the text in its place, which the store makes in the engine of a record that names the digest, before the life
replays it. An engine that no key names opens as it is. The Kernel of a record takes what kernel gives.
"""

LISTENING = [
  ("  global say, act, drive, get, peek, transcript\n", "  global say, act, drive, get, peek, transcript, listening\n"),
  ("  def get(about):\n", "  def listening():\n    return alive\n\n  def get(about):\n"),
]
"""The changes that give boot the ears that still hear, as listening, which the lives of 00ac9534e read."""

OLD_LIVES = (
  "            m[4] for x in transcript(c) if question(x) and not over(x[1]) and (m := get(x[2])) and m[4] in words\n"
)
"""The lives of bc342872a, which read slot 4 of the maker of each open question."""

RUNG_LIVES = (
  "            m[4]\n"
  "            for x in transcript(c)\n"
  '            if question(x) and not over(x[1]) and question(("rung", x[2])) and (m := get(x[2])) and m[4] in words\n'
)
"""The lives of 72f9fdc97, which read only a maker that is a rung."""

LIVES = (
  "            get(r)[4]\n"
  "            for x in listening()\n"
  "            if scope(x) == c\n"
  "            for r in lineage(x)\n"
  '            if question(("rung", r)) and get(r)[4] in words\n'
)
"""The lives of 00ac9534e."""


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
  ],
  # The engines from bc342872a (#1404) to 7ba1825fc gather the lives of a chain from the maker of each open question:
  # those up to d5550efda read its slot 4, which an act that no rung made has not, as the inner act of an act of a
  # life word, so gathered raises IndexError and the life writes no checkpoint; and each of them drops a life word whose
  # rung stands higher in the lineage, or whose act is done while its lasting ear still hears, so a born life makes no
  # watcher of the skills again. The lives of 00ac9534e read each life word through the lineage of each ear that hears.
  **{
    digest: [*LISTENING, (OLD_LIVES, LIVES)]
    for digest in (
      "55d2b55ef3f0d893811a9b3a753c7bccb07408f9548f7c3ea4f0298d55e4c284",  # bc342872a
      "18c4d46fdede426f80829f0656515c2efaa4daa6a34bd9b2bded42e919c293e5",  # ac353a4d2
      "d7432602912326f3d1090334e375c50f5c9f82986a757e10dd966e972bc23222",  # bd2e4e935
      "86bc898f353c9c0e84426afaba3756eb1c0c0d16e20da36be547274e604aa027",  # 606d0528c
      "0a11688b1e0831a9b1b9beb620e51ec91ee3c26be05fe9d3cf7b24b03b6bd832",  # d5550efda
    )
  },
  **{
    digest: [*LISTENING, (RUNG_LIVES, LIVES)]
    for digest in (
      "1a358a320146986dc372541f71ccd7da27989cf6f2a5f6355c506bf876245c2d",  # 72f9fdc97
      "70bb886877d9f53861b4332df552cac163b13e6a8ece5eeedf35ef8ce33cdddd",  # 7ba1825fc
    )
  },
}


def kernel(held: str, ours: str) -> str:
  """The Kernel of a record, as it runs in this build: since 9146a67b8 a Kernel ends at its birth each run that an
  earlier Kernel took and lost, and the opening of the sandbox does not, so a Kernel of the major that has no lost
  takes the lost of the Kernel of this build, which its birth runs first."""
  head = "def kernel(names: Names) -> Ear:\n"
  if "def lost(" in held or head not in held:
    return held
  return held.replace(head, ours[ours.index("def lost(") : ours.index(head)] + head + "  yield from lost(names)\n", 1)
