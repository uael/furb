import { closeSync, openSync, readSync } from "node:fs";

/** The word that names a session: the act rename_session, done with the name that the operator said, which the record
 * keeps as the word of a rung of the operator, whatever build, extension or engine plays it. */
export function renameWord(name: string): string {
  return `act("rename_session", "", lambda id: (yield "done", id, ${JSON.stringify(name)}))`;
}

const SAID = /^act\("rename_session", "", lambda id: \(yield "done", id, ("(?:[^"\\]|\\.)*")\)\)$/s;

/** The name that a word of a rung says, when it is the word that names a session. */
function said(word: unknown): string | undefined {
  if (typeof word !== "string") return undefined;
  const found = SAID.exec(word);
  if (!found?.[1]) return undefined;
  try {
    const name: unknown = JSON.parse(found[1]);
    return typeof name === "string" ? name : undefined;
  } catch {
    return undefined;
  }
}

/** The last name that the operator gave a session, by a plain scan of its record: each entry is a fact of the record,
 * and each rung whose word names the session says a name. No replay runs, so no drift, no lease and no other build or
 * host hides it, and the read changes nothing. Nothing when the record names none or cannot be read; an empty name
 * says that the operator took the default name again. The scan reads the record a slice at a time and parses only the
 * entries that name rename_session, so a long record costs a read of its bytes. */
export function recordName(record: string): string | undefined {
  let fd: number;
  try {
    fd = openSync(record, "r");
  } catch {
    return undefined;
  }
  let name: string | undefined;
  const take = (line: string) => {
    if (!line.includes("rename_session")) return;
    try {
      const entry: unknown = JSON.parse(line);
      const fact = Array.isArray(entry) ? entry[0] : undefined;
      if (Array.isArray(fact) && fact[0] === "rung") name = said(fact[4]) ?? name;
    } catch {}
  };
  try {
    const slice = Buffer.alloc(1 << 20);
    let rest = Buffer.alloc(0);
    for (let read = readSync(fd, slice); read > 0; read = readSync(fd, slice)) {
      let held = Buffer.concat([rest, slice.subarray(0, read)]);
      for (let end = held.indexOf(10); end >= 0; end = held.indexOf(10)) {
        // Most entries hold no rename, and a test of the bytes keeps the scan from decoding them.
        const line = held.subarray(0, end);
        if (line.includes("rename_session")) take(line.toString("utf8"));
        held = held.subarray(end + 1);
      }
      rest = Buffer.from(held);
    }
    if (rest.length) take(rest.toString("utf8"));
  } catch {
  } finally {
    closeSync(fd);
  }
  return name;
}
