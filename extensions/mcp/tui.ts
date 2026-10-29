import type { Command, Completion, Furb, Item, Line, Seen, Tone } from "../../tui/src/parts.ts";

/** A tool of a server, as `mcp_servers` gives it. */
interface Tool {
  name: string;
  title?: string | null;
  description?: string | null;
  input?: Schema | null;
  output?: Schema | null;
  annotations?: Record<string, unknown> | null;
}
/** A JSON schema, of which the part reads the properties, the type, the enum and the description. */
interface Schema {
  type?: string | string[];
  properties?: Record<string, Schema>;
  required?: string[];
  items?: Schema;
  enum?: unknown[];
  anyOf?: Schema[];
  oneOf?: Schema[];
  description?: string;
}
/** A resource of a server. */
interface Resource {
  uri: string;
  name?: string | null;
  description?: string | null;
  mime?: string | null;
}
/** A prompt of a server, with its arguments. */
interface Prompt {
  name: string;
  description?: string | null;
  arguments?: { name: string; description?: string | null; required?: boolean | null }[] | null;
}
/** A server, as `mcp_servers` and `mcp_control` give it. */
interface Server {
  name: string;
  scope?: string;
  transport?: string;
  target?: string;
  state?: string;
  error?: string | null;
  version?: string | null;
  instructions?: string | null;
  tools?: Tool[];
  resources?: Resource[];
  templates?: { uri: string; name?: string | null; description?: string | null }[];
  prompts?: Prompt[];
}

/** The tone of each state of a server. */
const tones: Record<string, Tone> = {
  connected: "success",
  failed: "danger",
  "needs-auth": "warning",
  unapproved: "warning",
  disabled: "faint",
  pending: "muted",
};

/** The actions on a server, by their key: what each does, and the states of a server that it serves. */
const actions = [
  {
    key: "r",
    action: "reconnect",
    label: "reconnect",
    when: ["connected", "failed", "needs-auth", "pending"],
  },
  { key: "e", action: "enable", label: "enable", when: ["disabled"] },
  {
    key: "d",
    action: "disable",
    label: "disable",
    when: ["connected", "failed", "needs-auth", "pending", "unapproved"],
  },
  { key: "a", action: "approve", label: "approve", when: ["unapproved"] },
  { key: "l", action: "login", label: "log in", when: ["needs-auth", "connected", "failed"] },
  { key: "o", action: "logout", label: "log out", when: ["connected", "needs-auth", "failed"] },
] as const;
/** The lists of a server, by their key. */
const lists = [
  { key: "t", label: "tools" },
  { key: "s", label: "resources" },
  { key: "p", label: "prompts" },
] as const;

/** A str of python with the text of a string, which JSON writes as python reads it. */
const py = (text: string) => JSON.stringify(text);

/** A dict of python of str values, which JSON writes as python reads it. */
const pyDict = (values: Record<string, string>) => JSON.stringify(values);

/** A count of things, with the plural of its noun. */
const many = (count: number, noun: string) => `${count} ${noun}${count === 1 ? "" : "s"}`;

/** The first line of a text, cut. */
const gist = (text: unknown, size = 120) => {
  const line =
    String(text ?? "")
      .trim()
      .split("\n")[0] ?? "";
  return line.length > size ? `${line.slice(0, size - 1)}…` : line;
};

/** The text of the error of an act that failed: the first argument of the exception that it holds. */
const fault = (value: unknown) => {
  const args = (value as { args?: unknown[] } | null)?.args;
  return String(args?.[0] ?? (value as { is?: unknown } | null)?.is ?? "It failed.");
};

/** Whether a value is an exception that an act holds. */
const raised = (value: unknown) =>
  Boolean(value && typeof value === "object" && "is" in value && "args" in value);

/** The servers as the record holds them now: the list of the last `mcp_servers` that is done, with each later
 * `mcp_control` that is done in the place of its server. */
export function servers(seen: Seen): Server[] {
  const asked = seen.of("mcp_servers").filter((act) => act.done && Array.isArray(act.value));
  const last = asked.at(-1);
  if (!last) return [];
  const held = new Map((last.value as Server[]).map((one) => [one.name, one]));
  for (const act of seen.of("mcp_control"))
    if (
      act.done &&
      act.began > last.began &&
      act.value &&
      typeof act.value === "object" &&
      !raised(act.value)
    ) {
      const one = act.value as Server;
      if (typeof one.name === "string") held.set(one.name, one);
    }
  return [...held.values()];
}

/** The type of a schema in a few words, as python names it. */
function typeOf(schema: Schema | undefined): string {
  if (!schema) return "any";
  if (schema.enum) return schema.enum.map((one) => JSON.stringify(one)).join(" | ");
  const union = schema.anyOf ?? schema.oneOf;
  if (union) return union.map(typeOf).join(" | ");
  const names: Record<string, string> = {
    string: "str",
    integer: "int",
    number: "float",
    boolean: "bool",
    null: "None",
    object: "dict",
  };
  const kinds = [schema.type ?? "any"].flat();
  return kinds
    .map((kind) => (kind === "array" ? `list[${typeOf(schema.items)}]` : (names[kind] ?? kind)))
    .join(" | ");
}

/** The signature of a tool, as `server.tool(name: type, ...)`, its required parameters first. */
function signature(server: string, tool: Tool): string {
  const properties = tool.input?.properties ?? {};
  const required = new Set(tool.input?.required ?? []);
  const names = Object.keys(properties).sort(
    (one, other) => Number(required.has(other)) - Number(required.has(one)),
  );
  const parameters = names.map(
    (name) => `${name}: ${typeOf(properties[name])}${required.has(name) ? "" : " = None"}`,
  );
  return `${server}.${tool.name}(${parameters.join(", ")})`;
}

/** The lines of a server under it while it is selected: its target, its version, its error, what it gives, and the
 * first line of its instructions. */
function details(one: Server): Line[] {
  const lines: Line[] = [
    [
      ["  ", "faint"],
      [one.target ?? "", "muted"],
    ],
  ];
  if (one.version)
    lines.push([
      ["  version ", "faint"],
      [String(one.version), "muted"],
    ]);
  if (one.error)
    lines.push([
      ["  ", "faint"],
      [gist(one.error, 200), "danger"],
    ]);
  const counts = [
    many(one.tools?.length ?? 0, "tool"),
    many(one.resources?.length ?? 0, "resource"),
    many(one.prompts?.length ?? 0, "prompt"),
  ];
  lines.push([
    ["  ", "faint"],
    [counts.join(", "), "muted"],
  ]);
  if (one.instructions)
    lines.push([
      ["  ", "faint"],
      [gist(one.instructions, 200), "faint"],
    ]);
  return lines;
}

/** The dialog of the servers: each with its state, its scope, its transport, what it gives and its error, and an action
 * on the selected one by its key, or Enter for the menu of the server. */
async function dialog(seen: Seen, selected = ""): Promise<void> {
  const all = servers(seen);
  if (!all.length) {
    seen.say(
      "No MCP server is configured: add one in .mcp.json, .furb/mcp.json or the mcp.json of the config dir.",
    );
    return;
  }
  const items = all.map(
    (one): Item<Server> => ({
      label: one.name,
      detail: [one.state, one.scope, one.transport, many(one.tools?.length ?? 0, "tool")]
        .filter(Boolean)
        .join(" · "),
      tone: tones[one.state ?? ""] ?? "text",
      lines: details(one),
      value: one,
    }),
  );
  const chosen = await seen.choose("MCP servers", items, {
    note: "Enter opens the menu of a server.",
    actions: [...actions, ...lists].map(({ key, label }) => ({ key, label })),
    selected: Math.max(
      0,
      all.findIndex((one) => one.name === selected),
    ),
  });
  if (!chosen) return;
  await act(seen, chosen.value, chosen.action);
}

/** Does what a key or the menu chose on a server: an action of `mcp_control`, a list, or the menu itself for Enter. */
async function act(seen: Seen, one: Server, key: string): Promise<void> {
  if (key === "t") return tools(seen, one);
  if (key === "s") return resources(seen, one);
  if (key === "p") return prompts(seen, one);
  const action = actions.find((each) => each.key === key);
  if (!action) return menu(seen, one);
  if (action.action === "approve") {
    const yes = await seen.confirm(
      `Approve ${one.name}`,
      `The project names the MCP server ${one.name}, which runs \`${one.target ?? ""}\` with your rights. Approve it only when you trust the project.`,
      { yes: "Approve and connect", no: "Leave it off" },
    );
    if (!yes) return dialog(seen, one.name);
  }
  seen.say(`MCP: ${action.label} ${one.name}…`);
  try {
    const got = (await seen.run(`mcp_control(${py(action.action)}, ${py(one.name)})`)) as Server | undefined;
    const state = got?.state ?? "done";
    seen.say(`MCP: ${one.name} is ${state}${got?.error ? `: ${gist(got.error)}` : ""}.`);
  } catch (error) {
    seen.say(
      `MCP: ${action.label} ${one.name} failed: ${gist(error instanceof Error ? error.message : error, 300)}`,
    );
  }
  return dialog(seen, one.name);
}

/** The menu of a server: each action that serves its state, and its lists. */
async function menu(seen: Seen, one: Server): Promise<void> {
  const items: Item<string>[] = [
    ...actions
      .filter((each) => (each.when as readonly string[]).includes(one.state ?? ""))
      .map((each) => ({
        label: each.label[0]?.toUpperCase() + each.label.slice(1),
        detail: `${each.key}`,
        value: each.key,
      })),
    { label: "Tools", detail: many(one.tools?.length ?? 0, "tool"), value: "t" },
    { label: "Resources", detail: many(one.resources?.length ?? 0, "resource"), value: "s" },
    { label: "Prompts", detail: many(one.prompts?.length ?? 0, "prompt"), value: "p" },
  ];
  const chosen = await seen.choose(`${one.name}: ${one.state ?? ""}`, items, {
    note: [one.scope, one.transport, one.target].filter(Boolean).join(" · "),
  });
  if (chosen) await act(seen, one, chosen.value);
}

/** The tools of a server: each with its description and its signature, and Enter for its parameters. */
async function tools(seen: Seen, one: Server): Promise<void> {
  const all = one.tools ?? [];
  const chosen = await seen.choose(
    `Tools of ${one.name}`,
    all.map(
      (tool): Item<Tool> => ({
        label: tool.name,
        detail: gist(tool.title || tool.description),
        lines: [
          [
            ["  ", "faint"],
            [signature(one.name, tool), "accent"],
          ],
          ...String(tool.description ?? "")
            .split("\n")
            .filter((line) => line.trim())
            .slice(0, 6)
            .map(
              (line): Line => [
                ["  ", "faint"],
                [line, "muted"],
              ],
            ),
          ...hints(tool),
        ],
        value: tool,
      }),
    ),
    {
      note: `${many(all.length, "tool")}. A word calls each as await mcp_call(server, tool, arguments). Enter shows its parameters.`,
    },
  );
  if (!chosen) return dialog(seen, one.name);
  return schema(seen, one, chosen.value);
}

/** The lines of the annotations of a tool that say what it may do. */
function hints(tool: Tool): Line[] {
  const said = tool.annotations ?? {};
  const marks = [
    said.readOnlyHint ? "reads only" : "",
    said.destructiveHint ? "may destroy" : "",
    said.idempotentHint ? "idempotent" : "",
    said.openWorldHint ? "reaches the world" : "",
  ].filter(Boolean);
  return marks.length
    ? [
        [
          ["  ", "faint"],
          [marks.join(", "), "warning"],
        ],
      ]
    : [];
}

/** The parameters of a tool, each with its type, whether it is needed, and its description, then its output. */
async function schema(seen: Seen, one: Server, tool: Tool): Promise<void> {
  const properties = tool.input?.properties ?? {};
  const required = new Set(tool.input?.required ?? []);
  const items = Object.entries(properties).map(
    ([name, part]): Item<string> => ({
      label: name,
      detail: `${typeOf(part)}${required.has(name) ? ", needed" : ""}`,
      tone: required.has(name) ? "text" : "muted",
      lines: String(part.description ?? "")
        .split("\n")
        .filter((line) => line.trim())
        .slice(0, 8)
        .map(
          (line): Line => [
            ["  ", "faint"],
            [line, "muted"],
          ],
        ),
      value: name,
    }),
  );
  if (tool.output) items.push({ label: "returns", detail: typeOf(tool.output), tone: "faint", value: "" });
  await seen.choose(signature(one.name, tool), items, { note: gist(tool.description, 300) });
  return tools(seen, one);
}

/** The resources of a server: Enter puts a mention of the chosen one in the input. */
async function resources(seen: Seen, one: Server): Promise<void> {
  const all = one.resources ?? [];
  const templates = one.templates ?? [];
  const chosen = await seen.choose(
    `Resources of ${one.name}`,
    [
      ...all.map(
        (resource): Item<string> => ({
          label: resource.name || resource.uri,
          detail: [resource.mime, resource.uri].filter(Boolean).join(" · "),
          lines: resource.description
            ? [
                [
                  ["  ", "faint"],
                  [gist(resource.description, 300), "muted"],
                ],
              ]
            : [],
          value: resource.uri,
        }),
      ),
      ...templates.map(
        (template): Item<string> => ({
          label: template.name || template.uri,
          detail: `template · ${template.uri}`,
          tone: "muted",
          lines: template.description
            ? [
                [
                  ["  ", "faint"],
                  [gist(template.description, 300), "muted"],
                ],
              ]
            : [],
          value: template.uri,
        }),
      ),
    ],
    { note: "Enter puts @server:uri in the input, which tells the model to read the resource." },
  );
  if (!chosen) return dialog(seen, one.name);
  seen.draft(`@${one.name}:${chosen.value} `);
}

/** The prompts of a server: Enter runs the chosen one, as its command does. */
async function prompts(seen: Seen, one: Server): Promise<void> {
  const all = one.prompts ?? [];
  const chosen = await seen.choose(
    `Prompts of ${one.name}`,
    all.map(
      (each): Item<Prompt> => ({
        label: `/${command(one.name, each.name)}`,
        detail: gist(each.description),
        lines: (each.arguments ?? []).map(
          (argument): Line => [
            ["  ", "faint"],
            [argument.name, argument.required ? "text" : "muted"],
            [argument.description ? `  ${gist(argument.description)}` : "", "faint"],
          ],
        ),
        value: each,
      }),
    ),
  );
  if (!chosen) return dialog(seen, one.name);
  await prompt(seen, one.name, chosen.value, "");
}

/** The name of the command of a prompt of a server, as Claude Code names it. */
const command = (server: string, name: string) => `mcp__${server}__${name}`.replace(/[\s/]+/g, "_");

/** Runs a prompt of a server: the words after its command fill its arguments in order, a form asks each argument that
 * they leave out, and the messages that the prompt gives go to the chain as a message of the operator. */
async function prompt(seen: Seen, server: string, one: Prompt, argument: string): Promise<void> {
  const wanted = one.arguments ?? [];
  const given = argument.trim() ? argument.trim().split(/\s+/) : [];
  // The last argument takes each word that is left, so a prompt of one argument takes the whole text.
  const values: Record<string, string> = {};
  for (const [index, each] of wanted.entries())
    values[each.name] = index === wanted.length - 1 ? given.slice(index).join(" ") : (given[index] ?? "");
  let filled: Record<string, string> | undefined = values;
  if (wanted.some((each) => each.required && !values[each.name]) || (wanted.length && !given.length)) {
    filled = await seen.form(
      `/${command(server, one.name)}`,
      wanted.map((each) => ({
        name: each.name,
        label: each.name,
        detail: each.description ?? "",
        value: values[each.name] ?? "",
        required: Boolean(each.required),
      })),
      { note: gist(one.description, 300) },
    );
    if (!filled) return;
  }
  const kept = Object.fromEntries(Object.entries(filled).filter(([, value]) => value !== ""));
  const got = (await seen.run(`mcp_prompt(${py(server)}, ${py(one.name)}, ${pyDict(kept)})`)) as
    | { text?: string }
    | undefined;
  const text = String(got?.text ?? "").trim();
  if (!text) {
    seen.say(`MCP: the prompt ${one.name} of ${server} gave no text.`);
    return;
  }
  await seen.send(text);
}

/** The lines of a text, cut to a number of lines, which say how many they leave out. */
function cut(text: string, most: number, tone: Tone): Line[] {
  const lines = text.split("\n");
  const shown = lines.slice(0, most).map((line): Line => [[line, tone]]);
  if (lines.length > most) shown.push([[`${lines.length - most} more lines`, "faint"]]);
  return shown;
}

/** The part of the extension mcp: the command /mcp, which opens the dialog of the servers and their actions, a status
 * in the footer while servers are configured, the row of each call of a tool, a command for each prompt of each
 * connected server, and the resources of the servers after `@` in the input. It reads the servers from the acts that
 * the record holds, and acts through words that the record keeps. */
export default function mcp(furb: Furb): void {
  furb.command("mcp", "Manage the MCP servers: state, tools, resources, prompts, login", (_, seen) =>
    dialog(seen),
  );
  furb.status((seen) => {
    const all = servers(seen);
    if (!all.length) return "";
    const count = (state: string) => all.filter((one) => one.state === state).length;
    const failed = count("failed");
    const auth = count("needs-auth");
    const waits = count("unapproved");
    return [
      [`MCP: ${many(all.length, "server")}`, "muted"],
      [failed ? `, ${failed} failed` : "", "danger"],
      [auth ? `, ${auth} ${auth === 1 ? "needs" : "need"} login` : "", "warning"],
      [waits ? `, ${waits} unapproved` : "", "warning"],
    ];
  });
  furb.row("mcp_call", (act) => {
    const [server, tool, args] = act.words as [unknown, unknown, unknown];
    const given = args && typeof args === "object" ? JSON.stringify(args) : "";
    const label = `${String(server)}.${String(tool)}(${given.length > 80 ? `${given.slice(0, 79)}…` : given})`;
    const lines: Line[] = [];
    if (given)
      lines.push([
        ["arguments ", "faint"],
        [JSON.stringify(args, null, 2), "muted"],
      ]);
    if (!act.done) lines.push([["The call runs.", "faint"]]);
    else if (raised(act.value)) lines.push(...cut(fault(act.value), 20, "danger"));
    else {
      const value = act.value as { text?: unknown; content?: { type?: string; image?: string }[] } | null;
      lines.push(...cut(String(value?.text ?? ""), 40, "text"));
      for (const block of value?.content ?? [])
        if (block?.type === "image")
          lines.push([
            ["image ", "faint"],
            [String(block.image ?? ""), "accent"],
          ]);
    }
    return { label, details: lines };
  });
  furb.commands((seen) =>
    servers(seen)
      .filter((one) => one.state === "connected")
      .flatMap((one) =>
        (one.prompts ?? []).map(
          (each): Command => ({
            name: command(one.name, each.name),
            detail: gist(each.description) || `The prompt ${each.name} of the MCP server ${one.name}`,
            run: (argument, later) => prompt(later, one.name, each, argument),
          }),
        ),
      ),
  );
  furb.complete("@", (text, seen) => {
    const wanted = text.toLowerCase();
    return servers(seen)
      .filter((one) => one.state === "connected")
      .flatMap((one) =>
        (one.resources ?? []).map(
          (resource): Completion => ({
            label: `@${one.name}:${resource.uri}`,
            detail: gist(resource.name || resource.description, 60),
            text: `@${one.name}:${resource.uri} `,
          }),
        ),
      )
      .filter((one) => one.label.slice(1).toLowerCase().includes(wanted))
      .slice(0, 50);
  });
}
