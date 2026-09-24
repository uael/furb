import type { ActView, LiveAct, SidebarPart, TuiCommand, TuiContext, TuiPart } from "@furb/engine";
import { commands } from "./commands.ts";

/** The parts of the extensions for the TUI, merged: the commands, the prefixes, the views of the acts of their kinds,
 * the quiet header words, the header words whose detail is a path, and what each adds to the sidebar and does before
 * a prompt. Two parts that claim one command, one prefix or one kind of act are refused, with both names. */
export class Parts {
  readonly commands = new Map<string, TuiCommand>();
  /** Each prefix of the input, and the command it says. */
  readonly prefixes = new Map<string, string>();
  readonly quiet = new Set<string>();
  readonly paths = new Set<string>();
  private readonly views = new Map<string, ActView>();
  constructor(readonly list: readonly { name: string; part: TuiPart }[] = []) {
    const owners = new Map<string, string>();
    const claim = (what: string, owner: string) => {
      const before = owners.get(what);
      if (before) throw new Error(`The extensions ${before} and ${owner} both give ${what}.`);
      owners.set(what, owner);
    };
    for (const { name, part } of list) {
      for (const [command, given] of Object.entries(part.commands ?? {})) {
        if (Object.hasOwn(commands, command))
          throw new Error(`The extension ${name} gives the command /${command}, which the TUI has.`);
        claim(`the command /${command}`, name);
        this.commands.set(command, given);
      }
      for (const [prefix, command] of Object.entries(part.prefixes ?? {})) {
        claim(`the prefix ${prefix}`, name);
        this.prefixes.set(prefix, command);
      }
      for (const [kind, view] of Object.entries(part.acts ?? {})) {
        claim(`the acts of the kind ${kind}`, name);
        this.views.set(kind, view);
      }
      for (const word of part.quiet ?? []) this.quiet.add(word);
      for (const word of part.paths ?? []) this.paths.add(word);
    }
  }
  /** How the acts of a kind show, and nothing for a kind that no part shows. */
  view(kind: string): ActView | undefined {
    return this.views.get(kind);
  }
  /** Whether an act is no card, no point to rewind to, and no state of its chain: a chain, or an act that its part
   * hides. */
  hidden(act: LiveAct): boolean {
    return act.kind === "chain" || Boolean(this.views.get(act.kind)?.hidden);
  }
  /** The header words that tell how an act of a kind ended, beside those of every act. */
  ends(kind: string): string[] {
    return ["closed", "cancelled", ...(this.views.get(kind)?.ends ?? [])];
  }
  /** What the parts add to the sidebar for the chain on screen. */
  sidebar(view: { acts: readonly LiveAct[]; chain: string }): SidebarPart[] {
    return this.list.flatMap(({ part }) => {
      const got = part.sidebar?.(view);
      return got ? [got] : [];
    });
  }
  /** What each part does before a message of the operator is sent, in order. */
  async prompting(message: string, context: TuiContext): Promise<void> {
    for (const { part } of this.list) await part.prompting?.(message, context);
  }
  /** The command that a prefix of a text says, with the rest of the text as its argument. */
  prefixed(text: string): { command: string; argument: string } | undefined {
    for (const [prefix, command] of this.prefixes)
      if (text.startsWith(prefix)) return { command, argument: text.slice(prefix.length).trimStart() };
    return undefined;
  }
  async dispose(): Promise<void> {
    for (const { part } of [...this.list].reverse()) await part.dispose?.();
  }
}
