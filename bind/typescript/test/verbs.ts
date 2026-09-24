import { type Life, unwrapped } from "../src/index.ts";

/** A verb said by the operator on a chain, the root when the words name none: a verb of the engine, or one that an
 * extension bound on that chain. */
export function verb<T = unknown>(
  life: Life,
  name: string,
  args: unknown[] = [],
  kwargs: Record<string, unknown> = {},
): T {
  return life.call<T>(name, args, { on: life.root, ...kwargs });
}

/** What a name that the module of a chain binds crosses as: a callable of an extension, as `{is: "made", id}`. */
export function bound<T = unknown>(life: Life, name: string, on = life.root): T {
  return life.held<T>("modules", [on, name], "at");
}

/** A text of the files extension, as its plain data. */
export interface Text {
  path: string;
  content: string;
  before: string | null;
}

/** The text at a path that the read of the files extension gives, as its plain data. */
export function read(life: Life, path: string, on = life.root): Text {
  return unwrapped(life.call("read", [path], { on }));
}

/** A write of a content to a path, asked of the life as the write of the files extension asks it, and what the World
 * answered. */
export function write(
  life: Life,
  path: string,
  content: string,
  on = life.root,
): { path: string; content: string } {
  return life.call<[unknown, { path: string; content: string }]>("ask", ["write", on, path, content], {})[1];
}

/** The exit of a command, as its plain data. */
export interface Exit {
  code: number | null;
  stdout: Text;
  stderr: Text;
}

/** A command that the bash extension runs, by the name of its act. */
export function bash(
  life: Life,
  command: string,
  kwargs: Record<string, unknown> = {},
  on = life.root,
): string {
  return life.call<string>("bash", [command], { on, ...kwargs });
}

/** What a command came to, as its plain data. */
export async function exited(life: Life, id: string): Promise<Exit> {
  return unwrapped<Exit>(await life.result(id));
}
