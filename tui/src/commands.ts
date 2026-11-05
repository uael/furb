/** Names, labels, arguments, and help for every slash command. An argument in angle brackets is needed, and one in
 * square brackets may be left out. A last entry is another name that the command takes, and it completes to it. */
export const commands = {
  new: ["New session", "", "Start a fresh life"],
  watch: ["Watch a life", "", "Play a short life that asks no model and costs nothing"],
  model: ["Choose model", "[model]", "Choose the model that answers on this chain"],
  providers: ["Providers", "", "Add or remove a provider for this project"],
  provider: [
    "Change provider",
    "<add|remove> <name> [URL model window key_env protocol]",
    "Save a provider in the project settings and use it now",
  ],
  reload: ["Reload settings", "", "Read provider settings again and update the standing of this life"],
  skills: [
    "Reload skills",
    "",
    "Read the skills of the disk again; each shows in the list of / commands, and its command reads its SKILL.md",
    "reload-skills",
  ],
  retry: [
    "Retry now",
    "",
    "Ask again at once each model that waits to retry, as at a usage limit, on the provider settings of now",
  ],
  effort: ["Choose effort", "[level]", "Set the reasoning effort of the selected model"],
  shape: ["Response shape", "[type]", "Choose the result type of the next prompt"],
  theme: ["Color theme", "[name]", "Change the palette of every surface, or follow the terminal"],
  access: ["Access", "[reader|motion]", "Turn the screen reader mode or reduced motion on or off"],
  chain: ["New chain", "<label>", "A conversation with its own state"],
  fork: ["Fork chain", "<label>", "Copy this chain's current transcript and program"],
  tree: ["Session tree", "", "Browse the chains and their turns, and open or branch from one"],
  rewind: ["Rewind", "", "Go back to an earlier message or act of this chain, with its files or not"],
  undo: ["Undo message", "", "Remove the last user message from a new transcript branch"],
  redo: ["Redo message", "", "Return to the branch before undo"],
  queue: ["Queue follow-up", "[text]", "Send after current work, or manage queued messages"],
  files: ["Attach file", "", "Search project files for an @ reference"],
  image: ["Attach image", "[path]", "Attach an image file, or paste one from the clipboard"],
  editor: ["External editor", "", "Edit this draft with VISUAL or EDITOR"],
  run: ["Run Python", "<python>", "Write a rung through the gate"],
  bash: ["Run command", "<command>", "Stream a shell command"],
  read: ["Read file", "<path>", "Show a file to this chain"],
  changes: [
    "Review changes",
    "[turn|chain|life|branch]",
    "Show the files that this turn, this chain, the life or the branch against main changed",
  ],
  repair: [
    "Repair the journal",
    "",
    "Keep each change that the journal of file changes can read, and remove the lines that it cannot",
  ],
  renew: [
    "Start a new journal",
    "",
    "Start a new journal of file changes, and keep the damaged one beside it",
  ],
  cd: ["Change directory", "<path>", "Change this chain's working directory"],
  edit: ["Edit program", "[prompt id]", "Change a prompt's program and replay it"],
  inspect: ["Inspect a name", "<name>", "Read a value from this chain's module"],
  details: ["Details", "", "Expand or collapse an act in the current view"],
  grant: ["Set budget", "<dollars>", "Pause at a dollar ceiling"],
  context: ["Set context ceiling", "<fraction>", "Pause at a share of the model window"],
  status: [
    "Status",
    "",
    "Show the actor, the context, the directory, the budget and the MCP servers of this chain",
  ],
  copy: ["Copy the answer", "", "Copy the last answer of this chain to the clipboard"],
  pause: ["Pause", "[act or chain]", "Hold delivery while work completes"],
  wake: ["Wake", "[act or chain]", "Wake every paused chain, or one"],
  cancel: ["Cancel work", "[id]", "End an act, or what you sent last on this chain"],
  feed: ["Feed command", "<id> [text]", "Send a line of input to a command; no text closes its input"],
  close: ["Answer an act", "<id> <json>", "Close an act with a JSON value"],
  name: ["Name this session", "<name>", "Set a name in the session picker", "rename"],
  share: ["Share conversation", "[path]", "Export a standalone HTML conversation"],
  export: ["Export transcript", "<path>", "Write the transcript and program to a new JSON file"],
  workspace: ["Open workspace", "[path]", "Choose a workspace or add a project folder", "resume"],
  view: ["View of the life", "[main|loom|graph]", "Show the life as main, the loom, or the graph"],
  sidebar: [
    "Toggle sidebar",
    "",
    "Show or hide the workspaces with the threads of the session shown and the context",
  ],
  delete: ["Delete session", "", "Archive a session, or move it and its files to the workspace trash"],
  newthread: [
    "New thread",
    "",
    "Start a new thread with your next message, in place of a message to the thread that the input targets, as `+ New thread` does",
  ],
  autocollapse: ["Automatic fold", "", "Toggle the fold of the threads that are over"],
  extensions: [
    "Extensions",
    "",
    "List the extensions that this life runs, with what their parts give the TUI",
  ],
  serve: [
    "Serve to operators",
    "[address]",
    "Give the addresses at which other operators read this life, or read and drive it, over the network",
  ],
  end: ["End the lives", "", "End each open life, its commands and its models, then quit"],
  reopen: [
    "Open the life again",
    "",
    "Reach the life again after it ended in another window or its host was lost, in a host of this machine",
  ],
  stop: [
    "Stop the host",
    "",
    "End by force the process that holds this life, when the end of the life does not reach it",
  ],
  help: ["Help", "", "Keys, marks, and slash commands"],
  exit: ["Exit", "", "Save every session and quit, while a life in a host of its own goes on", "quit"],
} as const satisfies Record<string, readonly [string, string, string, string?]>;
const typed = (name: string, argument: string) => `/${name}${argument ? ` ${argument}` : ""}`;
/** Each slash command with its name, its other name or none, and as it is typed with its arguments: `usage` by its
 * name, `aliasUsage` by its other name, and `forms` by each name that it takes. */
export const slashes = Object.entries<readonly [string, string, string, string?]>(commands).map(
  ([name, [label, argument, detail, alias = ""]]) => ({
    name,
    label,
    argument,
    detail,
    alias,
    usage: typed(name, argument),
    aliasUsage: alias ? typed(alias, argument) : "",
    forms: alias ? `${typed(name, argument)} or ${typed(alias, argument)}` : typed(name, argument),
  }),
);

/** The text with the other name of a command, when it starts with one, written as the name of the command. */
export const canonical = (text: string): string => {
  const [, word = "", rest = ""] = text.match(/^\/([\w-]+)( .*|)$/s) ?? [];
  const found = word ? slashes.find(({ alias }) => alias === word) : undefined;
  return found ? `/${found.name}${rest}` : text;
};
