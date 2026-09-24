import { realpath } from "node:fs/promises";
import { pathToFileURL } from "node:url";
import type { Engine } from "./bridge.ts";
import { commands } from "./commands.ts";

export interface ExtensionContext {
  life: Engine;
  chain: string;
  directory: string;
  notify(message: string): void;
  submit(message: string): Promise<void>;
}
export interface ExtensionCommand {
  label: string;
  description: string;
  run(argument: string, context: ExtensionContext): void | Promise<void>;
}
export interface ExtensionAPI {
  registerCommand(name: string, command: ExtensionCommand): void;
}

/** Extensions run only when named by the user, through --extension or /extension. */
export class Extensions {
  readonly commands = new Map<string, ExtensionCommand>();
  readonly paths = new Set<string>();
  private readonly cleanups: (() => void | Promise<void>)[] = [];
  constructor(private readonly context: () => ExtensionContext) {}
  /** Load the extension at a path, which the caller resolves as it resolves every path the user gives. */
  async load(path: string): Promise<void> {
    const file = await realpath(path);
    if (this.paths.has(file)) return;
    const module = await import(pathToFileURL(file).href);
    if (typeof module.default !== "function")
      throw new Error("An extension exports a default setup function.");
    const pending = new Map<string, ExtensionCommand>();
    const cleanup = await module.default({
      registerCommand: (name: string, command: ExtensionCommand) => {
        if (
          !/^[a-z][a-z0-9-]*$/.test(name) ||
          name in commands ||
          this.commands.has(name) ||
          pending.has(name)
        )
          throw new Error(`Command ${name} is unavailable.`);
        if (
          typeof command?.run !== "function" ||
          typeof command.label !== "string" ||
          typeof command.description !== "string"
        )
          throw new Error("An extension command needs a label, description, and run function.");
        pending.set(name, command);
      },
    } satisfies ExtensionAPI);
    for (const [name, command] of pending) this.commands.set(name, command);
    if (typeof cleanup === "function") this.cleanups.push(cleanup);
    this.paths.add(file);
  }
  async run(name: string, argument: string): Promise<boolean> {
    const command = this.commands.get(name);
    if (!command) return false;
    await command.run(argument, this.context());
    return true;
  }
  async dispose(): Promise<void> {
    for (const cleanup of this.cleanups.reverse()) await cleanup();
    this.commands.clear();
  }
}
