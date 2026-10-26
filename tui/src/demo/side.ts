import type { Script } from "./script.ts";

/** The files of the other projects of the demo, which each hold what a real project of its kind holds, small. */
export const projects: Record<string, Record<string, string>> = {
  atlas: {
    "README.md":
      "# Atlas\n\nMap tiles for a web page.\n\n- Fetch a tile\n- Keep it for later\n- Work with no network\n",
    "src/tiles.ts":
      "export interface Tile {\n  z: number;\n  x: number;\n  y: number;\n}\n\nexport const url = (tile: Tile) => `https://tiles.example.com/${tile.z}/${tile.x}/${tile.y}.png`;\n\nexport async function fetchTile(tile: Tile): Promise<Uint8Array> {\n  const response = await fetch(url(tile));\n  if (!response.ok) throw new Error(`tile ${tile.z}/${tile.x}/${tile.y}: ${response.status}`);\n  return new Uint8Array(await response.arrayBuffer());\n}\n",
    "src/cache.ts":
      'import type { Tile } from "./tiles.ts";\n\nconst kept = new Map<string, Uint8Array>();\nconst key = (tile: Tile) => `${tile.z}/${tile.x}/${tile.y}`;\n\nexport const get = (tile: Tile) => kept.get(key(tile));\nexport const put = (tile: Tile, bytes: Uint8Array) => void kept.set(key(tile), bytes);\n',
    "test/tiles.test.ts":
      'import { expect, test } from "bun:test";\nimport { url } from "../src/tiles.ts";\n\ntest("a tile has one address", () => {\n  expect(url({ z: 3, x: 4, y: 5 })).toBe("https://tiles.example.com/3/4/5.png");\n});\n',
  },
  "marlin-api": {
    "README.md":
      "# Marlin API\n\nThe service behind the Marlin app.\n\n- GET /boats\n- GET /boats/:id\n- POST /boats\n",
    "src/routes.ts":
      'export const routes = [\n  { method: "GET", path: "/boats" },\n  { method: "GET", path: "/boats/:id" },\n  { method: "POST", path: "/boats" },\n];\n\nexport function list(page = 1, size = 20) {\n  const all = Array.from({ length: 95 }, (_, id) => ({ id, name: `boat ${id}` }));\n  return all.slice(page * size, (page + 1) * size);\n}\n',
    "src/limits.ts":
      "const hits = new Map<string, number[]>();\n\nexport function allow(client: string, now = Date.now(), per = 60_000, most = 100): boolean {\n  const recent = (hits.get(client) ?? []).filter((at) => now - at < per);\n  recent.push(now);\n  hits.set(client, recent);\n  return recent.length <= most;\n}\n",
    "openapi.yaml":
      "openapi: 3.0.3\ninfo:\n  title: Marlin API\n  version: 1.4.0\npaths:\n  /boats:\n    get:\n      parameters:\n        - name: page\n          in: query\n          schema: { type: integer, minimum: 1 }\n      responses:\n        '200': { description: A page of boats }\n",
    "test/limits.test.ts":
      'import { expect, test } from "bun:test";\nimport { allow } from "../src/limits.ts";\n\ntest("the 101st call in a minute is refused", () => {\n  for (let i = 0; i < 100; i++) expect(allow("a", 1000 + i)).toBe(true);\n  expect(allow("a", 1100)).toBe(false);\n});\n',
  },
  website: {
    "README.md": "# Website\n\nThe pages of the project, built with one script.\n",
    "index.html":
      '<!doctype html>\n<html lang="en">\n<head><meta charset="utf-8"><title>Fieldnotes</title><link rel="stylesheet" href="site.css"></head>\n<body>\n  <h1>Fieldnotes</h1>\n  <p>A small place to keep ideas.</p>\n  <a href="pricing.html">Pricing</a>\n</body>\n</html>\n',
    "pricing.md": "# Pricing\n\n- Free: 100 notes\n- Plus: 5 dollars a month, no limit\n",
    "site.css":
      "body { font: 16px system-ui; margin: 2rem auto; max-width: 40rem; }\na { color: #0b57d0; }\n",
    "build.ts":
      'import { readFileSync, writeFileSync } from "node:fs";\n\nconst page = (title: string, body: string) =>\n  `<!doctype html><title>${title}</title><link rel="stylesheet" href="site.css">${body}`;\n\nwriteFileSync("pricing.html", page("Pricing", readFileSync("pricing.md", "utf8")));\n',
  },
};

const ranking = `def score(note, words):
  text = note.lower()
  return sum(text.count(w) for w in words) * 10 - len(text) // 40


notes = ["small ideas", "ideas about travel and ideas about food", "budget", "an idea"]
words = ["ideas"]
ranked = sorted(notes, key=lambda n: score(n, words), reverse=True)
for number, note in enumerate(ranked, 1):
  print(number, note, score(note, words))`;

const rankingEnd = `close("A note ranks higher for each time it holds a word of the query, and lower for each 40 characters of length. The two notes that hold ideas rank first.")`;

const guideReview = `guide = read("README.md")
problems = []
for number, line in enumerate(guide.lines, 1):
  if len(line) > 100:
    problems.append((number, "long line"))
  elif line.endswith(" "):
    problems.append((number, "trailing space"))
close(f"{len(guide.lines)} lines read, {len(problems)} problem(s)." + ("" if not problems else " First: line %d, %s." % problems[0]))`;

const coldStart = String.raw`import json

lines = []
size = 0
while size < 4000:
  lines.append(json.dumps({"text": f"note {size}", "at": size}))
  size += 1
write(Text(".notes/cold.jsonl", "\n".join(lines) + "\n"))
count = await bash("wc -l < .notes/cold.jsonl")
print(count.stdout.content.strip(), "notes to read at a start")`;

const coldEnd = `close("A start reads 4000 notes in one pass. The file is 80 KB, so the read is not the cost: parsing each line is. Keep the parsed notes in one array, as storage.ts does.")`;

const spike = `# SQLite needs a package that the project does not have, so I check before I write any code.
pack = read("package.json", HIDDEN)
if "sqlite" in pack.content:
  close("The package list names sqlite already.")
else:
  got = await bash("ls node_modules 2>&1 | head -3; echo 'no sqlite package in the project'")
  close("SQLite needs a new dependency, bun:sqlite aside. The file of lines needs none, so the spike stops here.")`;

const tiles = `def around(z, x, y):
  return [(z, x + dx, y + dy) for dx in (-1, 0, 1) for dy in (-1, 0, 1)]


wanted = around(3, 4, 5)
cache = read("src/cache.ts", HIDDEN)
print(len(wanted), "tiles around the view,", len(cache.lines), "lines of cache")
fresh = [tile for tile in wanted if tile[1] >= 0 and tile[2] >= 0]
close(f"{len(fresh)} of {len(wanted)} tiles are inside the map, and the cache holds {len(cache.lines)} lines.")`;

const tilesAsk = String.raw`answer = await prompt(bool, "Offline mode keeps 200 tiles, about 12 MB on a phone. Keep the 200 as the default?", to=OPERATOR)
if answer:
  write(read("src/cache.ts").append("export const MOST = 200;\n"))
close("The cache keeps at most 200 tiles." if answer else "The cache has no ceiling yet.")`;

const changelog = String.raw`log = Text("CHANGELOG.md", "# Changelog\n\n## 0.2.0\n\n- Tiles are kept in a cache.\n- A tile has one address.\n")
write(log)
close("CHANGELOG.md names the two changes of 0.2.0.")`;

const research = `apps = ["leaflet", "maplibre", "openlayers"]
seen = {}
for app in apps:
  seen[app] = await bash(f"echo {app}: tiles are cached by the browser, and a service worker keeps them for offline use")
print(len(seen), "apps looked at")
await wait(3600)`;

const rate = `def burst(client, n, start):
  ok = 0
  for i in range(n):
    ok += 1 if i < 100 else 0
  return ok


print(burst("a", 130, 1000), "of 130 calls pass")
limits = read("src/limits.ts")
print([n for n, line in enumerate(limits.lines, 1) if "now" in line])`;

const rateEnd = `close("A client gets 100 calls in a minute, and the 101st is refused until the oldest call is 60 seconds old. test/limits.test.ts proves the edge.")`;

const pages = `routes = read("src/routes.ts")
bug = [n for n, line in enumerate(routes.lines, 1) if "slice(page * size" in line]
print("the slice starts at page * size on line", bug)`;

const pagesFix = `routes = read("src/routes.ts")
fixed = routes.replace("all.slice(page * size, (page + 1) * size)", "all.slice((page - 1) * size, page * size)")
write(fixed)
run = await bash("sleep 300; echo 'tests pass'")
print(run.stdout.content)`;

const auth = `token = {"value": "abc", "expires": 1000}
now = 990
leeway = 30
while token["expires"] - now < leeway:
  token = {"value": token["value"] + "x", "expires": token["expires"] + 900}
  print("refreshed, expires at", token["expires"])
close("A token is refreshed 30 seconds before it ends, so a call never meets an old one.")`;

const openapi = `spec = read("openapi.yaml")
paths = [x for x in spec.lines if x.startswith("  /")]
params = [x for x in spec.lines if "- name:" in x]
print(paths, params)
close(f"{len(paths)} path(s) and {len(params)} parameter(s) in openapi.yaml. page has a minimum of 1, and routes.ts counts pages from 0: that is the bug of the pagination.")`;

const flaky = `runs = []
for n in range(5):
  runs.append(await bash("test $((" + str(n) + " % 3)) -ne 2 && echo pass || (echo 'fail: the 101st call is allowed' >&2; exit 1)"))
failed = [i for i, r in enumerate(runs) if r.code != 0]
print("failed runs:", failed)
raise Refused("the test fails 1 time in 3: " + repr(failed))`;

const pricing = String.raw`page = read("pricing.md")
rows = [x[2:] for x in page.lines if x.startswith("- ")]
table = "| Plan | Price |\n| --- | --- |\n" + "\n".join("| " + " | ".join(x.split(": ", 1)) + " |" for x in rows) + "\n"
write(Text("pricing.md", "# Pricing\n\n" + table))
close("The prices are a table now: " + str(len(rows)) + " plans.")`;

const docs = `built = await bash("bun build.ts 2>&1 | tail -3; ls *.html")
print(built.stdout.content)
html = read("index.html", HIDDEN)
close("The page builds. index.html links to pricing.html" + (" and holds a title." if "<title>" in html.content else "."))`;

const dark = String.raw`css = read("site.css")
write(css.append("@media (prefers-color-scheme: dark) { body { background: #111; color: #eee; } a { color: #8ab4f8; } }\n"))
close("site.css follows the color scheme of the system. The link keeps a contrast of 7 to 1.")`;

const blog = String.raw`posts = [("0.3", "Search that finds a word"), ("0.4", "Notes that stay")]
items = "".join(f"<li><a href='{v}.html'>{t}</a></li>" for v, t in posts)
write(Text("blog.html", "<!doctype html><title>Blog</title><ul>" + items + "</ul>\n"))
close("blog.html lists the " + str(len(posts)) + " posts, the newest last.")`;

const one = (cue: string, thinking: string, word: string, cost = 0.012): Script => ({
  cue,
  weight: { input: 2400, output: 240, cost },
  steps: [{ thinking, word }],
});

export const side: Script[] = [
  {
    cue: "Rank the notes",
    weight: { input: 2200, output: 260, cost: 0.014 },
    steps: [
      {
        thinking: "A note that says the word twice should come before a note that says it once.",
        word: ranking,
      },
      { thinking: "The order is right, so I report it.", word: rankingEnd },
    ],
  },
  one("Review the README of the guide", "A line that is too long is the first thing to check.", guideReview),
  {
    cue: "How long is a cold start with 4000 notes",
    weight: { input: 2100, output: 210, cost: 0.011 },
    steps: [
      { thinking: "I make 4000 notes and count them.", word: coldStart },
      { thinking: "The count is right, so I answer.", word: coldEnd },
    ],
  },
  one(
    "Spike: keep the notes in SQLite",
    "A new dependency is a ruling, so I check what the project has.",
    spike,
  ),
  one("Show the tiles around the view", "Nine tiles surround the one in the middle.", tiles),
  one("Offline mode", "A phone has little room, so the owner decides the ceiling.", tilesAsk),
  one("Write the changelog of 0.2.0", "Two changes, so two lines.", changelog),
  one(
    "Compare the offline tiles of three map libraries",
    "I look at three libraries, and wait for the owner.",
    research,
  ),
  {
    cue: "Rate limits",
    weight: { input: 2600, output: 250, cost: 0.016 },
    steps: [
      { thinking: "I count how many calls of a burst pass.", word: rate },
      { thinking: "The edge is the 101st call.", word: rateEnd },
    ],
  },
  {
    cue: "The list of boats repeats the last page",
    weight: { input: 2900, output: 300, cost: 0.02 },
    steps: [
      { thinking: "The slice starts at page * size, so page 1 skips the first page.", word: pages },
      { thinking: "I count pages from 1 and run the tests.", word: pagesFix },
    ],
  },
  one("Refresh the token before it ends", "A loop keeps the token fresh.", auth),
  one("Check openapi.yaml against the routes", "The spec and the routes must say the same.", openapi),
  one("The test of the limit is flaky", "Five runs show how often it fails.", flaky, 0.02),
  one("Make the prices a table", "The list has two plans.", pricing),
  one("Build the site and read the page", "The build writes the html.", docs),
  one("Follow the color scheme of the system", "One media query serves it.", dark),
  one("List the posts of the blog", "Two posts, newest last.", blog),
];
