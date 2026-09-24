import { mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import type { Hearing, WorldContext, WorldPart } from "../extension.js";
import { type Fact, isQuestion } from "../types.js";

/** The most bytes the World reads of one file, since a text a model cannot hold is no answer. */
const CAP = 524288;

/** The part of the files extension: a read and a write of the disk, each resolved where its chain stands, answered
 * with the plain data of the path and the content, or with a refusal. A path of a scheme is another ear's to
 * answer, and the door of an act that no longer lives is refused. */
export default function files(context: WorldContext): WorldPart {
  /** Whether a path is a door: its first part names an act of the life, so `./` before it reaches the file. */
  function* door(path: string): Hearing<boolean> {
    try {
      yield { verb: "get", args: [path.split("/")[0]] };
      return true;
    } catch {
      return false;
    }
  }
  function read(path: string): { path: string; content: string } {
    let info: ReturnType<typeof statSync>;
    try {
      info = statSync(path);
    } catch (error) {
      // A path that names nothing is said in plain words, which the model and the operator both read.
      if ((error as NodeJS.ErrnoException).code === "ENOENT") throw new Error(`There is no file at ${path}.`);
      throw error;
    }
    if (!info.isFile() || info.size > CAP)
      throw new Error(`Read needs a text file at most ${CAP} bytes: ${path}`);
    return {
      path,
      content: new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(readFileSync(path)),
    };
  }
  function write(path: string, content: string): { path: string; content: string } {
    let before = "";
    try {
      before = readFileSync(path, "utf8");
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
    }
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, content);
    context.change({ path, before, after: content });
    return { path, content: readFileSync(path, "utf8") };
  }
  return {
    *hears(fact: Fact) {
      const [kind, qid, , on, path, content] = fact;
      if ((kind !== "read" && kind !== "write") || !isQuestion(kind, qid) || typeof path !== "string") return;
      if (path.includes("://")) return;
      let value: unknown;
      try {
        if (yield* door(path))
          throw new Error(
            `${path} is the door of nothing that ${kind === "read" ? "lives" : "takes a word"}`,
          );
        const at = context.at(yield* context.where(String(on)), path);
        value = kind === "read" ? read(at) : write(at, String(content));
      } catch (error) {
        value = context.refused(error instanceof Error ? error.message : String(error));
      }
      yield ["done", qid, value];
    },
  };
}
