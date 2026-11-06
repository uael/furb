import { closeSync, openSync, readSync } from "node:fs";

/** The word that names a session: the act rename_session, done with the name that the operator said, which the record
 * keeps as the word of a rung of the operator, whatever build, extension or engine plays it. */
export function renameWord(name: string): string {
  return `act("rename_session", "", lambda id: (yield "done", id, ${JSON.stringify(name)}))`;
}
