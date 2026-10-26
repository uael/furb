import { mkdir, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { demoSession, seedDemoFiles } from "../demo.ts";
import type { Preferences } from "../preferences.ts";
import type { Session } from "../session.ts";
import type { SessionEntry, Workspace, Workspaces } from "../workspaces.ts";
import { quiet } from "./life.ts";
import { projects } from "./side.ts";

/** A new project of the demo in a folder: the notes app, or one of the other projects that a developer keeps beside it. */
export async function seedProject(directory: string, name: string): Promise<void> {
  const files = projects[name];
  if (!files) return seedDemoFiles(directory);
  for (const [path, content] of Object.entries(files)) {
    await mkdir(dirname(join(directory, path)), { recursive: true });
    await writeFile(join(directory, path), content);
  }
}

/** The sessions that stand on disk, done: each is a record that a past window left, which the sidebar lists with no
 * life behind it. */
const kept: Record<string, [name: string, message: string][]> = {
  fieldnotes: [
    [
      "Search ranking",
      "Rank the notes: a note that holds the word twice comes before a note that holds it once.",
    ],
    ["Guide review", "Review the README of the guide and say what is wrong with it."],
    ["Cold start", "How long is a cold start with 4000 notes? Count them, and say where the time goes."],
    ["Spike: SQLite", "Spike: keep the notes in SQLite, and say if it is worth the new dependency."],
  ],
  atlas: [
    ["Tile cache", "Show the tiles around the view at zoom 3, and say how many fall inside the map."],
    ["Archive", "Write the changelog of 0.2.0."],
  ],
  "marlin-api": [
    ["Rate limits", "Rate limits: how many calls of a burst of 130 pass in one minute?"],
    ["Auth refresh", "Refresh the token before it ends, so that no call meets an old one."],
    ["OpenAPI check", "Check openapi.yaml against the routes, and say where they differ."],
  ],
  website: [
    ["Pricing page", "Make the prices a table."],
    ["Docs build", "Build the site and read the page."],
    ["Dark mode", "Follow the color scheme of the system."],
    ["Blog feed", "List the posts of the blog."],
  ],
};

/** The sessions that run in a window of the demo, in the state that a real day leaves them. */
const running: Record<
  string,
  [name: string, message: string, state: "done" | "blocked" | "working" | "paused" | "error"][]
> = {
  atlas: [
    ["Changelog", "Write the changelog of 0.2.0.", "done"],
    [
      "Research",
      "Compare the offline tiles of three map libraries, and keep going until the owner says stop.",
      "paused",
    ],
    ["Offline mode", "Offline mode: keep tiles for a phone, with a ceiling.", "blocked"],
  ],
  "marlin-api": [
    [
      "Pagination bug",
      "The list of boats repeats the last page: find the cause, fix it, and run the tests.",
      "working",
    ],
    ["Flaky test", "The test of the limit is flaky: run it five times and say why.", "error"],
  ],
};

const slug = (name: string) => name.toLowerCase().replace(/[^a-z0-9]+/g, "-");

/** The workspaces of one developer: the notes app, whose life is the dense one, and three more projects, each with
 * sessions that a past window left, and sessions that run now. */
export async function seedWorkspaces({
  library,
  preferences,
  directory,
}: {
  library: Workspaces;
  preferences: Preferences;
  /** The folder of a project of the demo, new, which this puts the files of the project in. */
  directory: (name: string) => Promise<string>;
}): Promise<{ groups: Record<string, Workspace>; entries: Record<string, SessionEntry> }> {
  const folders: Record<string, string> = {};
  for (const name of ["fieldnotes", "atlas", "marlin-api", "website"]) {
    folders[name] = await directory(name);
    await seedProject(folders[name], name);
    for (const [title, message] of kept[name] ?? []) {
      const past = await demoSession({
        cwd: folders[name],
        record: join(folders[name], ".furb/sessions", `${slug(title)}.jsonl`),
        preferences,
      });
      await past.command(`/name ${title}`);
      await past.engine.result(await past.engine.prompt("str", { message, on: past.engine.root }));
      // The cost of the answer reaches the state of the session a moment after the answer, and the list reads it there.
      await quiet(past, 500, 15_000);
      await past.dispose();
    }
  }
  const groups: Record<string, Workspace> = {};
  const entries: Record<string, SessionEntry> = {};
  const work: [
    session: Session,
    message: string,
    state: "done" | "blocked" | "working" | "paused" | "error",
  ][] = [];
  for (const name of Object.keys(folders)) {
    const group = await library.add(folders[name] ?? "");
    groups[name] = group;
    for (const [title, message, state] of running[name] ?? []) {
      const entry = await library.create(group, title);
      if (!entry.session) throw new Error(`The session ${title} did not open.`);
      entries[title] = entry;
      work.push([entry.session, message, state]);
    }
  }
  // Each session works while another one is the current one, so that a finished session reads as unread.
  for (const [session, message, state] of work) await drive(session, message, state);
  return { groups, entries };
}

async function drive(
  session: Session,
  message: string,
  state: "done" | "blocked" | "working" | "paused" | "error",
): Promise<void> {
  const { engine } = session;
  await engine.prompt("str", { message, on: engine.root });
  await quiet(session, 900, 30_000);
  if (state === "paused") await session.submit("/pause");
}
