import type { TuiExtension } from "@furb/engine";
import bash from "./bash.ts";
import files from "./files.ts";
import grant from "./grant.ts";

/** The parts for the TUI of the builtin extensions, by the name of their extension. */
export const builtinTuiParts: Readonly<Record<string, TuiExtension>> = { files, bash, grant };
