"""hearer, the ear that hears a fact now."""

from conftest import WORLD, born, relived, said, settle
from furb import engine


async def test_the_hearer_is_the_ear_that_hears_a_fact_now() -> None:
  """The hearer is the ear that hears a fact now, empty while no ear hears, and the Kernel makes the rung of a word the hearer while it steps that word."""
  _, log, root = born()
  assert engine.hearer.get() == ""
  word = "\n".join(
    [
      "heard = []",
      "def ear():",
      "  while True:",
      "    if (a := (yield)) and a[0] == 'note':",
      "      heard.append(hearer.get())",
      "drive(ear(), 'ear')",
      "say('note', chain1)",
      "here = hearer.get()",
    ]
  )
  await engine.rung(word, on=root)
  await settle()
  got = (engine.module(root)["heard"], engine.module(root)["here"], engine.hearer.get())
  assert got == (["ear"], said(log, "rung")[-1][1], "")


async def test_a_fact_said_under_the_site_of_an_ear_while_another_ear_is_the_hearer_is_not_kept() -> None:
  """A fact said under the site of an ear while another ear is the hearer is said by the code that runs there, so the record does not keep it, and a later life says it again when it runs that code again."""
  sand, log, root = born()
  word = "\n".join(
    [
      "with site.set('world'):",
      "  say('note', chain1, 'world')",
      "x = await bash('true')",
      "with site.set('kernel'):",
      "  say('note', chain1, 'kernel')",
      "y = await bash('true')",
    ]
  )
  await engine.rung(word, on=root)
  engine.pause(root)
  await settle()
  notes = [e[0] for e in sand.record if e[0][0] == "note"]
  outs = [e[0][:3] for e in sand.record if e[0][0] == "out"]
  paused = [e[0][:3] for e in sand.record if e[0][0] == "pause"]
  assert (notes, outs, paused) == (
    [],
    [("out", "bash1", WORLD), ("out", "bash2", WORLD)],
    [("pause", root, "operator")],
  )
  log, root = await relived(sand, sand.record)
  assert [a[3] for a in said(log, "note")] == ["world", "kernel"]
  assert engine.peek("bash2") is not None
  assert engine.paused(root)
