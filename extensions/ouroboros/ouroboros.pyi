from furb.engine import Text

def enabled() -> list[str]:
  """enabled gives the name of each extension that the life runs, once, in the order the life enabled it, as the enable facts of the root say, and it asks nothing."""

def bound_by(on: str = "") -> dict[str, str]:
  """bound_by gives the names that the words of a chain bind at their top level, a def, a class, an assignment or a quote, each with the rung whose word bound it last, and it asks nothing.
  The words of the extensions that play on the chain count, each under the rung that played it, and a name that the engine defines is not there unless a word binds it again.
  """

def extend(name: str, *rungs: str, tui: str = "", session: bool = False) -> Text:
  """extend writes the words of the rungs it names, in order, as the program of the chain holds them, with a blank line between two, after the word that the file holds, as the word of the extension of that name in .furb/extensions/<name>, with its manifest SKILL.md, whose metadata names that word under furb-word and which keeps each other line that it holds, and gives the word as it landed.
  extend writes tui, when it is given, as tui.ts in the folder of the extension, the part that the TUI loads, and the manifest names tui.ts under furb-view of its metadata whenever the folder holds that file.
  extend turns the extension on in .furb/config.json with the path of its folder from the folder of that config, and keeps every other entry of the config.
  When a write of the config of the project lands, the World enables each extension that it turns on by its path and that the life does not run, so it plays as a rung on every chain there is and on each chain at its birth, and the record keeps the fact for every later life.
  When extend writes again the word of an extension that the life runs, the World enables it again with the new word, and each chain plays the definitions of that word alone, so a function and a name in capitals take their new value and every other name keeps its value.
  extend with session keeps the kit in the record alone: it writes no file, says the fact enable of the root with the word, so each chain plays it as an extension, and gives the word at session://<name>/<name>.py.
  A later life on the record plays the kit on each chain from that fact, with no file in the project or the config.
  """
