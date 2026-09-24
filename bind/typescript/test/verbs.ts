import type { Streams } from "../src/builtin/bash.ts";
import type { Life, World } from "../src/index.ts";

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

/** The text at a path that the read of the files extension gives. */
export function read(life: Life, path: string) {
  return verb<{ is: "Text"; path: string; content: string; before: string | null }>(life, "read", [path]);
}

/** A write of a content to a path, asked of the World as the write of the files extension asks it. */
export function write(life: Life, path: string, content: string): void {
  life.call("ask", ["write", life.root, path, content], {});
}

/** A command that the bash extension runs, by the name of its act. */
export function bash(life: Life, command: string, kwargs: Record<string, unknown> = {}): string {
  return verb(life, "bash", [command], kwargs);
}

/** What a command came to, once it is done, as the part of the bash extension holds it: no value of the engine
 * crosses for it. */
export async function exited(world: World | undefined, life: Life, id: string): Promise<Streams> {
  await life.result(id);
  return world?.activity.acts.get(id)?.value as Streams;
}
